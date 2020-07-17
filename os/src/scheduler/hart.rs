use super::EntryReason;
use super::{GlobalPlan, PolicyContext, SwitchReason};
use crate::interrupt::{Context, InterruptToken};
use crate::process::{create_kernel_thread, KernelTask, RawThreadState, Thread, ThreadToken};
use crate::sbi::set_timer;
use crate::sync::{IntrCell, IntrGuardMut};
use alloc::boxed::Box;
use alloc::collections::linked_list::LinkedList;
use alloc::sync::Arc;
use bit_field::BitField;
use core::cell::Cell;
use core::mem;
use core::sync::atomic::{AtomicUsize, Ordering};
use riscv::register::{
    sie::{clear_stimer, set_stimer},
    sstatus::{self, clear_sie, set_sie},
};
use riscv::{asm::wfi, register::time};

const DEFAULT_SCHEDULER_REENTRY_TIMEOUT: usize = 100000;

extern "C" {
    fn save_gregs_assuming_intr_disabled(context: &mut Context) -> usize;
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

    /// # of `IntrGuard`s currently active on this hardware thread.
    num_intr_guards: Cell<usize>,

    /// Value of the SIE bit before `IntrGuard`s.
    sie_before_intr_guard: Cell<bool>,

    /// The current thread.
    ///
    /// NOT safe to drop since it contains the stack of the running code itself.
    current: IntrCell<Box<Thread>>,

    /// A list of threads that are waiting to be dropped.
    ///
    /// Avoid using continuous storage due to how our allocator works.
    will_drop: IntrCell<LinkedList<Box<Thread>>>,

    /// The scheduler state.
    scheduler_state: IntrCell<SchedulerState>,
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
            current: IntrCell::new(initial_thread),
            num_intr_guards: Cell::new(0),
            sie_before_intr_guard: Cell::new(true),
            will_drop: IntrCell::new(LinkedList::new()),
            scheduler_state: IntrCell::new(SchedulerState::NotRunning {
                scheduler_thread: Self::scheduler_thread(),
            }),
        });
        ht.populate_thread_state();

        ht
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn has_active_intr_guards(&self) -> bool {
        self.num_intr_guards.get() != 0
    }

    pub unsafe fn acquire_intr_guard(&self) {
        // On the same thread. So `Relaxed` works.
        let prev_sie = sstatus::read().sie();
        clear_sie();
        let prev_n = self.num_intr_guards.get();
        self.num_intr_guards.set(prev_n + 1);
        if prev_n == 0 {
            self.sie_before_intr_guard.set(prev_sie);
        }
    }

    pub unsafe fn release_intr_guard(&self) {
        let prev = self.num_intr_guards.get();
        if prev == 0 {
            panic!("release_intr_guard: prev == 0");
        }
        self.num_intr_guards.set(prev - 1);
        if prev == 1 {
            if self.sie_before_intr_guard.get() {
                set_sie();
            }
        }
    }

    fn scheduler_thread() -> Box<Thread> {
        struct SchedulerThread;
        impl KernelTask for SchedulerThread {
            fn run(self: Box<Self>, ht: &HardwareThread, token: &ThreadToken) {
                drop(self);
                ht.scheduler_loop(token)
            }
        }
        create_kernel_thread(Box::new(SchedulerThread)).unwrap()
    }

    fn scheduler_loop(&self, token: &ThreadToken) -> ! {
        loop {
            // Drop all threads in `will_drop`.
            for th in mem::replace(&mut *self.will_drop.borrow_mut(self), LinkedList::new()) {
                unsafe {
                    th.drop_assuming_not_current();
                }
            }
            // Choose next thread to run.
            let previous_thread = match mem::replace(
                &mut *self.scheduler_state.borrow_mut(self),
                SchedulerState::WillLeave,
            ) {
                SchedulerState::Running { previous_thread } => previous_thread,
                _ => panic!("scheduler_loop: bad previous state"),
            };
            match self
                .plan
                .next(self, PolicyContext::Critical, SwitchReason::Periodic)
            {
                Some(next) => {
                    // Re-entrance is possible
                    self.plan
                        .add_thread(self, PolicyContext::Critical, previous_thread);
                    unsafe {
                        prepare_scheduler_reentry();
                        self.ll_yield(
                            next,
                            |old| {
                                *self.scheduler_state.borrow_mut(self) =
                                    SchedulerState::NotRunning {
                                        scheduler_thread: old,
                                    };
                            },
                            token,
                        );
                    }
                }
                None => unsafe {
                    prepare_scheduler_reentry();
                    self.ll_yield(
                        previous_thread,
                        |old| {
                            *self.scheduler_state.borrow_mut(self) = SchedulerState::NotRunning {
                                scheduler_thread: old,
                            };
                        },
                        token,
                    );
                },
            }
        }
    }

    /// Populate the state of a newly-pinned thread.
    ///
    /// Should be called each time after `self.current` is changed.
    fn populate_thread_state(&self) {
        let self_ptr = self as *const _ as *mut HardwareThread;
        self.current.borrow_mut(self).raw_thread_state_mut().hart = self_ptr;
    }

    fn prepare_return_to_user(&self) {
        unsafe {
            llvm_asm!("csrw sscratch, $0" :: "r" (self.current.borrow_mut(self).raw_thread_state_mut()) :: "volatile");
        }
    }

    fn prepare_return_to_kernel(&self) {
        let self_ptr = self as *const _ as *mut HardwareThread;
        // gp
        self.current
            .borrow_mut(self)
            .raw_thread_state_mut()
            .kcontext
            .gregs[3] = self_ptr as usize;
    }

    pub unsafe fn enter_kernel(&mut self, token: &InterruptToken, reason: EntryReason) -> ! {
        let was_user = self
            .current
            .borrow_mut(self)
            .raw_thread_state_mut()
            .was_user();
        match was_user {
            true => self.enter_from_user(token, reason),
            false => self.enter_from_kernel(token, reason),
        }
    }

    fn enter_from_user(&self, token: &InterruptToken, reason: EntryReason) -> ! {
        match reason {
            EntryReason::Timer => self.return_to_current(token),
            _ => panic!("enter_from_user: Unknown reason: {:?}", reason),
        }
    }

    fn enter_from_kernel(&self, token: &InterruptToken, reason: EntryReason) -> ! {
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
                println!("Breakpoint at {:p}", addr as *mut ());
                self.return_to_current(token);
            }
            _ => panic!("enter_from_kernel: Unknown reason: {:?}", reason),
        }
    }

    pub fn return_to_current(&self, _: &InterruptToken) -> ! {
        unsafe { self.force_return_to_current() }
    }

    pub fn with_current<F: FnOnce(&mut Thread) -> R, R>(&self, f: F) -> R {
        let mut current = self.current.borrow_mut(self);
        f(&mut **current)
    }

    pub unsafe fn start(&self) -> ! {
        prepare_scheduler_reentry();
        self.force_return_to_current();
    }

    unsafe fn force_return_to_current(&self) -> ! {
        // We are in interrupt context. SIE is already disabled, so this is safe.
        let ts = self.current.borrow_mut(self).raw_thread_state_mut() as *mut RawThreadState;
        let ts = &mut *ts;

        match ts.was_user() {
            true => self.prepare_return_to_user(),
            false => self.prepare_return_to_kernel(),
        };
        ts.leave();
    }

    /// Does not allocate.
    #[inline(never)]
    unsafe fn ll_yield<F: FnOnce(Box<Thread>)>(
        &self,
        next: Box<Thread>,
        consume_old: F,
        _: &ThreadToken,
    ) {
        let ts = self.current.borrow_mut(self).raw_thread_state_mut() as *mut RawThreadState;
        let ts = &mut *ts;

        self.acquire_intr_guard(); // Mask interrupts

        if save_gregs_assuming_intr_disabled(&mut ts.kcontext) == 0 {
            // restore path. SIE = 1

            // already consumed by the other branch
            mem::forget(next);
            mem::forget(consume_old);
        } else {
            // save path - never returns. SIE = 0

            // switch thread
            let prev = self.replace_current(next);

            // drops both `consume_old` and `prev`
            // This is dangerous, because `consume_old` must not be interrupted.
            consume_old(prev);

            // read previous sstatus. SIE = 0
            let mut prev_sstatus: usize = mem::transmute(sstatus::read());

            // sanity check
            assert!(
                self.sie_before_intr_guard.get() == true,
                "ll_yield: sie_before_intr_guard.get() != true"
            );

            // "drop" guard without actually unmasking interrupts
            assert!(self.num_intr_guards.get() == 1, "ll_yield: interrupt guards must not be held on the current hart after `consume_old`");
            self.num_intr_guards.set(0);

            // fixup sstatus "as if" it is generated on an interrupt
            {
                // step 1. set spie
                prev_sstatus.set_bit(5, true);

                // step 2. set spp to supervisor
                prev_sstatus.set_bit(8, true);
            }

            // assign the fixed-up sstatus to `kcontext`
            ts.kcontext.sstatus = prev_sstatus;

            // kcontext valid
            ts.kcontext_valid = 1;

            // return
            self.force_return_to_current();
        }
    }

    pub fn exit_thread(&self, token: &ThreadToken) -> ! {
        self.yield_or_exit(token, true);
        unreachable!()
    }

    pub fn do_yield(&self, token: &ThreadToken) {
        self.yield_or_exit(token, false);
    }

    /// Releases the current thread and switches to a new thread.
    ///
    /// Requires:
    ///
    /// - Thread context
    /// - No
    ///
    /// # Safety
    ///
    /// The callback, `f`, is called after `self.current` is no longer the current thread.
    /// This might not be safe, depending on what the callback does.
    pub unsafe fn release_current<F: FnOnce(Box<Thread>)>(&self, f: F, token: &ThreadToken) {
        assert!(
            self.has_active_intr_guards() == false,
            "release_current: must not hold any interrupt guards"
        );
        loop {
            match self
                .plan
                .next(self, PolicyContext::NonCritical(token), SwitchReason::Yield)
            {
                Some(next) => {
                    self.ll_yield(next, move |th| f(th), token);
                    break;
                }
                None => {
                    wfi();
                }
            }
        }
    }

    fn replace_current(&self, new_current: Box<Thread>) -> Box<Thread> {
        let ret = mem::replace(&mut *self.current.borrow_mut(self), new_current);
        self.populate_thread_state();
        ret
    }

    fn yield_or_exit(&self, token: &ThreadToken, exit: bool) {
        loop {
            match self
                .plan
                .next(self, PolicyContext::NonCritical(token), SwitchReason::Yield)
            {
                Some(next) => {
                    unsafe {
                        self.ll_yield(
                            next,
                            move |old| {
                                if exit {
                                    self.will_drop.borrow_mut(self).push_back(old);
                                } else {
                                    // Although interrupts are disabled, this is not critical
                                    // because there is no possibility of re-entrance.
                                    self.plan.add_thread(
                                        self,
                                        PolicyContext::NonCritical(token),
                                        old,
                                    );
                                }
                            },
                            token,
                        );
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

    fn tick(&self, token: &InterruptToken) -> ! {
        // Mask off timer interrupt, until the scheduler thread re-enables it.
        unsafe {
            clear_stimer();
        }

        // Temporarily take out scheduler_state
        let prev_state = mem::replace(
            &mut *self.scheduler_state.borrow_mut(self),
            SchedulerState::WillLeave,
        );
        match prev_state {
            SchedulerState::NotRunning { scheduler_thread } => {
                // Switch to scheduler.
                let previous_thread = self.replace_current(scheduler_thread);
                *self.scheduler_state.borrow_mut(self) =
                    SchedulerState::Running { previous_thread };
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
