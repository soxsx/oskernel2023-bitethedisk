pub mod cpu;
pub mod processor;
pub mod schedule;

use alloc::sync::Arc;
pub use processor::*;
pub use schedule::*;

use crate::trap::TrapContext;

use super::{switch::__switch, task::TaskControlBlock, TaskContext};

/// 从全局变量 `PROCESSOR` 中取出当前正在执行的任务
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().take_current()
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行任务的任务控制块的引用计数的一份拷贝
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().current().clone()
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行任务的用户地址空间 token
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.lock().get_user_token();

    token
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().lock().trap_context()
}

/// 换到 idle 控制流并开启新一轮的任务调度
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = acquire_processor();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);

    unsafe { __switch(switched_task_cx_ptr, idle_task_cx_ptr) }
}
