#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm, panic_info_message)]

#[macro_use]
mod console;
mod sbi;
mod panic;

// Entry point written in assembly.
global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Hello RISC-V!");
    panic!("End of rust_main");
}