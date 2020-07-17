use crate::process::ThreadToken;

#[derive(Debug)]
pub enum EntryReason {
    Syscall,
    PageFault,
    Timer,
    Breakpoint(usize),
}
