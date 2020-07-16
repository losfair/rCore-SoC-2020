use super::{without_interrupts, IntrCell};
use crate::memory::PhysicalAddress;
use crate::process::Thread;
use crate::process::ThreadToken;
use crate::scheduler::{global_plan, HardwareThread};
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::linked_list::LinkedList;
use spin::Mutex as SpinMutex;

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

    /// Wakes a thread waiting on `addr`.
    ///
    /// Must only be called from a thread context because of possible allocator reentry.
    pub fn wake_one(&self, ht: &HardwareThread, addr: PhysicalAddress, _: &ThreadToken) {
        // Preempting out a thread that holds a spinlock is not great :)
        without_interrupts(ht, || {
            let mut wakeup_sets = self.wakeup_sets.lock();

            // TODO: Make allocator spin on its lock when SIE is disabled.
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
                global_plan().add_thread(th);
            }
        });
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
        // Preempting out a thread that holds a spinlock is not great, so we mask off interrupts first
        let intr_cell = IntrCell::new(());
        let intr_guard = intr_cell.borrow_mut(ht);

        let mut wakeup_sets = self.wakeup_sets.lock();
        if condition() {
            unsafe {
                ht.release_current(
                    move |current| {
                        wakeup_sets
                            .entry(addr)
                            .or_insert(LinkedList::new())
                            .push_back(current);
                        drop(wakeup_sets); // release lock
                        drop(intr_guard); // release interrupt guard
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
