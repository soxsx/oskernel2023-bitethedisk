//! 单/多核的调度逻辑
//!
//! 包括获取当前 CPU 上的计算单元 [`super::Processor`]，修改进程状态，调度进程

use core::cell::RefMut;

use alloc::sync::Arc;

use crate::task::{
    check_hanging,
    manager::{check_interupt, fetch_task},
    switch::__switch,
    task::TaskStatus,
    unblock_task, TaskContext, TaskControlBlock,
};

use super::{acquire_processor, Processor};

/// 进入 idle 控制流，它运行在这个 CPU 核的启动栈上，
/// 功能是循环调用 fetch_task 直到顺利从任务管理器中取出一个任务，随后便准备通过任务切换的方式来执行
pub fn run_tasks() {
    loop {
        let processor = acquire_processor();

        if let Some(hanging_task) = check_hanging() {
            run_task(hanging_task, processor);
        } else if let Some(interupt_task) = check_interupt() {
            unblock_task(interupt_task);
        } else if let Some(task) = fetch_task() {
            run_task(task, processor);
        }
    }
}

fn run_task(task: Arc<TaskControlBlock>, mut processor: RefMut<'_, Processor>) {
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    let mut task_inner = task.inner_mut();
    let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
    task_inner.task_status = TaskStatus::Running;
    drop(task_inner);
    *processor.current_mut() = Some(task);
    drop(processor);
    unsafe { __switch(idle_task_cx_ptr, next_task_cx_ptr) }
}
