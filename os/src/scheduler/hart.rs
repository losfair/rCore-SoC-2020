use super::EntryReason;
use super::{GlobalPlan, SwitchReason};
use crate::interrupt::InterruptToken;
use crate::process::{create_kernel_thread, KernelTask, Thread, ThreadToken};
use crate::sbi::set_timer;
use alloc::boxed::Box;
use alloc::collections::linked_list::LinkedList;
use alloc::sync::Arc;
use core::mem;
use riscv::register::sie::{clear_stimer, set_stimer};
use riscv::{asm::wfi, register::time};

const DEFAULT_SCHEDULER_REENTRY_TIMEOUT: usize = 100000;

global_asm!(".globl raw_yield\nraw_yield:\nebreak\nret");
extern "C" {
    /// Special yield point.
    #[allow(improper_ctypes)]
    fn raw_yield(next: Box<Thread>, exit: usize);
}

/// Scheduler state.
enum SchedulerState {
    WillLeave,
    NotRunning { scheduler_thread: Box<Thread> },
    Running { previous_thread: Box<Thread> },
}

pub struct HardwareThread {
    id: Id,
    plan: Arc<GlobalPlan>,

    /// The current thread.
    ///
    /// NOT safe to drop since it contains the stack of the running code itself.
    current: Box<Thread>,

    /// A list of threads that are waiting to be dropped.
    /// 
    /// Avoid using continuous storage due to how our allocator works.
    will_drop: LinkedList<Box<Thread>>,

    /// The scheduler state.
    scheduler_state: SchedulerState,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Id(pub u32);

impl Drop for HardwareThread {
    fn drop(&mut self) {
        panic!("HardwareThread must not be dropped");
    }
}

impl HardwareThread {
    pub fn new(id: Id, plan: Arc<GlobalPlan>, initial_thread: Box<Thread>) -> Box<Self> {
        let mut ht = Box::new(HardwareThread {
            id,
            plan,
            current: initial_thread,
            will_drop: LinkedList::new(),
            scheduler_state: SchedulerState::NotRunning {
                scheduler_thread: Self::scheduler_thread(),
            },
        });
        ht.populate_thread_state();

        ht
    }

    fn scheduler_thread() -> Box<Thread> {
        struct SchedulerThread;
        impl KernelTask for SchedulerThread {
            fn run(self: Box<Self>, ht: &mut HardwareThread, token: &ThreadToken) {
                drop(self);
                ht.scheduler_loop(token)
            }
        }
        create_kernel_thread(Box::new(SchedulerThread)).unwrap()
    }

    fn scheduler_loop(&mut self, token: &ThreadToken) -> ! {
        loop {
            // Drop all threads in `will_drop`.
            for th in mem::replace(&mut self.will_drop, LinkedList::new()) {
                unsafe {
                    th.drop_assuming_not_current();
                }
            }
            // Choose next thread to run.
            let previous_thread =
                match mem::replace(&mut self.scheduler_state, SchedulerState::WillLeave) {
                    SchedulerState::Running { previous_thread } => previous_thread,
                    _ => panic!("scheduler_loop: bad previous state"),
                };
            match self.plan.next(self.id, SwitchReason::Periodic, token) {
                Some(next) => {
                    self.plan.add_thread(previous_thread, token);
                    unsafe {
                        prepare_scheduler_reentry();
                        raw_yield(next, 0);
                    }
                }
                None => unsafe {
                    prepare_scheduler_reentry();
                    raw_yield(previous_thread, 0);
                },
            }
        }
    }

    /// Populate the state of a newly-pinned thread.
    ///
    /// Should be called each time after `self.current` is changed.
    fn populate_thread_state(&mut self) {
        let self_ptr = self as *mut HardwareThread;
        self.current.raw_thread_state_mut().hart = self_ptr;
    }

    fn prepare_return_to_user(&mut self) {
        unsafe {
            llvm_asm!("csrw sscratch, $0" :: "r" (self.current.raw_thread_state_mut()) :: "volatile");
        }
    }

    fn prepare_return_to_kernel(&mut self) {
        let self_ptr = self as *mut HardwareThread;
        self.current.raw_thread_state_mut().kcontext.gregs[3] = self_ptr as usize;
        // gp
    }

    pub unsafe fn enter_kernel(&mut self, token: &InterruptToken, reason: EntryReason) -> ! {
        match self.current.raw_thread_state_mut().was_user() {
            true => self.enter_from_user(token, reason),
            false => self.enter_from_kernel(token, reason),
        }
    }

    fn enter_from_user(&mut self, token: &InterruptToken, reason: EntryReason) -> ! {
        match reason {
            EntryReason::Timer => self.return_to_current(token),
            _ => panic!("enter_from_user: Unknown reason: {:?}", reason),
        }
    }

    fn enter_from_kernel(&mut self, token: &InterruptToken, reason: EntryReason) -> ! {
        match reason {
            EntryReason::Timer => {
                static mut TICKS: usize = 0;
                unsafe {
                    TICKS += 1;
                    if TICKS % 100 == 0 {
                        println!("{} ticks", TICKS);
                    }
                }
                self.tick(token);
            }
            EntryReason::Breakpoint(addr, thread_token) => {
                if addr == raw_yield as usize {
                    //println!("Raw yield breakpoint. current = {:p}", &*self.current);
                    let kcontext = &self.current.raw_thread_state().kcontext;
                    let next: Box<Thread> = unsafe { mem::transmute(kcontext.gregs[10]) }; // a0
                    let exit = kcontext.gregs[11] != 0; // a1
                    self.finalize_raw_yield(next, exit, thread_token);
                    self.return_to_current(token);
                }

                println!("Breakpoint at {:p}", addr as *mut ());
                self.return_to_current(token);
            }
            _ => panic!("enter_from_kernel: Unknown reason: {:?}", reason),
        }
    }

    pub fn return_to_current(&mut self, _: &InterruptToken) -> ! {
        unsafe { self.force_return_to_current() }
    }

    pub unsafe fn start(&mut self) -> ! {
        prepare_scheduler_reentry();
        self.force_return_to_current();
    }

    unsafe fn force_return_to_current(&mut self) -> ! {
        match self.current.raw_thread_state_mut().was_user() {
            true => self.prepare_return_to_user(),
            false => self.prepare_return_to_kernel(),
        };
        unsafe {
            self.current.raw_thread_state_mut().leave();
        }
    }

    pub fn current(&self) -> &Thread {
        &*self.current
    }

    pub fn current_mut(&mut self) -> &mut Thread {
        &mut *self.current
    }

    pub fn exit_thread(&mut self, token: &ThreadToken) -> ! {
        self.yield_or_exit(token, true);
        unreachable!()
    }

    pub fn do_yield(&mut self, token: &ThreadToken) {
        self.yield_or_exit(token, false);
    }

    fn replace_current(&mut self, new_current: Box<Thread>) -> Box<Thread> {
        let ret = mem::replace(&mut self.current, new_current);
        self.populate_thread_state();
        ret
    }

    fn finalize_raw_yield(&mut self, next: Box<Thread>, exit: bool, token: &ThreadToken) {
        let old = self.replace_current(next);
        self.populate_thread_state();
        match (&self.scheduler_state, exit) {
            (SchedulerState::WillLeave, false) => {
                // We are leaving from the scheduler.
                self.scheduler_state = SchedulerState::NotRunning {
                    scheduler_thread: old,
                };
            }
            (SchedulerState::WillLeave, true) => {
                panic!(
                    "finalize_raw_yield: `exit` must be false when scheduler state is `WillLeave`"
                );
            }
            (SchedulerState::Running { .. }, _) => {
                panic!("finalize_raw_yield: `scheduler_state` must not be `Running`");
            }
            (SchedulerState::NotRunning { .. }, true) => {
                // We must not drop `old` here since we are using its stack space.
                self.will_drop.push_back(old);
            }
            (SchedulerState::NotRunning { .. }, false) => {
                self.plan.add_thread(old, token);
            }
        }
    }

    fn yield_or_exit(&mut self, token: &ThreadToken, exit: bool) {
        loop {
            match self.plan.next(self.id, SwitchReason::Yield, token) {
                Some(next) => {
                    unsafe {
                        raw_yield(next, if exit { 1 } else { 0 });
                    }
                    break;
                }
                None => {
                    if exit {
                        // If exit is requested, retry until we get a thread.
                        unsafe {
                            wfi();
                        }
                    } else {
                        // Otherwise, immediately return to the current thread.
                        break;
                    }
                }
            }
        }
    }

    fn tick(&mut self, token: &InterruptToken) -> ! {
        // Mask off timer interrupt, until the scheduler thread re-enables it.
        unsafe {
            clear_stimer();
        }

        // Temporarily take out scheduler_state
        match mem::replace(&mut self.scheduler_state, SchedulerState::WillLeave) {
            SchedulerState::NotRunning { scheduler_thread } => {
                // Switch to scheduler.
                let previous_thread = self.replace_current(scheduler_thread);
                self.scheduler_state = SchedulerState::Running { previous_thread };
                self.return_to_current(token);
            }
            _ => panic!("tick: bad scheduler state"),
        }
    }
}

/// Sets up the timer for kernel re-entry.
fn prepare_scheduler_reentry() {
    set_timer(time::read() + DEFAULT_SCHEDULER_REENTRY_TIMEOUT);
    unsafe {
        set_stimer();
    }
}
