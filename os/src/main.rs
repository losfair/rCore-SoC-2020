#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm, panic_info_message)]

#[macro_use]
mod console;
mod interrupt;
mod panic;
mod sbi;
mod scheduler;
mod user;

// Entry point written in assembly.
global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Kernel loaded.");
    interrupt::init();
    scheduler::idle();
    panic!("End of rust_main");
}
