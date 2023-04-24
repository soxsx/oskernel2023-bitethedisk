pub mod processor;

use alloc::sync::Arc;
pub use processor::*;

use crate::trap::TrapContext;

use super::{manager::fetch_task, task::{TaskStatus, TaskControlBlock}, TaskContext, switch::__switch};

/// 进入 idle 控制流，它运行在这个 CPU 核的启动栈上，
/// 功能是循环调用 fetch_task 直到顺利从任务管理器中取出一个任务，随后便准备通过任务切换的方式来执行
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.lock();
        // TASK_MANAGER.exclusive_access().list_alltask();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.lock();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            drop(task_inner);
            // release coming task TCB manually
            *processor.current_mut() = Some(task);
            // release processor manually
            drop(processor);
            unsafe { __switch(idle_task_cx_ptr, next_task_cx_ptr) }
        }
    }
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行的任务
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.lock().take_current()
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行任务的任务控制块的引用计数的一份拷贝
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.lock().current().clone()
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
    let mut processor = PROCESSOR.lock();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);

    unsafe { __switch(switched_task_cx_ptr, idle_task_cx_ptr) }
}
