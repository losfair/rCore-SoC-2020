use super::EntryReason;
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

impl HardwareThread {
    pub fn enter(&mut self, reason: EntryReason) -> ! {
        match self.current.context().was_user() {
            true => self.enter_from_user(reason),
            false => self.enter_from_kernel(reason),
        }
    }

    fn enter_from_user(&mut self, reason: EntryReason) -> ! {
        match reason {
            EntryReason::Timer => self.next_thread(),
            _ => panic!("enter_from_user: Unknown reason: {:?}", reason),
        }
    }

    fn enter_from_kernel(&mut self, reason: EntryReason) -> ! {
        match reason {
            EntryReason::Timer => self.next_thread(),
            _ => panic!("enter_from_kernel: Unknown reason: {:?}", reason),
        }
    }

    fn return_to_current(&mut self) -> ! {
        prepare_scheduler_reentry();
        unsafe {
            self.current.context().leave();
        }
    }

    pub fn next_thread(&mut self) -> ! {
        self.return_to_current()
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
