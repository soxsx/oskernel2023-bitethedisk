use crate::task::manager::TASK_MANAGER;
use crate::task::{task::TaskControlBlock, TaskContext};
use alloc::sync::Arc;

/// Processor provides a series of abstractions
pub struct Processor {
    /// Current task running on this processor
    current: Option<Arc<TaskControlBlock>>,
    /// Current idle task context on this processor
    idle_task_cx: TaskContext,
}

impl Processor {
    pub const fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::empty(),
        }
    }
    pub fn idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    pub fn current(&self) -> &Option<Arc<TaskControlBlock>> {
        &self.current
    }
    pub fn current_mut(&mut self) -> &mut Option<Arc<TaskControlBlock>> {
        &mut self.current
    }
    pub fn hang_current(&mut self, sleep_time: usize, duration: usize) {
        TASK_MANAGER
            .borrow_mut()
            .hang(sleep_time, duration, self.take_current().unwrap());
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}
