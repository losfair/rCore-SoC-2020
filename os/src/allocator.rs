use crate::layout;

static mut HEAP_TOP: usize = 0;
const PAGE_SIZE: usize = 4096;

pub fn init() {
    unsafe {
        HEAP_TOP = layout::kernel_end().0;
        assert!(HEAP_TOP % PAGE_SIZE == 0);
    }
    println!("allocator: Initialized.");
}

pub fn heap_usage() -> usize {
    unsafe { HEAP_TOP - layout::kernel_end().0 }
}

#[alloc_error_handler]
fn foo(_: core::alloc::Layout) -> ! {
    panic!("Allocation failed");
}

#[no_mangle]
extern "C" fn __dlmalloc_alloc(size: usize) -> usize {
    let old_top = unsafe { HEAP_TOP };
    match old_top.checked_add(size) {
        Some(x) if x <= layout::ram_end().0 => {
            unsafe {
                // dlmalloc zeros allocated memory.
                HEAP_TOP = x;
            }
            old_top
        }
        _ => usize::MAX,
    }
}

#[no_mangle]
extern "C" fn __dlmalloc_acquire_global_lock() {}

#[no_mangle]
extern "C" fn __dlmalloc_release_global_lock() {}

#[no_mangle]
static __DLMALLOC_PAGE_SIZE: usize = PAGE_SIZE;
