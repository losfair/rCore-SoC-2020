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
    unsafe {
        riscv::asm::ebreak();
    }
    test_alloc();
    println!("Entering idle state.");
    scheduler::idle();
    panic!("End of rust_main");
}

fn test_alloc() {
    return;
    use alloc::vec::Vec;
    for k in 0..100 {
        let mut v = vec![0u32; 10000];
        v[0] = 1;
        v[1] = 1;
        for i in 2..v.len() {
            v[i] = v[i - 1].wrapping_add(v[i - 2]);
        }
        println!("k={} v = {:?}", k, v[v.len() - 1]);
    }
}
