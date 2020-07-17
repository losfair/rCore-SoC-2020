use crate::allocator;
use crate::memory::boot_page_pool;
use crate::process::{spawn, KernelTask, LockedProcess, Thread, ThreadToken};
use crate::sbi;
use crate::scheduler::{global_plan, HardwareThread, HardwareThreadId};
use crate::sync::lock;
use crate::tests;
use alloc::boxed::Box;
use core::pin::Pin;
pub fn start() -> ! {
    let ht = HardwareThread::new(
        HardwareThreadId(0),
        global_plan().clone(),
        make_init_thread(),
    );
    unsafe { ht.start() }
}

fn make_init_thread() -> Box<Thread> {
    Thread::new(init_thread, 0, 0).unwrap()
}

fn init_thread(ht: &HardwareThread, token: &ThreadToken, _: usize, _: usize) -> ! {
    allocator::enable_locking();
    println!("Init thread started.");

    run_tests(ht, token);

    sbi::shutdown();
    //ht.exit_thread(token);
}

fn run_tests(ht: &HardwareThread, token: &ThreadToken) {
    println!("running tests");

    tests::test_mutex(ht, token);

    println!("all tests passed");
}
