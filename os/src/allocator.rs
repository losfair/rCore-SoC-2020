use crate::layout;

static mut HEAP_TOP: usize = 0;
const PAGE_SIZE: usize = 4096;

pub fn init() {
    unsafe {
        HEAP_TOP = round_up_to_page_size(layout::kernel_end());
    }
    println!("allocator: Initialized.");
}

fn round_up_to_page_size(x: usize) -> usize {
    if (x % PAGE_SIZE != 0) {
        (x & !(PAGE_SIZE - 1)) + PAGE_SIZE
    } else {
        x
    }
}

#[alloc_error_handler]
fn foo(_: core::alloc::Layout) -> ! {
    panic!("Allocation failed");
}

#[no_mangle]
extern "C" fn __dlmalloc_alloc(size: usize) -> usize {
    let old_top = unsafe { HEAP_TOP };
    match old_top.checked_add(size) {
        Some(x) if x <= layout::ram_end() => {
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
