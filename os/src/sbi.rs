#![allow(unused)]

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;

/// Invokes an SBI method.
///
/// # Safety
///
/// Calling into SBI allows powerful system control operations. The caller is responsible to ensure
/// that the arguments passed to `sbi_call` are valid.
#[inline(always)]
unsafe fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    llvm_asm!("ecall"
        : "={x10}" (ret)
        : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (which)
        : "memory"
        : "volatile");
    ret
}

pub unsafe fn send_ipi(hart_mask: *const usize) {
    unsafe {
        sbi_call(SBI_SEND_IPI, hart_mask as _, 0, 0);
    }
}

/// Writes a character to the console.
pub fn console_putchar(c: u8) {
    unsafe {
        sbi_call(SBI_CONSOLE_PUTCHAR, c as _, 0, 0);
    }
}

/// Reads a character from the console. Returns `-1` for nothing.
pub fn console_getchar() -> i32 {
    unsafe { sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0) as _ }
}

/// Shuts down the system.
pub fn shutdown() -> ! {
    unsafe {
        sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    }
    unreachable!()
}

/// Schedules a timer interrupt after the `time`-th cycle.
pub fn set_timer(time: usize) {
    unsafe {
        sbi_call(SBI_SET_TIMER, time, 0, 0);
    }
}
