use crate::memory::boot_page_pool;
use crate::process::{spawn, KernelTask, LockedProcess, Thread, ThreadToken};
use crate::scheduler::{global_plan, HardwareThread, HardwareThreadId};
use alloc::boxed::Box;

pub fn start() -> ! {
    let mut ht = HardwareThread::new(
        HardwareThreadId(0),
        global_plan().clone(),
        make_init_thread(),
    );
    unsafe { ht.start() }
}

fn make_init_thread() -> Box<Thread> {
    Thread::new(init_thread, 0, 0).unwrap()
}

fn init_thread(ht: &mut HardwareThread, token: &ThreadToken, _: usize, _: usize) -> ! {
    println!("Init thread started.");
    spawn(Box::new(YieldThread(0)), token).unwrap();
    spawn(Box::new(YieldThread(1)), token).unwrap();
    ht.exit_thread(token);
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
        for i in 0..10 {
            println!("yield thread: {}", self.0);
            ht.do_yield(token);
        }
    }
}
