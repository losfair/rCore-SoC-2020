use super::EntryReason;
use crate::process::RawThreadState;
use crate::process::Thread;
use crate::sbi::set_timer;
use alloc::boxed::Box;
use riscv::{asm::wfi, register::time};

const DEFAULT_SCHEDULER_REENTRY_TIMEOUT: usize = 100000;

pub struct HardwareThread {
    id: Id,
    current: Box<Thread>,
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
    pub fn new(id: Id, initial_thread: Box<Thread>) -> Box<Self> {
        let mut ht = Box::new(HardwareThread {
            id,
            current: initial_thread,
        });
        ht.populate_thread_state();

        ht
    }

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
        self.current.context_mut().gregs[3] = self_ptr as usize; // gp
    }

    pub unsafe fn enter_kernel(&mut self, ts: &mut RawThreadState, reason: EntryReason) -> ! {
        match self.current.context().was_user() {
            true => self.enter_from_user(ts, reason),
            false => self.enter_from_kernel(ts, reason),
        }
    }

    fn enter_from_user(&mut self, ts: &mut RawThreadState, reason: EntryReason) -> ! {
        match reason {
            EntryReason::Timer => self.return_to(ts),
            _ => panic!("enter_from_user: Unknown reason: {:?}", reason),
        }
    }

    fn enter_from_kernel(&mut self, ts: &mut RawThreadState, reason: EntryReason) -> ! {
        match reason {
            EntryReason::Timer => {
                static mut TICKS: usize = 0;
                unsafe {
                    TICKS += 1;
                    if TICKS % 100 == 0 {
                        println!("{} ticks", TICKS);
                    }
                }
                self.return_to(ts);
            }
            _ => panic!("enter_from_kernel: Unknown reason: {:?}", reason),
        }
    }

    fn return_to(&mut self, ts: &mut RawThreadState) -> ! {
        match self.current.context().was_user() {
            true => self.prepare_return_to_user(),
            false => self.prepare_return_to_kernel(),
        };
        prepare_scheduler_reentry();
        unsafe {
            ts.context.leave();
        }
    }

    pub fn return_to_current(&mut self) -> ! {
        match self.current.context().was_user() {
            true => self.prepare_return_to_user(),
            false => self.prepare_return_to_kernel(),
        };
        prepare_scheduler_reentry();
        unsafe {
            self.current.context().leave();
        }
    }
}

/// Sets up the timer for kernel re-entry.
fn prepare_scheduler_reentry() {
    set_timer(time::read() + DEFAULT_SCHEDULER_REENTRY_TIMEOUT);
}

fn idle() -> ! {
    prepare_scheduler_reentry();
    loop {
        unsafe {
            wfi();
        }
    }
}
