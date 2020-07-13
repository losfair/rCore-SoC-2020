use crate::sbi::set_timer;
use riscv::register::{sie, time};

pub fn init() {
    unsafe {
        sie::set_stimer(); // Enable STIE bit for timer interrupts.
    }
    println!("interrupt/timer: Initialized.");
}
