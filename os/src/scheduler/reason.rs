use crate::process::ThreadToken;

#[derive(Debug)]
pub enum EntryReason<'a> {
    Syscall,
    PageFault,
    Timer,
    Breakpoint(usize, &'a ThreadToken),
}
