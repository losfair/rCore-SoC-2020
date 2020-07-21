use crate::allocator;
use crate::memory::{boot_page_pool, remap_kernel};
use crate::process::{spawn, KernelTask, LockedProcess, Thread, ThreadToken};
use crate::sbi;
use crate::scheduler::{HardwareThread, HardwareThreadId, SimplePolicy};
use crate::smp;
use crate::sync::lock;
use crate::tests;
use alloc::boxed::Box;
use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};

pub fn start() -> ! {
    let ht = HardwareThread::new(
        HardwareThreadId(0),
        Box::new(SimplePolicy::new()),
        make_init_thread(),
    );
    unsafe { ht.start() }
}

pub unsafe fn ap_start(hart_id: u32) -> ! {
    println!("AP start: {}", hart_id);
    let ht = HardwareThread::new(
        HardwareThreadId(hart_id),
        Box::new(SimplePolicy::new()),
        make_apd_thread(),
    );
    unsafe { ht.start() }
}

fn make_init_thread() -> Box<Thread> {
    Thread::new(init_thread, 0, 0).unwrap()
}

/// Application Processor Daemon thread.
fn make_apd_thread() -> Box<Thread> {
    Thread::new(apd_thread, 0, 0).unwrap()
}

fn init_thread(ht: &HardwareThread, token: &ThreadToken, _: usize, _: usize) -> ! {
    unsafe {
        // Apply necessary memory protections.
        remap_kernel(token);
    }

    println!("Init thread started. Starting application processors.");
    for i in 1..smp::num_harts() {
        // Now locking is not yet enabled. So serially boot APs.
        print!("Hart {}... ", i);
        unsafe {
            smp::boot_ap(i);
            while !smp::ap_boot_done() {}
            smp::clear_ap_boot_done();
        }
        println!("ok.");
    }

    unsafe {
        // Enable the global allocator lock.
        allocator::enable_locking();
    }

    println!("Allocator locks enabled.");

    run_tests(ht, token);

    sbi::shutdown();
    //ht.exit_thread(token);
}

fn apd_thread(ht: &HardwareThread, token: &ThreadToken, _: usize, _: usize) -> ! {
    unsafe {
        smp::set_ap_boot_done();
    }
    loop {
        for _ in 0..1000000 {
            unsafe {
                llvm_asm!("" :::: "volatile");
            }
        }
        println!("apd thread tick");
    }
}

fn run_tests(ht: &HardwareThread, token: &ThreadToken) {
    println!("running tests");

    tests::test_mutex(ht, token);

    println!("all tests passed");
}
