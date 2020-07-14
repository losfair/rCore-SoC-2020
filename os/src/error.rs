#[derive(Debug)]
#[repr(i32)]
pub enum KernelError {
    OutOfMemory = -1,
}

pub type KernelResult<T> = Result<T, KernelError>;
