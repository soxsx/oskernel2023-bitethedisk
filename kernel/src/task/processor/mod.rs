//! processor 提供了一系列的抽象
//!
//! CPU 被抽象为 [`cpu::Cpu`] 对象，指代一个实际的具有 id 的物理 CPU
//!
//! [`Processor`] 是一个物理上的计算单元，和一个具体的 [`cpu::Cpu`] 绑定，
//! 它事实上负责进程 [`TaskControlBlock`] 的运行和进程上下文 [`TaskContext`] 的切换
use alloc::sync::Arc;
use core::cell::RefMut;
use sync_cell::SyncRefCell;
mod processor;
mod schedule;
use super::{switch::__switch, task::TaskControlBlock, TaskContext};
use crate::trap::TrapContext;
pub use processor::*;
pub use schedule::*;

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().take_current()
}
pub fn current_task() -> Arc<TaskControlBlock> {
    acquire_processor().current().clone().unwrap()
}
pub fn current_user_token() -> usize {
    let task = current_task();
    let memory_set = task.memory_set.read();
    let token = memory_set.token();
    drop(memory_set);
    token
}
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().inner_mut().trap_context()
}
/// 换到 idle 控制流并开启新一轮的任务调度
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = acquire_processor();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);
    unsafe { __switch(switched_task_cx_ptr, idle_task_cx_ptr) }
}

/// [`Processor`] 是描述 CPU执行状态 的数据结构。
/// 在单核环境下，我们仅创建单个 Processor 的全局实例 PROCESSOR
pub static mut PROCESSOR: SyncRefCell<Processor> = SyncRefCell::new(Processor::new());
pub fn acquire_processor<'a>() -> RefMut<'a, Processor> {
    unsafe { PROCESSOR.borrow_mut() }
}
