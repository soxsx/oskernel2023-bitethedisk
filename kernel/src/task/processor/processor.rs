use core::cell::RefMut;

use alloc::sync::Arc;

use crate::task::manager::TASK_MANAGER;
use crate::task::{task::TaskControlBlock, TaskContext};
use sync_cell::SyncRefCell;

/// [`Processor`] 是描述 CPU执行状态 的数据结构。
/// 在单核环境下，我们仅创建单个 Processor 的全局实例 PROCESSOR
pub static mut PROCESSOR: SyncRefCell<Processor> = SyncRefCell::new(Processor::new());

/// 每个核上的处理器，负责运行一个进程
pub struct Processor {
    /// 当前处理器上正在执行的任务
    current: Option<Arc<TaskControlBlock>>,
    /// 当前处理器上的 idle 控制流的任务上下文
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

    /// 取出当前正在执行的任务
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    /// 返回当前执行的任务的一份拷贝
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

pub fn acquire_processor<'a>() -> RefMut<'a, Processor> {
    unsafe { PROCESSOR.borrow_mut() }
}
