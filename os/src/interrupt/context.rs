use riscv::register::sstatus::{Sstatus, SPP};

extern "C" {
    fn leave_interrupt(context: &Context) -> !;
}

#[repr(C)]
#[derive(Debug)]
pub struct Context {
    pub gregs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl Context {
    pub unsafe fn leave(&self) -> ! {
        leave_interrupt(self);
    }

    pub fn was_user(&self) -> bool {
        match self.sstatus.spp() {
            SPP::Supervisor => false,
            SPP::User => true,
        }
    }
}
