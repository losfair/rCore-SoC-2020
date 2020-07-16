use super::global_wait_queue;
use crate::memory::VirtualAddress;
use crate::process::ThreadToken;
use crate::scheduler::HardwareThread;
use core::cell::UnsafeCell;
use core::marker::{PhantomData, PhantomPinned};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::sync::atomic::{AtomicU8, Ordering};

/// A `Mutex` backed by futex.
pub struct Mutex<T> {
    locked: AtomicU8,
    inner: UnsafeCell<T>,
    _pin: PhantomPinned,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(inner: T) -> Self {
        Mutex {
            locked: AtomicU8::new(0),
            inner: UnsafeCell::new(inner),
            _pin: PhantomPinned,
        }
    }

    /// Locks a pinned mutex.
    pub fn lock<'a>(
        self: Pin<&'a Self>,
        ht: &'a HardwareThread,
        token: &'a ThreadToken,
    ) -> MutexGuard<'a, T> {
        loop {
            match self.locked.compare_and_swap(0, 1, Ordering::Acquire) {
                0 => {
                    // Lock successful
                    break MutexGuard {
                        parent: self,
                        ht,
                        token,
                    };
                }
                1 => {
                    // Lock failed
                    let addr = VirtualAddress::from(&self.locked)
                        .to_phys()
                        .expect("Mutex::lock: bad self.locked vaddr");
                    global_wait_queue().wait(
                        ht,
                        addr,
                        || self.locked.load(Ordering::Relaxed) == 1,
                        token,
                    );
                }
                _ => unreachable!("Mutex::lock: got lock value other than 0 or 1"),
            }
        }
    }

    fn unlock<'a>(self: Pin<&'a Self>, ht: &'a HardwareThread, token: &'a ThreadToken) {
        assert_eq!(
            self.locked.compare_and_swap(1, 0, Ordering::Release),
            1,
            "Mutex::unlock: bad lock value"
        );

        let addr = VirtualAddress::from(&self.locked)
            .to_phys()
            .expect("Mutex::unlock: bad self.locked vaddr");
        global_wait_queue().wake_one(ht, addr, token);
    }
}

pub struct MutexGuard<'a, T> {
    parent: Pin<&'a Mutex<T>>,
    ht: &'a HardwareThread,
    token: &'a ThreadToken,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.parent.inner.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.parent.inner.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.parent.unlock(self.ht, self.token);
    }
}
