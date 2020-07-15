use crate::memory::boot_page_pool;
use crate::process::{spawn, KernelTask, LockedProcess, Thread, ThreadToken};
use crate::scheduler::{global_plan, HardwareThread, HardwareThreadId, SimplePolicy};
use crate::sync::Once;
use alloc::boxed::Box;
use alloc::sync::Arc;

pub fn start() -> ! {
    let mut ht = HardwareThread::new(
        HardwareThreadId(0),
        global_plan().clone(),
        make_init_thread(),
    );
    unsafe { ht.force_return_to_current() }
}

fn make_init_thread() -> Box<Thread> {
    Thread::new(init_thread, 0, 0).unwrap()
}

fn init_thread(ht: &mut HardwareThread, token: &ThreadToken, _: usize, _: usize) -> ! {
    println!("Init thread started.");
    spawn(Box::new(YieldThread(0)), token);
    spawn(Box::new(YieldThread(1)), token);
    let mut i: usize = 0;
    loop {
        println!("init_thread: {}", i);
        i += 1;
        if i == 10 {
            loop {}
        }
        ht.do_yield(token);
    }
}

struct YieldThread(usize);
impl KernelTask for YieldThread {
    fn run(self: Box<Self>, ht: &mut HardwareThread, token: &ThreadToken) {
        loop {
            println!("yield thread: {}", self.0);
            ht.do_yield(token);
        }
        drop(self);
    }
}
