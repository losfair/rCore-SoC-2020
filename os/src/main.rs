#![no_std]
#![no_main]
#![feature(
    global_asm,
    llvm_asm,
    panic_info_message,
    alloc_error_handler,
    new_uninit,
    map_first_last,
    raw
)]

#[macro_use]
extern crate alloc;

#[macro_use]
mod console;
mod allocator;
mod error;
mod init;
mod interrupt;
mod layout;
mod memory;
mod panic;
mod process;
mod sbi;
mod scheduler;
mod sync;
mod user;

// Entry point written in assembly.
global_asm!(include_str!("entry.asm"));

/// Use `dlmalloc` as the global allocator.
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Kernel loaded.");
    layout::print();
    allocator::init();
    memory::init();
    interrupt::init();
    scheduler::init();

    init::start();
}
