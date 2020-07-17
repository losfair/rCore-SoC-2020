use crate::memory::boot_page_pool;
use crate::process::{spawn, KernelTask, LockedProcess, Thread, ThreadToken};
use crate::scheduler::{global_plan, HardwareThread, HardwareThreadId};
use crate::sync::lock;
use alloc::boxed::Box;
use core::pin::Pin;
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

fn init_thread(ht: &HardwareThread, token: &ThreadToken, _: usize, _: usize) -> ! {
    println!("Init thread started.");
    spawn(ht, Box::new(YieldThread(0)), token).unwrap();
    spawn(ht, Box::new(YieldThread(1)), token).unwrap();
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

static TEST_MUTEX: lock::Mutex<usize> = lock::Mutex::new(0);

fn test_mutex() -> Pin<&'static lock::Mutex<usize>> {
    unsafe { Pin::new_unchecked(&TEST_MUTEX) }
}

struct YieldThread(usize);
impl KernelTask for YieldThread {
    fn run(self: Box<Self>, ht: &HardwareThread, token: &ThreadToken) {
        println!("thread {} begins to wait for mutex", self.0);
        for i in 0..10000 {
            let mut guard = test_mutex().lock(ht, token);
            println!("thread {} acquired mutex. value = {}", self.0, *guard);
            *guard += 1;
            for j in 0..5 {
                ht.do_yield(token);
            }
            println!("thread {} will release mutex", self.0);
            drop(guard);
            ht.do_yield(token);
            ht.do_yield(token);
        }
    }
}
