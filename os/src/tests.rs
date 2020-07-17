use crate::process::{spawn, KernelTask, ThreadToken};
use crate::scheduler::HardwareThread;
use alloc::boxed::Box;
use core::mem;
use core::pin::Pin;

pub fn test_mutex(ht: &HardwareThread, token: &ThreadToken) {
    use crate::sync::lock;
    static TEST_MUTEX: lock::Mutex<usize> = lock::Mutex::new(0);
    static COMPLETION_1: lock::Mutex<()> = lock::Mutex::new(());
    static COMPLETION_2: lock::Mutex<()> = lock::Mutex::new(());

    fn get_test_mutex() -> Pin<&'static lock::Mutex<usize>> {
        unsafe { Pin::new_unchecked(&TEST_MUTEX) }
    }

    fn get_completion_1() -> Pin<&'static lock::Mutex<()>> {
        unsafe { Pin::new_unchecked(&COMPLETION_1) }
    }

    fn get_completion_2() -> Pin<&'static lock::Mutex<()>> {
        unsafe { Pin::new_unchecked(&COMPLETION_2) }
    }

    struct YieldThread(usize, lock::MutexGuard<'static, ()>);
    impl KernelTask for YieldThread {
        fn run(self: Box<Self>, ht: &HardwareThread, token: &ThreadToken) {
            //println!("thread {} begins to wait for mutex", self.0);
            for i in 0..10000 {
                let mut guard = get_test_mutex().lock(token);
                //println!("thread {} acquired mutex. value = {}", self.0, *guard);
                *guard += 1;
                for j in 0..5 {
                    ht.do_yield(token);
                }
                //println!("thread {} will release mutex", self.0);
                drop(guard);
                ht.do_yield(token);
            }
        }
    }

    let token: &'static ThreadToken = unsafe { mem::transmute(token) };

    println!("running test: test_mutex");

    spawn(
        ht,
        Box::new(YieldThread(0, get_completion_1().lock(token))),
        token,
    )
    .unwrap();
    spawn(
        ht,
        Box::new(YieldThread(1, get_completion_2().lock(token))),
        token,
    )
    .unwrap();

    get_completion_1().lock(token);
    get_completion_2().lock(token);

    assert_eq!(
        *get_test_mutex().lock(token),
        20000,
        "test_mutex: final result mismatch"
    );

    println!("test_mutex ok");
}
