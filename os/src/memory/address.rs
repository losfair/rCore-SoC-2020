use crate::layout;
use bit_field::BitField;

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct VirtualPageNumber(pub usize);

impl VirtualPageNumber {
    pub fn levels(self) -> [usize; 3] {
        [
            self.0.get_bits(18..27),
            self.0.get_bits(9..18),
            self.0.get_bits(0..9),
        ]
    }

    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress(self.0 << 12)
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct PhysicalPageNumber(pub usize);

impl PhysicalPageNumber {
    pub fn start_address(self) -> PhysicalAddress {
        PhysicalAddress(self.0 << 12)
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct VirtualAddress(pub usize);

impl VirtualAddress {
    pub fn vpn(self) -> VirtualPageNumber {
        VirtualPageNumber(self.0 >> 12)
    }

    pub fn to_phys(self) -> Option<PhysicalAddress> {
        self.0
            .checked_sub(layout::kernel_idmap_start().0)
            .map(PhysicalAddress)
    }

    pub fn as_ptr<T>(self) -> *const T {
        self.0 as _
    }

    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as _
    }
}

impl<T> From<*mut T> for VirtualAddress {
    fn from(other: *mut T) -> Self {
        VirtualAddress(other as _)
    }
}

impl<T> From<*const T> for VirtualAddress {
    fn from(other: *const T) -> Self {
        VirtualAddress(other as _)
    }
}

impl<T> From<&mut T> for VirtualAddress {
    fn from(other: &mut T) -> Self {
        VirtualAddress(other as *mut T as _)
    }
}

impl<T> From<&T> for VirtualAddress {
    fn from(other: &T) -> Self {
        VirtualAddress(other as *const T as _)
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct PhysicalAddress(pub usize);

impl PhysicalAddress {
    pub fn ppn(self) -> PhysicalPageNumber {
        PhysicalPageNumber(self.0 >> 12)
    }

    pub fn to_virt(self) -> Option<VirtualAddress> {
        self.0
            .checked_add(layout::kernel_idmap_start().0)
            .map(VirtualAddress)
    }
}
