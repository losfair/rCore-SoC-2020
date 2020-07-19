mod hart;
mod plan;
mod reason;

pub use hart::{HardwareThread, Id as HardwareThreadId};
pub use plan::{Policy, PolicyContext, SimplePolicy, SwitchReason};
pub use reason::EntryReason;

use crate::sync::Once;
use alloc::boxed::Box;
use alloc::sync::Arc;

pub fn init() {
    println!("scheduler: Initialized.");
}
