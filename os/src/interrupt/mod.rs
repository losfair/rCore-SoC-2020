mod context;
mod handler;
mod timer;
use riscv::register::sstatus;

pub use context::Context;
pub use handler::InterruptToken;

pub fn init() {
    handler::init();
    timer::init();

    // Don't enable supervisor-mode interrupts yet. Do this in the first process.

    println!("interrupt: Initialized.");
}
