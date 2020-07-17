use super::LockedProcess;
use crate::error::*;
use crate::interrupt::{Context, InterruptToken};
use crate::scheduler::{EntryReason, HardwareThread};
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::mem;
use core::sync::atomic::{AtomicU64, Ordering};

pub struct Thread {
    id: Id,
    pub process: Option<LockedProcess>,
    kernel_stack: Box<KernelStack>,
    auto_drop_allowed: bool,
}

/// A token that indicates execution in a kernel thread context (SIE = 1).
#[derive(Debug)]
pub struct ThreadToken(());

impl ThreadToken {
    pub unsafe fn new() -> &'static ThreadToken {
        static TOKEN: ThreadToken = ThreadToken(());
        &TOKEN
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        if !self.auto_drop_allowed {
            panic!("Attempting to automatically drop a thread. Use `drop_assuming_not_current` instead.");
        }
    }
}

#[repr(C)]
pub struct RawThreadState {
    /// Usermode context.
    pub ucontext: Context,

    /// Kernel-mode context.
    pub kcontext: Context,

    /// The hardware thread that this (software) thread is running on.
    pub hart: *mut HardwareThread,

    // Whether `kcontext` contains valid kernel context.
    pub kcontext_valid: usize,
}

impl RawThreadState {
    pub fn was_user(&self) -> bool {
        self.kcontext_valid == 0
    }

    pub fn last_context(&self) -> &Context {
        if self.was_user() {
            &self.ucontext
        } else {
            &self.kcontext
        }
    }

    pub fn last_context_mut(&mut self) -> &mut Context {
        if self.was_user() {
            &mut self.ucontext
        } else {
            &mut self.kcontext
        }
    }

    pub unsafe fn leave(&mut self) -> ! {
        if self.was_user() {
            self.ucontext.leave();
        } else {
            self.kcontext_valid = 0;
            self.kcontext.leave();
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Id(pub u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// The kernel stack.
///
/// Wrapped with `UnsafeCell` because the current stack pointer can be an implicit reference
/// to the kernel stack.
#[repr(C, align(16))]
struct KernelStack(UnsafeCell<[u8; 65536]>);

impl Thread {
    pub fn new(
        entry: fn(&HardwareThread, &ThreadToken, usize, usize) -> !,
        entry_ctx1: usize,
        entry_ctx2: usize,
    ) -> KernelResult<Box<Thread>> {
        unsafe extern "C" fn thread_entry_trampoline(
            ts: &mut RawThreadState,
            entry: fn(&HardwareThread, &ThreadToken, usize, usize) -> !,
            entry_ctx1: usize,
            entry_ctx2: usize,
        ) -> ! {
            let token = ThreadToken(());
            entry(&mut *ts.hart, &token, entry_ctx1, entry_ctx2)
        }
        let id = Id(NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let mut th = Thread {
            id,
            process: None,
            kernel_stack: KernelStack::new(),
            auto_drop_allowed: false,
        };
        let ts_ptr = th.raw_thread_state_mut() as *mut _;
        th.raw_thread_state_mut().kcontext_valid = 1;
        th.raw_thread_state_mut().kcontext.sepc = thread_entry_trampoline as usize;
        th.raw_thread_state_mut().kcontext.sstatus = 0x120;
        th.raw_thread_state_mut().kcontext.gregs[2] = ts_ptr as usize; // sp
        th.raw_thread_state_mut().kcontext.gregs[10] = ts_ptr as usize; // a0
        th.raw_thread_state_mut().kcontext.gregs[11] = entry as usize; // a1
        th.raw_thread_state_mut().kcontext.gregs[12] = entry_ctx1 as usize; // a2
        th.raw_thread_state_mut().kcontext.gregs[13] = entry_ctx2 as usize; // a3
        Ok(Box::new(th))
    }

    fn check_ts_size() {
        assert!(mem::size_of::<RawThreadState>() % 16 == 0);
        assert!(mem::size_of::<RawThreadState>() == (34 * 2 + 2) * 8);
    }

    /// Drops a thread, assuming we are not currently running on its stack.
    pub unsafe fn drop_assuming_not_current(mut self: Box<Self>) {
        self.auto_drop_allowed = true;
    }

    pub fn raw_thread_state_mut_ptr(&self) -> *mut RawThreadState {
        Self::check_ts_size();
        unsafe {
            let kernel_stack = &mut *self.kernel_stack.0.get();
            mem::transmute(&mut kernel_stack[kernel_stack.len() - mem::size_of::<RawThreadState>()])
        }
    }

    pub fn raw_thread_state(&self) -> &RawThreadState {
        unsafe { &*self.raw_thread_state_mut_ptr() }
    }

    pub fn raw_thread_state_mut(&mut self) -> &mut RawThreadState {
        unsafe { &mut *self.raw_thread_state_mut_ptr() }
    }
}

impl KernelStack {
    fn new() -> Box<KernelStack> {
        unsafe { Box::new_zeroed().assume_init() }
    }
}

impl RawThreadState {
    pub unsafe fn enter_kernel(&mut self, token: &InterruptToken, reason: EntryReason) -> ! {
        (*self.hart).enter_kernel(token, reason)
    }
}
