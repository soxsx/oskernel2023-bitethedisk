use crate::trap::TrapContext;

use nix::SigMask;

use super::current_task;

pub fn current_add_signal(signal: SigMask) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_mut();
    task_inner.pending_signals.set(signal, true);
}

// Pushing the signal processing context onto the stack.
// [man7: 关于 signal context 的要求](https://man7.org/linux/man-pages/man7/signal.7.html)
#[derive(Debug, Clone)]
#[repr(C)]
pub struct SignalContext {
    pub context: TrapContext,
    pub mask: SigMask,
}

impl SignalContext {
    pub fn from_another(cx: &TrapContext, mask: SigMask) -> Self {
        SignalContext {
            context: cx.clone(),
            mask: mask.clone(),
        }
    }
}
