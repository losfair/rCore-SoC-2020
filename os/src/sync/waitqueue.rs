use super::{without_interrupts, IntrCell};
use crate::memory::PhysicalAddress;
use crate::process::Thread;
use crate::process::ThreadToken;
use crate::scheduler::{global_plan, HardwareThread, PolicyContext};
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::linked_list::LinkedList;
use spin::{Mutex as SpinMutex, MutexGuard as SpinMutexGuard};

static GLOBAL_WAIT_QUEUE: WaitQueue = WaitQueue::new();

pub struct WaitQueue {
    wakeup_sets: SpinMutex<BTreeMap<PhysicalAddress, LinkedList<Box<Thread>>>>,
}

impl WaitQueue {
    const fn new() -> WaitQueue {
        WaitQueue {
            wakeup_sets: SpinMutex::new(BTreeMap::new()),
        }
    }

    fn lock_wakeup_sets<'a>(
        &'a self,
        ht: &HardwareThread,
        token: &ThreadToken,
    ) -> SpinMutexGuard<'a, BTreeMap<PhysicalAddress, LinkedList<Box<Thread>>>> {
        assert!(
            ht.has_active_intr_guards() == false,
            "lock_wakeup_sets: bad has_active_intr_guards"
        );
        loop {
            match self.wakeup_sets.try_lock() {
                Some(x) => break x,
                None => {
                    ht.do_yield(token);
                }
            }
        }
    }

    /// Wakes a thread waiting on `addr`.
    ///
    /// Must only be called from a thread context because of possible allocator reentry.
    pub fn wake_one(&self, ht: &HardwareThread, addr: PhysicalAddress, token: &ThreadToken) {
        let mut wakeup_sets = self.lock_wakeup_sets(ht, token);

        let th = match wakeup_sets.get_mut(&addr) {
            Some(s) => {
                let elem = s
                    .pop_front()
                    .expect("WaitQueue::wake_one: empty wait queue in non-empty entry");
                if s.len() == 0 {
                    wakeup_sets.remove(&addr).unwrap();
                }
                Some(elem)
            }
            None => None,
        };

        if let Some(th) = th {
            global_plan().add_thread(ht, PolicyContext::NonCritical(token), th);
        }
    }

    /// Registers the current thread to wait on `addr`.
    ///
    /// Must only be called from a thread context because of possible allocator reentry.
    pub fn wait<F: FnOnce() -> bool>(
        &self,
        ht: &HardwareThread,
        addr: PhysicalAddress,
        condition: F,
        token: &ThreadToken,
    ) {
        let mut wakeup_sets = self.lock_wakeup_sets(ht, token);

        if condition() {
            unsafe {
                ht.release_current(
                    move |current| {
                        wakeup_sets
                            .entry(addr)
                            .or_insert(LinkedList::new())
                            .push_back(current);
                        drop(wakeup_sets); // release lock
                    },
                    token,
                );
            }
        }
    }
}

pub fn global_wait_queue() -> &'static WaitQueue {
    &GLOBAL_WAIT_QUEUE
}
