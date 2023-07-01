use core::cell::RefMut;

use alloc::{sync::Arc, vec::Vec};

use crate::task::{processor::cpu::Cpu, task::TaskControlBlock, TaskContext};

/// - Processor 是描述 CPU执行状态 的数据结构。
/// - 在单核CPU环境下，我们仅创建单个 Processor 的全局实例 PROCESSOR
#[cfg(not(feature = "multi_harts"))]
pub static mut PROCESSOR: Cpu = Cpu::new();

#[cfg(feature = "multi_harts")]
pub static mut PROCESSORS: [Cpu; 2] = [Cpu::new(), Cpu::new()];

/// 每个核上的处理器，负责运行一个进程
pub struct Processor {
    /// 当前处理器上正在执行的任务
    current: Option<Arc<TaskControlBlock>>,
    /// 当前处理器上的 idle 控制流的任务上下文
    idle_task_cx: TaskContext,

    /// 挂起的进程，需要在 [`run_tasks`] 检查是否达到可以运行的状态
    ///
    /// [`run_tasks`]: super::schedule::run_tasks
    hq: HangUpQueue,
}

pub struct HangingTask {
    ready: bool,
    inner: Arc<TaskControlBlock>,
}

impl HangingTask {
    pub fn new(task: Arc<TaskControlBlock>) -> Self {
        Self {
            ready: false,
            inner: task,
        }
    }

    pub fn is_ready(mut self, checker: &dyn Fn(&mut Self) -> bool) -> bool {
        checker(&mut self)
    }
}

pub struct HangUpQueue(pub Vec<HangingTask>);

impl HangUpQueue {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn try_fetch(&mut self) {
        todo!()
    }
}

impl Processor {
    pub const fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::empty(),
            hq: HangUpQueue::new(),
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
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn acquire_processor<'a>() -> RefMut<'a, Processor> {
    #[cfg(not(feature = "multi_harts"))]
    {
        unsafe { PROCESSOR.get_mut() }
    }

    #[cfg(feature = "multi_harts")]
    {
        use super::processor::PROCESSORS;
        unsafe { PROCESSORS[hartid!()].get_mut() }
    }
}
