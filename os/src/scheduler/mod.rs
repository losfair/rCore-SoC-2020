mod hart;
mod plan;
mod reason;

pub use hart::{HardwareThread, Id as HardwareThreadId};
pub use plan::{GlobalPlan, Policy, PolicyContext, SimplePolicy, SwitchReason};
pub use reason::EntryReason;

use crate::sync::Once;
use alloc::boxed::Box;
use alloc::sync::Arc;

static GLOBAL_PLAN: Once<Arc<GlobalPlan>> = Once::new();

pub fn global_plan() -> &'static Arc<GlobalPlan> {
    GLOBAL_PLAN.call_once(|| Arc::new(GlobalPlan::new(Box::new(SimplePolicy::new(1)))))
}

pub fn init() {
    global_plan();
    println!("scheduler: Initialized.");
}
