#![no_std]
#![no_main]
#![feature(
    global_asm,
    llvm_asm,
    panic_info_message,
    alloc_error_handler,
    new_uninit,
    map_first_last,
    raw,
    const_btree_new
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
mod smp;
mod sync;
mod tests;
mod user;

use memory::PhysicalAddress;

// Entry point written in assembly.
global_asm!(include_str!("entry.asm"));

/// Use `dlmalloc` as the global allocator.
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

#[no_mangle]
pub unsafe extern "C" fn rust_main(hart_id: u32, dtb_pa: PhysicalAddress) -> ! {
    let x: u32 = 42;
    if hart_id == 0 {
        kernel_boot(dtb_pa);
    } else {
        smp::ap_boot(hart_id);
    }
}

unsafe fn kernel_boot(dtb_pa: PhysicalAddress) -> ! {
    println!("Kernel booting on Hart 0. DTB: {:x?}", dtb_pa);
    smp::wait_for_ap();
    println!("Number of Harts: {}", smp::num_harts());
    layout::print();
    allocator::init();
    memory::init();
    interrupt::init();
    scheduler::init();

    init::start();
}
