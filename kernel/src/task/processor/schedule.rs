//! 单/多核的调度逻辑
//!
//! 包括获取当前 CPU 上的计算单元 [`super::Processor`]，修改进程状态，调度进程

use crate::task::{switch::__switch, TaskContext};

/// 进入 idle 控制流，它运行在这个 CPU 核的启动栈上，
/// 功能是循环调用 fetch_task 直到顺利从任务管理器中取出一个任务，随后便准备通过任务切换的方式来执行
#[cfg(not(feature = "multi_harts"))]
pub fn run_tasks() {
    use crate::task::{manager::fetch_task, processor::acquire_processor, task::TaskStatus};

    loop {
        let mut processor = acquire_processor();

        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.write();
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

#[cfg(feature = "multi_harts")]
pub fn run_tasks() -> ! {
    use super::acquire_processor;
    use crate::task::{manager::fetch_task, task::TaskStatus};

    loop {
        if let Some(task) = fetch_task() {
            info!("task {} fetched by hart {}", task.pid(), hartid!());
            let mut processor = acquire_processor();
            let idle_task_cx_ptr = processor.idle_task_cx_ptr();

            let mut task_inner = task.write();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            drop(task_inner);

            *processor.current_mut() = Some(task);

            drop(processor);

            unsafe { __switch(idle_task_cx_ptr, next_task_cx_ptr) }
        }
    }
}
