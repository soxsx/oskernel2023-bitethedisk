bitflags! {
    /// 进程信号
    #[derive(PartialEq, Eq, Debug)]
    pub struct SignalFlags: u32 {
        const SIGINT    = 1 << 2;
        const SIGILL    = 1 << 4;
        const SIGABRT   = 1 << 6;
        const SIGFPE    = 1 << 8;
        const SIGKILL   = 1 << 9;
        const SIGUSR1   = 1 << 10;
        const SIGSEGV   = 1 << 11;
    }
}
