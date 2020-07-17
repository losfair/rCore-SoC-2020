use crate::layout;
use crate::process::ThreadToken;
use crate::scheduler::HardwareThread;
use crate::sync::{YieldMutex, YieldMutexGuard};
use core::pin::Pin;

static mut HEAP_TOP: usize = 0;
const PAGE_SIZE: usize = 4096;

/// Mutex for the global allocator.
///
/// Using `YieldMutex` instead of sleeping mutex here to prevent re-entering the allocator itself.
static GLOBAL_MUTEX: YieldMutex<()> = YieldMutex::new(());
static mut LOCKING: bool = false;

pub fn init() {
    unsafe {
        HEAP_TOP = layout::kernel_end().0;
        assert!(HEAP_TOP % PAGE_SIZE == 0);
    }
    println!("allocator: Initialized.");
}

pub fn enable_locking() {
    unsafe {
        assert!(
            LOCKING == false,
            "allocator::enable_locking: attempting to enable locking twice"
        );
        LOCKING = true;
    }
    println!("allocator: Locking enabled.");
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
extern "C" fn __dlmalloc_acquire_global_lock() {
    unsafe {
        if LOCKING {
            let hart = HardwareThread::this_hart();
            //println!("acquire allocator lock on hart {:?} ({:p})", hart.id(), hart);
            hart.put_allocator_mutex_guard(GLOBAL_MUTEX.lock(ThreadToken::new()));
            //println!("acquire allocator lock done");
        }
    }
}

#[no_mangle]
extern "C" fn __dlmalloc_release_global_lock() {
    unsafe {
        if LOCKING {
            //println!("release allocator lock");
            let hart = HardwareThread::this_hart();
            hart.drop_allocator_mutex_guard();
            //println!("release allocator lock done");
        }
    }
}

#[no_mangle]
static __DLMALLOC_PAGE_SIZE: usize = PAGE_SIZE;
