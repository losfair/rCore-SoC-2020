extern "C" {
    static KERNEL_START: Data;
    static TEXT_START: Data;
    static RODATA_START: Data;
    static DATA_START: Data;
    static BSS_START: Data;
    static KERNEL_END: Data;
}

const RAM_END: usize = 0x88000000;

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
        println!("Assuming end of RAM at {:p}", RAM_END as *const u8);
    }
}

pub fn kernel_start() -> usize {
    unsafe { &KERNEL_START as *const Data as usize }
}

pub fn text_start() -> usize {
    unsafe { &TEXT_START as *const Data as usize }
}

pub fn rodata_start() -> usize {
    unsafe { &RODATA_START as *const Data as usize }
}

pub fn data_start() -> usize {
    unsafe { &DATA_START as *const Data as usize }
}

pub fn bss_start() -> usize {
    unsafe { &BSS_START as *const Data as usize }
}

pub fn kernel_end() -> usize {
    unsafe { &KERNEL_END as *const Data as usize }
}

pub fn ram_end() -> usize {
    RAM_END
}
