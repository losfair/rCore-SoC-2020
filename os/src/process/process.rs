use crate::error::*;
use crate::memory::{boot_mapping, LockedPagePool, Mapping, Segment};
use crate::sync::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub struct Process {
    mapping: Mapping,
    segments: Vec<Segment>,
}

#[derive(Clone)]
pub struct LockedProcess(Arc<Mutex<Process>>);

impl Process {
    pub fn new(pool: LockedPagePool) -> KernelResult<Process> {
        Ok(Process {
            mapping: boot_mapping().fork(pool)?,
            segments: vec![],
        })
    }
}

impl LockedProcess {
    pub fn new(pool: LockedPagePool) -> KernelResult<LockedProcess> {
        Process::new(pool).map(|x| LockedProcess(Arc::new(Mutex::new(x))))
    }
}
