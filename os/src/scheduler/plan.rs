use super::HardwareThreadId;
use crate::process::{LockedProcess, ProcessId, Thread, ThreadToken};
use crate::sync::without_interrupts;
use crate::sync::Mutex;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::mem;
use core::sync::atomic::{AtomicU32, Ordering};

pub struct GlobalPlan {
    processes: Mutex<BTreeMap<ProcessId, LockedProcess>>,
    policy: Box<dyn Policy<Thread>>,
}

#[derive(Copy, Clone, Debug)]
pub enum SwitchReason {
    Yield,
    Periodic,
}

/// A scheduling policy.
pub trait Policy<T: Send>: Send + Sync {
    fn add_thread(&self, thread: Box<T>);
    fn next(
        &self,
        ht_id: HardwareThreadId,
        reason: SwitchReason,
        token: &ThreadToken,
    ) -> Option<Box<T>>;
    fn drain_threads(&mut self) -> Vec<Box<T>>;
}

pub struct SimplePolicy<T> {
    run_queue: Mutex<VecDeque<Box<T>>>,
    remaining_ticks_per_ht: Vec<AtomicU32>,
    max_ticks: u32,
}

impl<T> SimplePolicy<T> {
    pub fn new(num_hardware_threads: usize) -> SimplePolicy<T> {
        SimplePolicy {
            run_queue: Mutex::new(VecDeque::new()),
            remaining_ticks_per_ht: (0..num_hardware_threads)
                .map(|_| AtomicU32::new(0))
                .collect(),
            max_ticks: 10,
        }
    }
}

impl<T: Send> Policy<T> for SimplePolicy<T> {
    fn add_thread(&self, thread: Box<T>) {
        self.run_queue.lock().push_back(thread);
    }
    fn next(
        &self,
        ht_id: HardwareThreadId,
        reason: SwitchReason,
        _: &ThreadToken,
    ) -> Option<Box<T>> {
        let attempt_switch: bool;

        match reason {
            SwitchReason::Yield => {
                // The thread requested to yield itself and give up its remaining time slice.
                attempt_switch = true;
            }
            SwitchReason::Periodic => {
                // Periodic timer interrupt.
                let remaining_ticks: &AtomicU32 = &self.remaining_ticks_per_ht[ht_id.0 as usize];
                attempt_switch = match remaining_ticks.load(Ordering::Relaxed) {
                    0 => {
                        remaining_ticks.store(self.max_ticks, Ordering::Relaxed);
                        true
                    }
                    x => {
                        remaining_ticks.store(x - 1, Ordering::Relaxed);
                        false
                    }
                };
            }
        }
        if attempt_switch {
            self.run_queue.lock().pop_front()
        } else {
            None
        }
    }
    fn drain_threads(&mut self) -> Vec<Box<T>> {
        Vec::from(mem::replace(&mut *self.run_queue.lock(), VecDeque::new()))
    }
}

impl GlobalPlan {
    pub fn new(policy: Box<dyn Policy<Thread>>) -> GlobalPlan {
        GlobalPlan {
            processes: Mutex::new(BTreeMap::new()),
            policy,
        }
    }

    pub fn add_thread(&self, thread: Box<Thread>) {
        self.policy.add_thread(thread)
    }

    pub fn next(
        &self,
        ht_id: HardwareThreadId,
        reason: SwitchReason,
        token: &ThreadToken,
    ) -> Option<Box<Thread>> {
        self.policy.next(ht_id, reason, token)
    }
}
