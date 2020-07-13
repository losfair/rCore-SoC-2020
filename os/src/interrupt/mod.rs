mod context;
mod handler;
mod timer;
use riscv::register::sstatus;

pub use context::Context;

pub fn init() {
    handler::init();
    timer::init();

    // Now we are all set up. Enable S-mode interrupts.
    unsafe {
        sstatus::set_sie();
    }

    println!("interrupt: Initialized.");
}
