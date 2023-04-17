bitflags! {
    /// 进程信号
    #[derive(PartialEq, Eq, Debug)]
    pub struct SignalFlags: u32 {   /// - Killed
        const SIGINT    = 1 << 2;   /// - Illegal Instruction
        const SIGILL    = 1 << 4;   /// - Aborted
        const SIGABRT   = 1 << 6;   /// - Erroneous Arithmetic Operation
        const SIGFPE    = 1 << 8;   /// - Segmentation Fault
        const SIGKILL   = 1 << 9;
        const SIGUSR1   = 1 << 10;
        const SIGSEGV   = 1 << 11;
    }
}
