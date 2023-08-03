use super::current_task;
mod siginfo;
pub use siginfo::*;

pub fn current_add_signal(signal: SigMask) {
    let task = current_task();
    let mut task_inner = task.inner_mut();
    task_inner.pending_signals.set(signal, true);
}
