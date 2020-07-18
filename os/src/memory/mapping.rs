use super::LockedPagePool;
use super::{
    PageTable, PageTableEntry, PageTableEntryFlags, PageTableHandle, PhysicalPageNumber,
    VirtualPageNumber,
};
use crate::error::*;
use crate::layout;
use crate::process::ThreadToken;
use crate::scheduler::HardwareThread;
use crate::sync::without_interrupts;
use alloc::vec::Vec;
use core::ops::Range;

pub struct Mapping {
    /// All page tables used in this process.
    ///
    /// `tables[0]` is the root table.
    tables: Vec<PageTableHandle>,

    /// All non-page-table owned pages in this process.
    owned_pages: Vec<VirtualPageNumber>,

    /// PPN of the root table.
    root_ppn: PhysicalPageNumber,

    /// Page pool from which pages in this mapping are allocated from.
    pool: LockedPagePool,

    ready_for_auto_drop: bool,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub range: Range<VirtualPageNumber>,
    pub backing: SegmentBacking,
    pub flags: PageTableEntryFlags,
}

#[derive(Clone, Debug)]
pub enum SegmentBacking {
    Linear { phys_start: PhysicalPageNumber },
    Owned,
}

impl Mapping {
    pub unsafe fn new_without_kernel_region(
        pool: LockedPagePool,
        token: &ThreadToken,
    ) -> KernelResult<Self> {
        let root_table = PageTable::new(pool.clone(), token)?;
        let root_ppn = root_table.ppn();
        Ok(Mapping {
            tables: vec![root_table],
            owned_pages: vec![],
            root_ppn,
            pool,
            ready_for_auto_drop: false,
        })
    }

    pub fn release(mut self, token: &ThreadToken) {
        for &vpn in &self.owned_pages {
            self.pool.free(vpn, token);
        }
        self.ready_for_auto_drop = true;
    }

    pub fn fork(&self, pool: LockedPagePool, token: &ThreadToken) -> KernelResult<Self> {
        let mut new_mapping = unsafe { Mapping::new_without_kernel_region(pool, token)? };

        let ram_mapping_vpn = layout::ram_start().vpn();
        let levels = ram_mapping_vpn.levels();

        // We are going to reuse the first level entry (1 GB).
        // So verify that other levels are zero, just to make sure.
        for subindex in &levels[1..] {
            assert_eq!(*subindex, 0);
        }

        new_mapping.tables[0].entries[levels[0]] = self.tables[0].entries[levels[0]];

        Ok(new_mapping)
    }

    pub fn entry(
        &mut self,
        vpn: VirtualPageNumber,
        token: &ThreadToken,
    ) -> KernelResult<&mut PageTableEntry> {
        let root_table_ptr: *mut PageTable = self
            .root_ppn
            .start_address()
            .to_virt()
            .expect("Mapping::entry: bad root_ppn")
            .as_mut_ptr();
        let mut entry = &mut unsafe { &mut *root_table_ptr }.entries[vpn.levels()[0]];
        for subindex in &vpn.levels()[1..] {
            if entry.is_empty() {
                //println!("Heap usage before: {}", crate::allocator::heap_usage());
                let new_table = PageTable::new(self.pool.clone(), token)?;
                //println!("Heap usage after: {}", crate::allocator::heap_usage());
                let new_ppn = new_table.ppn();
                self.tables.push(new_table);
                *entry = PageTableEntry::new(new_ppn, PageTableEntryFlags::VALID);
            }
            entry = &mut unsafe { &mut *entry.next_level() }.entries[*subindex];
        }
        Ok(entry)
    }

    pub fn map_one(
        &mut self,
        vpn: VirtualPageNumber,
        ppn: PhysicalPageNumber,
        flags: PageTableEntryFlags,
        token: &ThreadToken,
    ) -> KernelResult<()> {
        let entry = self.entry(vpn, token)?;
        *entry = PageTableEntry::new(ppn, flags);
        Ok(())
    }

    pub fn map_segment(&mut self, seg: &Segment, token: &ThreadToken) -> KernelResult<()> {
        println!("Mapping segment: {:x?}", seg);
        for vpn in seg.range.start.0..seg.range.end.0 {
            let vpn = VirtualPageNumber(vpn);
            match seg.backing {
                SegmentBacking::Linear { mut phys_start } => {
                    // Calculate the physical address of the backing frame.
                    let page_offset = vpn.0 - seg.range.start.0;
                    phys_start.0 += page_offset;
                    self.map_one(vpn, phys_start, seg.flags, token)?;
                }
                SegmentBacking::Owned => {
                    let kernel_vpn = self.pool.allocate(token)?;
                    self.owned_pages.push(kernel_vpn);
                    self.map_one(
                        vpn,
                        kernel_vpn
                            .to_phys()
                            .expect("Mapping::map_segment: bad kernel vpn for owned segment"),
                        seg.flags,
                        token,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Activates this mapping in a thread context.
    ///
    /// This method is safe because each `Mapping` is guaranteed to include the kernel region.
    pub fn activate_thread(&self, _: &ThreadToken) {
        let new_satp = self.root_ppn.0 | (8 << 60); // Sv39
        without_interrupts(HardwareThread::this_hart(), || unsafe {
            llvm_asm!("csrw satp, $0" :: "r"(new_satp) :: "volatile");
            llvm_asm!("sfence.vma" :::: "volatile");
        });
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        if !self.ready_for_auto_drop {
            panic!("Mapping::drop: release() not called");
        }
    }
}
