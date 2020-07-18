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

use crate::process::ThreadToken;
use crate::sync::Once;

static mut BOOT_MAPPING: Option<Mapping> = None;
static BOOT_PAGE_POOL: Once<LockedPagePool> = Once::new();

pub fn init() {
    println!("memory: Initialized.");
}

pub fn boot_page_pool() -> &'static LockedPagePool {
    BOOT_PAGE_POOL.call_once(|| LockedPagePool::new())
}

/// Remaps kernel memory to enforce protections and remove the first 4 GB identity mapping.
///
/// # Safety
///
/// Can only be called once, and before the first call to `boot_mapping`.
pub unsafe fn remap_kernel(token: &ThreadToken) {
    assert!(
        BOOT_MAPPING.is_none(),
        "memory::remap_kernel: BOOT_MAPPING is not empty"
    );
    BOOT_MAPPING = Some(
        boot::remap_kernel(boot_page_pool().clone(), token)
            .expect("memory::remap_kernel: remap_kernel failed"),
    );
    println!("memory: Kernel remapped.");
}

pub fn boot_mapping() -> &'static Mapping {
    unsafe {
        BOOT_MAPPING
            .as_ref()
            .expect("boot_mapping: BOOT_MAPPING not initialized")
    }
}
