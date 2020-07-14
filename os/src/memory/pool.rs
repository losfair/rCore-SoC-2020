use super::{VirtualAddress, VirtualPageNumber};
use crate::error::*;
use crate::sync::Mutex;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
use alloc::sync::Arc;
use alloc::vec::Vec;

const PAGES_PER_SET: u8 = 64; // 256 KB

pub struct PagePool {
    sets: Vec<PageSetInfo>,
    usable_pages: BTreeSet<(u32, u8)>, // (set_index, page_index)
    allocated_pages: BTreeMap<VirtualPageNumber, (u32, u8)>,

    /// Used to determine when to shrink.
    free_count_before_shrink: usize,
}

#[derive(Clone)]
pub struct LockedPagePool(Arc<Mutex<PagePool>>);

struct PageSetInfo {
    set: Box<PageSet>,
    used_pages: usize,
}

#[repr(C)]
struct PageSet {
    pages: [Page; PAGES_PER_SET as usize],
}

#[repr(C, align(4096))]
struct Page([u8; 4096]);

impl LockedPagePool {
    pub fn new() -> LockedPagePool {
        LockedPagePool(Arc::new(Mutex::new(PagePool::new())))
    }

    pub fn allocate(&self) -> KernelResult<VirtualPageNumber> {
        self.0.lock().allocate()
    }

    pub fn free(&self, vpn: VirtualPageNumber) {
        self.0.lock().free(vpn)
    }
}

impl PagePool {
    pub fn new() -> PagePool {
        PagePool {
            sets: Vec::new(),
            usable_pages: BTreeSet::new(),
            allocated_pages: BTreeMap::new(),

            free_count_before_shrink: 0,
        }
    }

    pub fn allocate(&mut self) -> KernelResult<VirtualPageNumber> {
        match self.usable_pages.pop_first() {
            Some((major, minor)) => {
                let set_info = &mut self.sets[major as usize];
                let page = &mut set_info.set.pages[minor as usize] as *mut Page;
                set_info.used_pages += 1;
                let vpn = VirtualAddress::from(page).vpn();
                self.allocated_pages.insert(vpn, (major, minor));
                Ok(vpn)
            }
            None => {
                self.grow()?;
                self.allocate()
            }
        }
    }

    pub fn free(&mut self, vpn: VirtualPageNumber) {
        let (major, minor) = match self.allocated_pages.remove(&vpn) {
            Some(x) => x,
            None => panic!(
                "PagePool::free: Attempting to free a non-existing page: {:x?}",
                vpn
            ),
        };
        let set_info = &mut self.sets[major as usize];
        let page = &mut set_info.set.pages[minor as usize];

        // Zero out the freed page.
        for b in &mut page.0 as &mut [u8] {
            *b = 0;
        }

        set_info.used_pages -= 1;
        self.usable_pages.insert((major, minor));

        if self.free_count_before_shrink == 64 {
            self.free_count_before_shrink = 0;
            self.shrink();
        } else {
            self.free_count_before_shrink += 1;
        }
    }

    fn grow(&mut self) -> KernelResult<()> {
        let new_set_info = PageSetInfo {
            set: unsafe { Box::new_zeroed().assume_init() },
            used_pages: 0,
        };
        let major_index = self.sets.len() as u32;
        self.sets.push(new_set_info);
        for minor_index in 0..PAGES_PER_SET {
            self.usable_pages.insert((major_index, minor_index));
        }
        Ok(())
    }

    fn shrink(&mut self) {
        loop {
            let last = match self.sets.last() {
                Some(x) => x,
                None => break,
            };
            if last.used_pages == 0 {
                let major = (self.sets.len() - 1) as u32;
                for minor in 0..PAGES_PER_SET {
                    assert!(
                        self.usable_pages.remove(&(major, minor)) == true,
                        "PagePool::shrink: last set does not match `usable_pages`"
                    );
                }
                self.sets.pop().unwrap();
            } else {
                break;
            }
        }
    }
}
