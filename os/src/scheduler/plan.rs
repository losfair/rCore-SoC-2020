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

/// Per-hart scheduling policy.
pub trait Policy<T> {
    fn add_thread(&self, ht: &HardwareThread, context: PolicyContext, thread: Box<T>);
    fn next(
        &self,
        ht: &HardwareThread,
        context: PolicyContext,
        reason: SwitchReason,
    ) -> Option<Box<T>>;
}

pub struct SimplePolicy<T> {
    critical_buffer: SpinMutex<CriticalBuffer<T>>,
    remaining_ticks: AtomicU32,
    max_ticks: u32,
}

struct CriticalBuffer<T> {
    local_run_queue: Box<ArrayDeque<[Box<T>; 512]>>,
}

impl<T> CriticalBuffer<T> {
    fn new() -> CriticalBuffer<T> {
        CriticalBuffer {
            local_run_queue: Box::new(ArrayDeque::new()),
        }
    }
}

impl<T> SimplePolicy<T> {
    pub fn new() -> SimplePolicy<T> {
        SimplePolicy {
            critical_buffer: SpinMutex::new(CriticalBuffer::new()),
            remaining_ticks: AtomicU32::new(0),
            max_ticks: 10,
        }
    }
}

impl<T: Send> Policy<T> for SimplePolicy<T> {
    fn add_thread(&self, ht: &HardwareThread, context: PolicyContext, thread: Box<T>) {
        match context {
            PolicyContext::NonCritical(token) => without_interrupts(ht, || {
                let mut buffer = self.critical_buffer.try_lock().expect(
                    "SimplePolicy::add_thread: cannot lock critical buffer in non-critical path",
                );
                buffer
                    .local_run_queue
                    .push_back(thread)
                    .expect("SimplePolicy::add_thread: critical buffer full");
            }),
            PolicyContext::Critical => {
                let mut buffer = self.critical_buffer.try_lock().expect(
                    "SimplePolicy::add_thread: cannot lock critical buffer in critical path",
                );
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
                attempt_switch = match self.remaining_ticks.load(Ordering::Relaxed) {
                    0 => {
                        self.remaining_ticks
                            .store(self.max_ticks, Ordering::Relaxed);
                        true
                    }
                    x => {
                        self.remaining_ticks.store(x - 1, Ordering::Relaxed);
                        false
                    }
                };
            }
        }
        if attempt_switch {
            match context {
                PolicyContext::NonCritical(token) => without_interrupts(ht, || {
                    let mut buffer = self.critical_buffer.try_lock().expect(
                        "SimplePolicy::next: cannot lock critical buffer in non-critical path",
                    );
                    buffer.local_run_queue.pop_front()
                }),
                PolicyContext::Critical => {
                    // Ensure to free up space for one following `add_thread`.
                    let mut buffer = self
                        .critical_buffer
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
