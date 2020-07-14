use super::LockedPagePool;
use super::{
    PageTable, PageTableEntry, PageTableEntryFlags, PageTableHandle, PhysicalPageNumber,
    VirtualAddress, VirtualPageNumber,
};
use crate::error::*;
use alloc::vec::Vec;
use core::ops::Range;

pub struct Mapping {
    /// All page tables used in this process.
    ///
    /// `tables[0]` is the root table.
    tables: Vec<PageTableHandle>,

    /// PPN of the root table.
    root_ppn: PhysicalPageNumber,

    /// Page pool from which pages in this mapping are allocated from.
    pool: LockedPagePool,
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
}

impl Mapping {
    pub fn new(pool: LockedPagePool) -> KernelResult<Self> {
        let root_table = PageTable::new(pool.clone())?;
        let root_ppn = root_table.ppn();
        Ok(Mapping {
            tables: vec![root_table],
            root_ppn,
            pool,
        })
    }

    pub fn entry(&mut self, vpn: VirtualPageNumber) -> KernelResult<&mut PageTableEntry> {
        let root_table_ptr: *mut PageTable = self
            .root_ppn
            .start_address()
            .to_virt()
            .expect("Mapping::entry: bad root_ppn")
            .as_mut_ptr();
        let mut entry = &mut unsafe { &mut *root_table_ptr }.entries[vpn.levels()[0]];
        for subindex in &vpn.levels()[1..] {
            if entry.is_empty() {
                println!("Heap usage before: {}", crate::allocator::heap_usage());
                let new_table = PageTable::new(self.pool.clone())?;
                println!("Heap usage after: {}", crate::allocator::heap_usage());
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
    ) -> KernelResult<()> {
        let entry = self.entry(vpn)?;
        *entry = PageTableEntry::new(ppn, flags);
        Ok(())
    }

    pub fn map_segment(&mut self, seg: &Segment) -> KernelResult<()> {
        println!("Mapping segment: {:x?}", seg);
        for vpn in seg.range.start.0..seg.range.end.0 {
            let vpn = VirtualPageNumber(vpn);
            match seg.backing {
                SegmentBacking::Linear { mut phys_start } => {
                    // Calculate the physical address of the backing frame.
                    let page_offset = vpn.0 - seg.range.start.0;
                    phys_start.0 += page_offset;
                    self.map_one(vpn, phys_start, seg.flags)?;
                }
            }
        }
        Ok(())
    }

    pub unsafe fn activate(&self) {
        let new_satp = self.root_ppn.0 | (8 << 60); // Sv39
        llvm_asm!("csrw satp, $0" :: "r"(new_satp) :: "volatile");
        llvm_asm!("sfence.vma" :::: "volatile");
    }
}
