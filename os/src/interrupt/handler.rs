use super::context::Context;
use crate::process::RawThreadState;
use crate::scheduler::switch_to;
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
    println!("Breakpoint at 0x{:x}", ts.context.sepc);
    ts.context.sepc += 2;
    unsafe {
        ts.context.leave();
    }
}

fn on_stimer(ts: &mut RawThreadState) -> ! {
    static mut TICKS: usize = 0;
    unsafe {
        TICKS += 1;
        if TICKS % 100 == 0 {
            println!("{} ticks", TICKS);
        }
        switch_to(&ts.context);
    }
}
