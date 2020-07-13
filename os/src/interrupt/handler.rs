use super::context::Context;
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
pub extern "C" fn handle_interrupt(context: &mut Context, scause: Scause, stval: usize) -> ! {
    match scause.cause() {
        Trap::Exception(Exception::Breakpoint) => on_breakpoint(context),
        Trap::Interrupt(Interrupt::SupervisorTimer) => on_stimer(context),
        _ => panic!(
            "Unknown interrupt: {:?}\n{:#x?}\nstval: {:?}",
            scause.cause(),
            context,
            stval
        ),
    }
}

fn on_breakpoint(context: &mut Context) -> ! {
    println!("Breakpoint at 0x{:x}", context.sepc);
    context.sepc += 2;
    unsafe {
        context.leave();
    }
}

fn on_stimer(context: &mut Context) -> ! {
    static mut TICKS: usize = 0;
    unsafe {
        TICKS += 1;
        if TICKS % 100 == 0 {
            println!("{} ticks", TICKS);
        }
        switch_to(context);
    }
}
