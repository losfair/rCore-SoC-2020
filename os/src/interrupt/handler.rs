use super::context::Context;
use crate::process::RawThreadState;
use crate::scheduler::{switch_to, EntryReason};
use riscv::register::{
    scause::{Exception, Interrupt, Scause, Trap},
    stvec,
};

global_asm!(include_str!("intr_entry.asm"));

extern "C" {
    fn __interrupt();
}

pub fn init() {
    unsafe {
        stvec::write(__interrupt as usize, stvec::TrapMode::Direct);
    }
    println!("interrupt/handler: Initialized.");
}

#[no_mangle]
pub extern "C" fn handle_interrupt(ts: &mut RawThreadState, scause: Scause, stval: usize) -> ! {
    match scause.cause() {
        Trap::Exception(Exception::Breakpoint) => on_breakpoint(ts),
        Trap::Interrupt(Interrupt::SupervisorTimer) => on_stimer(ts),
        _ => panic!(
            "Unknown interrupt: {:?}\n{:#x?}\nstval: {:?}",
            scause.cause(),
            ts.context,
            stval
        ),
    }
}

fn on_breakpoint(ts: &mut RawThreadState) -> ! {
    ts.context.sepc += 2;
    unsafe { ts.enter_kernel(EntryReason::Breakpoint) }
}

fn on_stimer(ts: &mut RawThreadState) -> ! {
    unsafe { ts.enter_kernel(EntryReason::Timer) }
}
