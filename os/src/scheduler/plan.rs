use super::HardwareThread;
use crate::process::{LockedProcess, ProcessId, Thread, ThreadToken};
use crate::sync::lock::Mutex;
use crate::sync::{without_interrupts, IntrCell};
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use arraydeque::ArrayDeque;
use core::mem;
use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex as SpinMutex;

pub struct GlobalPlan {
    processes: Mutex<BTreeMap<ProcessId, LockedProcess>>,
    policy: Box<dyn Policy<Thread>>,
}

#[derive(Copy, Clone, Debug)]
pub enum SwitchReason {
    Yield,
    Periodic,
}

/// The context in which a method on `Policy` is called.
#[derive(Copy, Clone, Debug)]
pub enum PolicyContext<'a> {
    /// Critical context: Scheduler.
    ///
    /// Locks should be used with care, and allocations are not allowed.
    Critical,

    /// Non-critical context.
    NonCritical(&'a ThreadToken),
}

/// A scheduling policy.
pub trait Policy<T: Send>: Send + Sync {
    fn add_thread(&self, ht: &HardwareThread, context: PolicyContext, thread: Box<T>);
    fn next(
        &self,
        ht: &HardwareThread,
        context: PolicyContext,
        reason: SwitchReason,
    ) -> Option<Box<T>>;
}

pub struct SimplePolicy<T> {
    run_queue: Pin<Box<Mutex<VecDeque<Box<T>>>>>,
    critical_buffers: Vec<SpinMutex<CriticalBuffer<T>>>,
    remaining_ticks_per_ht: Vec<AtomicU32>,
    max_ticks: u32,
}

struct CriticalBuffer<T> {
    local_run_queue: ArrayDeque<[Box<T>; 16]>,
}

impl<T> CriticalBuffer<T> {
    fn new() -> CriticalBuffer<T> {
        CriticalBuffer {
            local_run_queue: ArrayDeque::new(),
        }
    }
}

impl<T> SimplePolicy<T> {
    pub fn new(num_hardware_threads: usize) -> SimplePolicy<T> {
        SimplePolicy {
            run_queue: Box::pin(Mutex::new(VecDeque::new())),
            critical_buffers: (0..num_hardware_threads)
                .map(|_| SpinMutex::new(CriticalBuffer::new()))
                .collect(),
            remaining_ticks_per_ht: (0..num_hardware_threads)
                .map(|_| AtomicU32::new(0))
                .collect(),
            max_ticks: 10,
        }
    }
}

impl<T: Send> SimplePolicy<T> {
    fn refill_local_queue(&self, ht: &HardwareThread, token: &ThreadToken) {
        let (len, cap) = without_interrupts(ht, || {
            let mut buffer = self.critical_buffers[ht.id().0 as usize].try_lock().expect("SimplePolicy::refill_local_queue: cannot lock critical buffer in non-critical path");
            (
                buffer.local_run_queue.len(),
                buffer.local_run_queue.capacity(),
            )
        });
        if len < cap {
            let mut rq = self.run_queue.as_ref().lock(ht, token);
            for _ in len..cap {
                let maybe_thread = rq.pop_front();
                if let Some(th) = maybe_thread {
                    let push_result = without_interrupts(ht, || {
                        let mut buffer = self.critical_buffers[ht.id().0 as usize].try_lock().expect("SimplePolicy::refill_local_queue: cannot lock critical buffer in non-critical path");
                        buffer.local_run_queue.push_back(th)
                    });
                    match push_result {
                        Ok(()) => {}
                        Err(e) => {
                            // Local buffer is full.
                            rq.push_front(e.element);
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }
}

impl<T: Send> Policy<T> for SimplePolicy<T> {
    fn add_thread(&self, ht: &HardwareThread, context: PolicyContext, thread: Box<T>) {
        match context {
            PolicyContext::NonCritical(token) => {
                /*
                self.run_queue.as_ref().lock(ht, token).push_back(thread);
                self.refill_local_queue(ht, token);
                */
                without_interrupts(ht, || {
                    let mut buffer = self.critical_buffers[ht.id().0 as usize]
                        .try_lock()
                        .expect("SimplePolicy::add_thread: cannot lock critical buffer");
                    buffer
                        .local_run_queue
                        .push_back(thread)
                        .expect("SimplePolicy::add_thread: critical buffer full");
                })
            }
            PolicyContext::Critical => {
                let mut buffer = self.critical_buffers[ht.id().0 as usize]
                    .try_lock()
                    .expect("SimplePolicy::add_thread: cannot lock critical buffer");
                buffer
                    .local_run_queue
                    .push_back(thread)
                    .expect("SimplePolicy::add_thread: critical buffer full");
            }
        }
    }
    fn next(
        &self,
        ht: &HardwareThread,
        context: PolicyContext,
        reason: SwitchReason,
    ) -> Option<Box<T>> {
        let attempt_switch: bool;

        match reason {
            SwitchReason::Yield => {
                // The thread requested to yield itself and give up its remaining time slice.
                attempt_switch = true;
            }
            SwitchReason::Periodic => {
                // Periodic timer interrupt.
                let remaining_ticks: &AtomicU32 = &self.remaining_ticks_per_ht[ht.id().0 as usize];
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
            match context {
                PolicyContext::NonCritical(token) => {
                    /*
                    let maybe_local = without_interrupts(ht, || {
                        let mut buffer = self.critical_buffers[ht.id().0 as usize].try_lock().expect("SimplePolicy::next: cannot lock critical buffer in non-critical path");
                        buffer.local_run_queue.pop_front()
                    });
                    let result = match maybe_local {
                        Some(x) => Some(x),
                        None => self.run_queue.as_ref().lock(ht, token).pop_front(),
                    };
                    result
                    */
                    without_interrupts(ht, || {
                        let mut buffer = self.critical_buffers[ht.id().0 as usize]
                            .try_lock()
                            .expect(
                            "SimplePolicy::next: cannot lock critical buffer in non-critical path",
                        );
                        buffer.local_run_queue.pop_front()
                    })
                }
                PolicyContext::Critical => {
                    // Ensure to free up space for one following `add_thread`.
                    let mut buffer = self.critical_buffers[ht.id().0 as usize]
                        .try_lock()
                        .expect("SimplePolicy::next: cannot lock critical buffer in critical path");
                    buffer.local_run_queue.pop_front()
                }
            }
        } else {
            None
        }
    }
}

impl GlobalPlan {
    pub fn new(policy: Box<dyn Policy<Thread>>) -> GlobalPlan {
        GlobalPlan {
            processes: Mutex::new(BTreeMap::new()),
            policy,
        }
    }

    pub fn add_thread(&self, ht: &HardwareThread, context: PolicyContext, thread: Box<Thread>) {
        self.policy.add_thread(ht, context, thread)
    }

    pub fn next(
        &self,
        ht: &HardwareThread,
        context: PolicyContext,
        reason: SwitchReason,
    ) -> Option<Box<Thread>> {
        self.policy.next(ht, context, reason)
    }
}
