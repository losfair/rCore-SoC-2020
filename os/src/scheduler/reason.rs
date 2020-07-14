#[derive(Debug)]
pub enum EntryReason {
    Syscall,
    PageFault,
    Timer,
    Breakpoint,
}
