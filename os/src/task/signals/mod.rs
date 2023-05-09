use super::current_task;

pub mod signal_flags;

pub use signal_flags::SignalFlags;

pub fn check_signals_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.lock();
    match task_inner.signals {
        SignalFlags::SIGINT => Some((-2, "Killed, SIGINT=2")),
        SignalFlags::SIGILL => Some((-4, "Illegal Instruction, SIGILL=4")),
        SignalFlags::SIGABRT => Some((-6, "Aborted, SIGABRT=6")),
        SignalFlags::SIGFPE => Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8")),
        SignalFlags::SIGKILL => Some((-9, "Kill, SIGKILL=9")),
        SignalFlags::SIGSEGV => Some((-11, "Segmentation Fault, SIGSEGV=11")),
        _ => None,
    }
}

pub fn current_add_signal(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.lock();
    task_inner.signals.set(signal, true);
}
