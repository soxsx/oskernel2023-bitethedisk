use super::current_task;

pub mod siginfo;

pub use siginfo::*;

pub fn check_current_signals() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.write();
    match task_inner.pending_signals {
        SigMask::SIGINT => Some((-2, "Killed, SIGINT=2")),
        SigMask::SIGILL => Some((-4, "Illegal Instruction, SIGILL=4")),
        SigMask::SIGABRT => Some((-6, "Aborted, SIGABRT=6")),
        SigMask::SIGFPE => Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8")),
        SigMask::SIGKILL => Some((-9, "Kill, SIGKILL=9")),
        SigMask::SIGSEGV => Some((-11, "Segmentation Fault, SIGSEGV=11")),
        _ => None,
    }
}

pub fn current_add_signal(signal: SigMask) {
    let task = current_task().unwrap();
    let mut task_inner = task.write();
    task_inner.pending_signals.set(signal, true);
}
