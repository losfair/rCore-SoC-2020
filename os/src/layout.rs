use crate::memory::VirtualAddress;

extern "C" {
    static KERNEL_START: Data;
    static TEXT_START: Data;
    static RODATA_START: Data;
    static DATA_START: Data;
    static BSS_START: Data;
    static KERNEL_END: Data;
}

const RAM_START: usize = 0xffffffff80000000;
const RAM_END: usize = 0xffffffff88000000;
const KERNEL_IDMAP_START: usize = 0xffffffff00000000;

pub enum Data {}

pub fn print() {
    unsafe {
        println!("Kernel image layout:");
        println!("- Kernel start: {:p}", &KERNEL_START);
        println!("- Text start: {:p}", &TEXT_START);
        println!("- RoData start: {:p}", &RODATA_START);
        println!("- Data start: {:p}", &DATA_START);
        println!("- BSS start: {:p}", &BSS_START);
        println!("- Kernel end: {:p}", &KERNEL_END);
        println!("Assuming start of RAM at {:p}", RAM_START as *const u8);
        println!("Assuming end of RAM at {:p}", RAM_END as *const u8);
    }
}

pub fn kernel_start() -> VirtualAddress {
    VirtualAddress::from(unsafe { &KERNEL_START })
}

pub fn text_start() -> VirtualAddress {
    VirtualAddress::from(unsafe { &TEXT_START })
}

pub fn rodata_start() -> VirtualAddress {
    VirtualAddress::from(unsafe { &RODATA_START })
}

pub fn data_start() -> VirtualAddress {
    VirtualAddress::from(unsafe { &DATA_START })
}

pub fn bss_start() -> VirtualAddress {
    VirtualAddress::from(unsafe { &BSS_START })
}

pub fn kernel_end() -> VirtualAddress {
    VirtualAddress::from(unsafe { &KERNEL_END })
}

pub fn ram_start() -> VirtualAddress {
    VirtualAddress(RAM_START)
}

pub fn ram_end() -> VirtualAddress {
    VirtualAddress(RAM_END)
}

pub fn kernel_idmap_start() -> VirtualAddress {
    VirtualAddress(KERNEL_IDMAP_START)
}
