use crate::process::ThreadToken;
use crate::scheduler::HardwareThread;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU8, Ordering};

pub struct YieldMutex<T> {
    inner: UnsafeCell<T>,
    lock: AtomicU8,
}

unsafe impl<T: Send> Send for YieldMutex<T> {}
unsafe impl<T: Send> Sync for YieldMutex<T> {}

impl<T> YieldMutex<T> {
    pub const fn new(inner: T) -> YieldMutex<T> {
        YieldMutex {
            inner: UnsafeCell::new(inner),
            lock: AtomicU8::new(0),
        }
    }

    pub fn lock<'a>(&'a self, token: &'a ThreadToken) -> YieldMutexGuard<'a, T> {
        loop {
            match self.lock.compare_and_swap(0, 1, Ordering::Acquire) {
                0 => {
                    // Lock successful
                    break YieldMutexGuard { parent: self };
                }
                1 => {
                    HardwareThread::this_hart().do_yield(token);
                }
                _ => unreachable!("YieldMutex::lock: broken invariant"),
            }
        }
    }

    fn unlock(&self) {
        self.lock.store(0, Ordering::Release);
    }
}

pub struct YieldMutexGuard<'a, T> {
    parent: &'a YieldMutex<T>,
}

impl<'a, T> Deref for YieldMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.parent.inner.get() }
    }
}

impl<'a, T> DerefMut for YieldMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.parent.inner.get() }
    }
}

impl<'a, T> Drop for YieldMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.parent.unlock();
    }
}
