mod context;
mod handler;

pub use context::Context;
pub use handler::InterruptToken;

pub fn init() {
    handler::init();

    // Don't enable supervisor-mode interrupts yet. Do this in the first process.

    println!("interrupt: Initialized.");
}

pub unsafe fn ap_init() {
    handler::ap_init();
}
