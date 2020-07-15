use super::context::Context;
use crate::process::{RawThreadState, ThreadToken};
use crate::scheduler::{EntryReason, HardwareThread};
use core::{mem, ptr};
use riscv::register::{
    scause::{Exception, Interrupt, Scause, Trap},
    stvec,
};

global_asm!(include_str!("intr_entry.asm"));

extern "C" {
    fn __interrupt();
}

/// A token that indicates execution in an interrupt context (SIE = 0, possible kernel reentry).
#[derive(Debug)]
pub struct InterruptToken(());

pub fn init() {
    unsafe {
        stvec::write(__interrupt as usize, stvec::TrapMode::Direct);
    }
    println!("interrupt/handler: Initialized.");
}

/// High-level interrupt entry.
///
/// If we were in user mode, then `context` is the first member of a `RawThreadState`.
#[no_mangle]
pub extern "C" fn handle_interrupt(context: &mut Context, scause: Scause, stval: usize) -> ! {
    let token = InterruptToken(());
    let ts: &mut RawThreadState = if context.was_user() {
        println!("user mode interrupt entry");
        unsafe { mem::transmute(context) }
    } else {
        let ts: &mut RawThreadState;
        unsafe {
            let x: &mut HardwareThread;
            llvm_asm!("mv $0, gp" : "=r"(x) :::);
            ts = x.current_mut().raw_thread_state_mut();
            ptr::copy(context, &mut ts.kcontext, 1);
        }
        ts.kcontext_valid = 1;
        ts
    };
    match scause.cause() {
        Trap::Exception(Exception::Breakpoint) => on_breakpoint(ts, &token),
        Trap::Interrupt(Interrupt::SupervisorTimer) => on_stimer(ts, &token),
        _ => panic!(
            "Unknown interrupt: {:?}\n{:#x?}\nstval: {:?}",
            scause.cause(),
            ts.last_context(),
            stval
        ),
    }
}

fn on_breakpoint(ts: &mut RawThreadState, token: &InterruptToken) -> ! {
    let bkpt_addr = ts.last_context_mut().sepc;
    ts.last_context_mut().sepc += 2;
    unsafe {
        ts.enter_kernel(
            token,
            EntryReason::Breakpoint(bkpt_addr, ThreadToken::assume_synchronous_exception(token)),
        )
    }
}

fn on_stimer(ts: &mut RawThreadState, token: &InterruptToken) -> ! {
    unsafe { ts.enter_kernel(token, EntryReason::Timer) }
}
