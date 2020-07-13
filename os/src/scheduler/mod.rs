use crate::interrupt::Context;
use crate::sbi::set_timer;
use riscv::register::time;

const DEFAULT_SCHEDULER_REENTRY_TIMEOUT: usize = 100000;

pub unsafe fn switch_to(context: &Context) -> ! {
    prepare_scheduler_reentry();
    context.leave();
}

pub fn idle() -> ! {
    prepare_scheduler_reentry();
    loop {}
}

/// Sets up the timer for kernel re-entry.
fn prepare_scheduler_reentry() {
    set_timer(time::read() + DEFAULT_SCHEDULER_REENTRY_TIMEOUT);
}
