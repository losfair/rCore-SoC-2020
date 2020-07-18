use super::ThreadToken;
use crate::error::*;
use crate::memory::{boot_mapping, LockedPagePool, Mapping, Segment};
use crate::sync::lock::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, Ordering};

pub struct Process {
    id: Id,
    mapping: Mapping,
    segments: Vec<Segment>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Id(pub u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct LockedProcess(Pin<Arc<Mutex<Process>>>);

impl Process {
    pub fn new(pool: LockedPagePool, token: &ThreadToken) -> KernelResult<Process> {
        Ok(Process {
            id: Id(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
            mapping: boot_mapping().fork(pool, token)?,
            segments: vec![],
        })
    }

    pub fn id(&self) -> Id {
        self.id
    }
}

impl LockedProcess {
    pub fn new(pool: LockedPagePool, token: &ThreadToken) -> KernelResult<LockedProcess> {
        Process::new(pool, token).map(|x| LockedProcess(Arc::pin(Mutex::new(x))))
    }
}
