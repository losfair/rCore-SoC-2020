use crate::init;
use crate::interrupt;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static NUM_HARTS: AtomicU32 = AtomicU32::new(1); // boot core
static CURRENT_BOOTING: AtomicU32 = AtomicU32::new(0);
static CURRENT_BOOT_DONE: AtomicBool = AtomicBool::new(false);

pub fn num_harts() -> u32 {
    NUM_HARTS.load(Ordering::Relaxed)
}

pub fn wait_for_ap() {
    for _ in 0..1000000 {
        unsafe {
            llvm_asm!("" :::: "volatile");
        }
    }
}

pub unsafe fn ap_boot(hart_id: u32) -> ! {
    interrupt::ap_init();
    NUM_HARTS.fetch_add(1, Ordering::Relaxed);
    while CURRENT_BOOTING.load(Ordering::SeqCst) != hart_id {}
    init::ap_start(hart_id);
}

pub unsafe fn boot_ap(hart_id: u32) {
    CURRENT_BOOTING.store(hart_id, Ordering::SeqCst);
}

pub unsafe fn set_ap_boot_done() {
    CURRENT_BOOT_DONE.store(true, Ordering::SeqCst);
}

pub fn clear_ap_boot_done() {
    CURRENT_BOOT_DONE.store(false, Ordering::SeqCst);
}

pub fn ap_boot_done() -> bool {
    CURRENT_BOOT_DONE.load(Ordering::SeqCst)
}
