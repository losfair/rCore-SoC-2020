mod address;
mod boot;
mod mapping;
mod page_table;
mod pool;

pub use address::{PhysicalAddress, PhysicalPageNumber, VirtualAddress, VirtualPageNumber};
pub use mapping::{Mapping, Segment, SegmentBacking};
pub use page_table::{
    Entry as PageTableEntry, Flags as PageTableEntryFlags, Table as PageTable,
    TableHandle as PageTableHandle,
};
pub use pool::{LockedPagePool, PagePool};

use crate::sync::Once;

static mut BOOT_MAPPING: Option<Mapping> = None;
static BOOT_PAGE_POOL: Once<LockedPagePool> = Once::new();

pub fn init() {
    unsafe {
        BOOT_MAPPING = Some(
            boot::remap_kernel(boot_page_pool().clone())
                .expect("memory::init: remap_kernel failed"),
        );
    }
    println!("memory: Initialized.");
}

pub fn boot_page_pool() -> &'static LockedPagePool {
    BOOT_PAGE_POOL.call_once(|| LockedPagePool::new())
}

pub fn boot_mapping() -> &'static Mapping {
    unsafe {
        BOOT_MAPPING
            .as_ref()
            .expect("boot_mapping: BOOT_MAPPING not initialized")
    }
}
