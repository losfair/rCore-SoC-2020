#![no_std]
#![no_main]
#![feature(
    global_asm,
    llvm_asm,
    panic_info_message,
    alloc_error_handler,
    new_uninit,
    map_first_last
)]

#[macro_use]
extern crate alloc;

#[macro_use]
mod console;
mod allocator;
mod error;
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
    interrupt::init();
    memory::init();
    create_hart();
    println!("Entering idle state.");
    scheduler::idle();
    panic!("End of rust_main");
}

fn create_hart() {
    use alloc::boxed::Box;
    use process::{LockedProcess, Thread};
    use scheduler::{HardwareThread, HardwareThreadId};
    let p = LockedProcess::new(memory::boot_page_pool().clone()).unwrap();
    let th = Thread::new(p.clone(), th_entry, 42).unwrap();
    let mut hart = HardwareThread::new(HardwareThreadId(0), Box::new(th));
    hart.return_to_current();
}

extern "C" fn th_entry(data: usize) -> ! {
    println!("Thread entry! {}", data);
    loop {
        unsafe {
            llvm_asm!("wfi" :::: "volatile");
        }
    }
}
