use super::EntryReason;
use super::{GlobalPlan, SwitchReason};
use crate::interrupt::InterruptToken;
use crate::memory::boot_page_pool;
use crate::process::RawThreadState;
use crate::process::{LockedProcess, Thread, ThreadToken};
use crate::sbi::set_timer;
use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use alloc::sync::Arc;
use core::mem;
use riscv::register::sstatus::{clear_sie, set_sie};
use riscv::{asm::wfi, register::time};

const DEFAULT_SCHEDULER_REENTRY_TIMEOUT: usize = 100000;

global_asm!(".globl raw_yield\nraw_yield:\nebreak\nret");
extern "C" {
    /// Special yield point.
    fn raw_yield(next: Box<Thread>, exit: usize);
}

pub struct HardwareThread {
    id: Id,
    plan: Arc<GlobalPlan>,

    /// The current thread.
    ///
    /// NOT safe to drop since it contains the stack of the running code itself.
    current: Box<Thread>,

    /// A list of threads that are waiting to be dropped.
    will_drop: VecDeque<Box<Thread>>,
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
            will_drop: VecDeque::new(),
        });
        ht.populate_thread_state();

        ht
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
                    self.finalize_yield_or_exit(next, exit, thread_token);
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

    pub unsafe fn force_return_to_current(&mut self) -> ! {
        match self.current.raw_thread_state_mut().was_user() {
            true => self.prepare_return_to_user(),
            false => self.prepare_return_to_kernel(),
        };
        prepare_scheduler_reentry();
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

    fn finalize_yield_or_exit(&mut self, next: Box<Thread>, exit: bool, token: &ThreadToken) {
        let mut old = mem::replace(&mut self.current, next);
        if exit {
            // Dangerous zone: We MUST NOT drop `old` here since we are using its stack space.
            self.will_drop.push_back(old);
        } else {
            // Synchronous exception where we have control to its source. So `add_thread` would work fine.
            self.plan.add_thread(old, token);
        }
        self.populate_thread_state();
    }

    fn yield_or_exit(&mut self, token: &ThreadToken, exit: bool) {
        loop {
            // `plan` methods are not reentrant.
            // So here we need to mask the SIE bit to prevent `tick` from re-entering.
            //
            // Since we are in a thread context, `SIE` is always set before.
            unsafe {
                clear_sie();
            }
            match self.plan.next(self.id, SwitchReason::Yield(token)) {
                Some(next) => {
                    unsafe {
                        set_sie();
                        raw_yield(next, if exit { 1 } else { 0 });
                    }
                    break;
                }
                None => {
                    unsafe {
                        set_sie();
                    }
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
        match self.plan.next(self.id, SwitchReason::PeriodicInterrupt) {
            Some(next) => {
                println!("preempted: {:p} -> {:p}", &*self.current, &*next);
                let old = mem::replace(&mut self.current, next);
                self.plan.return_thread_interrupt(old);
                self.populate_thread_state();
                self.return_to_current(token);
            }
            None => {
                self.return_to_current(token);
            }
        }
    }

    pub fn spawn(&self, token: &ThreadToken, th: Box<Thread>) {}
}

/// Sets up the timer for kernel re-entry.
fn prepare_scheduler_reentry() {
    set_timer(time::read() + DEFAULT_SCHEDULER_REENTRY_TIMEOUT);
}
