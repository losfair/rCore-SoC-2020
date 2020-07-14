use bit_field::BitField;
use riscv::register::sstatus::{Sstatus, SPP};

extern "C" {
    fn leave_interrupt(context: &Context) -> !;
}

#[repr(C)]
#[derive(Debug)]
pub struct Context {
    pub gregs: [usize; 32],
    pub sstatus: usize,
    pub sepc: usize,
}

impl Context {
    pub unsafe fn leave(&self) -> ! {
        leave_interrupt(self);
    }

    pub fn was_user(&self) -> bool {
        match self.sstatus.get_bit(8) {
            true => false,
            false => true,
        }
    }
}
