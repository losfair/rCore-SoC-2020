pub mod lock;
mod waitqueue;
mod yield_mutex;

pub use spin::{Mutex, MutexGuard, Once};
pub use waitqueue::{global_wait_queue, WaitQueue};
pub use yield_mutex::{YieldMutex, YieldMutexGuard};

use crate::interrupt::InterruptToken;
use crate::scheduler::HardwareThread;
use core::cell::{Ref, RefCell, RefMut};
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

pub struct IntrCell<T> {
    inner: RefCell<T>,
}

impl<T> IntrCell<T> {
    pub const fn new(inner: T) -> IntrCell<T> {
        IntrCell {
            inner: RefCell::new(inner),
        }
    }

    pub fn borrow_mut<'a>(&'a self, ht: &'a HardwareThread) -> IntrGuardMut<'a, T> {
        unsafe {
            ht.acquire_intr_guard();
        }
        IntrGuardMut {
            inner: ManuallyDrop::new(self.inner.borrow_mut()),
            ht,
        }
    }

    pub fn borrow_mut_intr<'a>(&'a self, _: &'a InterruptToken) -> RefMut<'a, T> {
        self.inner.borrow_mut()
    }
}

pub struct IntrGuardMut<'a, T: 'a> {
    inner: ManuallyDrop<RefMut<'a, T>>,
    ht: &'a HardwareThread,
}

impl<'a, T: 'a> Deref for IntrGuardMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.inner.deref().deref()
    }
}

impl<'a, T: 'a> DerefMut for IntrGuardMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.inner.deref_mut().deref_mut()
    }
}

impl<'a, T: 'a> Drop for IntrGuardMut<'a, T> {
    fn drop(&mut self) {
        // Decrement borrow count and then release interrupt guard.
        unsafe {
            ManuallyDrop::drop(&mut self.inner);
            self.ht.release_intr_guard();
        }
    }
}

pub fn without_interrupts<F: FnOnce() -> R, R>(ht: &HardwareThread, f: F) -> R {
    let cell = IntrCell::new(());
    let _guard = cell.borrow_mut(ht);
    f()
}
