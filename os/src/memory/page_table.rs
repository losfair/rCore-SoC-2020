use super::LockedPagePool;
use super::{PhysicalPageNumber, VirtualAddress};
use crate::error::*;
use alloc::boxed::Box;
use bit_field::BitField;
use bitflags::bitflags;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

#[repr(C, align(4096))]
#[derive(Clone)]
pub struct Table {
    pub entries: [Entry; 512],
}

pub struct TableHandle {
    table: *mut Table,
    pool: LockedPagePool,
}

unsafe impl Send for TableHandle {}
unsafe impl Sync for TableHandle {}

impl Drop for TableHandle {
    fn drop(&mut self) {
        self.pool.free(VirtualAddress::from(self.table).vpn());
    }
}

impl Deref for TableHandle {
    type Target = Table;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.table }
    }
}

impl DerefMut for TableHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.table }
    }
}

impl Table {
    pub fn new(pool: LockedPagePool) -> KernelResult<TableHandle> {
        pool.allocate().map(|vpn| TableHandle {
            table: vpn.start_address().as_mut_ptr(),
            pool: pool.clone(),
        })
    }

    pub fn ppn(&self) -> PhysicalPageNumber {
        VirtualAddress::from(self)
            .to_phys()
            .expect("Table::ppn: bad address")
            .ppn()
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(transparent)]
pub struct Entry(usize);

impl Entry {
    pub fn new(page_number: PhysicalPageNumber, flags: Flags) -> Self {
        Entry(
            *0usize
                .set_bits(0..8, flags.bits())
                .set_bits(10..54, page_number.0),
        )
    }

    pub fn get(&self) -> usize {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn next_level(&self) -> *mut Table {
        self.ppn()
            .start_address()
            .to_virt()
            .expect("Entry::next_level: bad address")
            .as_mut_ptr()
    }

    pub fn ppn(&self) -> PhysicalPageNumber {
        PhysicalPageNumber(self.0.get_bits(10..54))
    }

    pub fn flags(&self) -> Flags {
        Flags::from_bits(self.0.get_bits(0..8)).unwrap()
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: usize {
        const VALID = 1 << 0;
        const READABLE = 1 << 1;
        const WRITABLE = 1 << 2;
        const EXECUTABLE = 1 << 3;
        const USER = 1 << 4;
        const GLOBAL = 1 << 5;
        const ACCESSED = 1 << 6;
        const DIRTY = 1 << 7;
    }
}
