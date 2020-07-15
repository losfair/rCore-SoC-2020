use super::LockedProcess;
use crate::error::*;
use crate::interrupt::{Context, InterruptToken};
use crate::scheduler::{EntryReason, HardwareThread};
use alloc::boxed::Box;
use core::mem;
use core::raw::TraitObject;
use core::sync::atomic::{AtomicU64, Ordering};
use riscv::register::sstatus::Sstatus;

pub struct Thread {
    id: Id,
    pub process: Option<LockedProcess>,
    kernel_stack: Box<KernelStack>,
}

/// A token that indicates execution in a kernel thread context (SIE = 1).
#[derive(Debug)]
pub struct ThreadToken(());

impl ThreadToken {
    pub unsafe fn assume_synchronous_exception(interrupt_token: &InterruptToken) -> &ThreadToken {
        static TOKEN: ThreadToken = ThreadToken(());
        &TOKEN
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

#[repr(C, align(16))]
struct KernelStack([u8; 65536]);

impl Thread {
    pub fn new(
        entry: fn(&mut HardwareThread, &ThreadToken, usize, usize) -> !,
        entry_ctx1: usize,
        entry_ctx2: usize,
    ) -> KernelResult<Box<Thread>> {
        unsafe extern "C" fn thread_entry_trampoline(
            ts: &mut RawThreadState,
            entry: fn(&mut HardwareThread, &ThreadToken, usize, usize) -> !,
            entry_ctx1: usize,
            entry_ctx2: usize,
        ) -> ! {
            let token = ThreadToken(());
            println!("thread entry. hart = {:p}", &mut *ts.hart);
            entry(&mut *ts.hart, &token, entry_ctx1, entry_ctx2)
        }
        let id = Id(NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let mut th = Thread {
            id,
            process: None,
            kernel_stack: KernelStack::new(),
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

    pub fn raw_thread_state(&self) -> &RawThreadState {
        Self::check_ts_size();
        unsafe {
            mem::transmute(
                &self.kernel_stack.0[self.kernel_stack.0.len() - mem::size_of::<RawThreadState>()],
            )
        }
    }

    pub fn raw_thread_state_mut(&mut self) -> &mut RawThreadState {
        Self::check_ts_size();
        unsafe {
            mem::transmute(
                &mut self.kernel_stack.0
                    [self.kernel_stack.0.len() - mem::size_of::<RawThreadState>()],
            )
        }
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
