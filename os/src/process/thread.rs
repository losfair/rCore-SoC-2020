use super::LockedProcess;
use crate::error::*;
use crate::interrupt::Context;
use crate::scheduler::{EntryReason, HardwareThread};
use alloc::boxed::Box;
use core::mem;
use core::sync::atomic::{AtomicU64, Ordering};
use riscv::register::sstatus::Sstatus;

pub struct Thread {
    id: Id,
    process: LockedProcess,
    kernel_stack: Box<KernelStack>,
}

#[repr(C)]
pub struct RawThreadState {
    pub context: Context,
    pub hart: *mut HardwareThread,
    _padding: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Id(pub u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[repr(C, align(16))]
struct KernelStack([u8; 65536]);

impl Thread {
    pub fn new(
        process: LockedProcess,
        entry: extern "C" fn(usize) -> !,
        entry_ctx: usize,
    ) -> KernelResult<Thread> {
        let id = Id(NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let mut th = Thread {
            id,
            process,
            kernel_stack: KernelStack::new(),
        };
        let ts_ptr = th.raw_thread_state_mut() as *mut _;
        th.context_mut().sepc = entry as usize;
        th.context_mut().sstatus = 0x120;
        th.context_mut().gregs[2] = ts_ptr as usize; // sp
        th.context_mut().gregs[10] = entry_ctx; // a0
        Ok(th)
    }

    pub fn raw_thread_state(&self) -> &RawThreadState {
        assert!(mem::size_of::<RawThreadState>() % 16 == 0);
        unsafe {
            mem::transmute(
                &self.kernel_stack.0[self.kernel_stack.0.len() - mem::size_of::<RawThreadState>()],
            )
        }
    }

    pub fn raw_thread_state_mut(&mut self) -> &mut RawThreadState {
        assert!(mem::size_of::<RawThreadState>() % 16 == 0);
        unsafe {
            mem::transmute(
                &mut self.kernel_stack.0
                    [self.kernel_stack.0.len() - mem::size_of::<RawThreadState>()],
            )
        }
    }

    pub fn context(&mut self) -> &Context {
        &self.raw_thread_state().context
    }

    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.raw_thread_state_mut().context
    }
}

impl KernelStack {
    fn new() -> Box<KernelStack> {
        unsafe { Box::new_zeroed().assume_init() }
    }
}

impl RawThreadState {
    pub unsafe fn enter_kernel(&mut self, reason: EntryReason) -> ! {
        (*self.hart).enter_kernel(self, reason)
    }
}
