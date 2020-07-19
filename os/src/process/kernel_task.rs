use super::{Thread, ThreadToken};
use crate::error::*;
use crate::scheduler::{HardwareThread, PolicyContext};
use alloc::boxed::Box;
use core::mem;
use core::raw::TraitObject;

pub trait KernelTask {
    fn run(self: Box<Self>, ht: &HardwareThread, token: &ThreadToken);
}

pub fn create_kernel_thread(task: Box<dyn KernelTask>) -> KernelResult<Box<Thread>> {
    let obj: TraitObject = unsafe { mem::transmute(task) };
    let th = Thread::new(
        second_level_trampoline,
        obj.data as usize,
        obj.vtable as usize,
    )?;
    Ok(th)
}

pub fn spawn(
    ht: &HardwareThread,
    task: Box<dyn KernelTask>,
    token: &ThreadToken,
) -> KernelResult<()> {
    let th = create_kernel_thread(task)?;
    ht.policy()
        .add_thread(ht, PolicyContext::NonCritical(token), th);
    Ok(())
}

fn second_level_trampoline(
    ht: &HardwareThread,
    token: &ThreadToken,
    data: usize,
    vtable: usize,
) -> ! {
    let task: Box<dyn KernelTask> = unsafe {
        mem::transmute(TraitObject {
            data: data as _,
            vtable: vtable as _,
        })
    };
    task.run(ht, token);
    ht.exit_thread(token);
}
