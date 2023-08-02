use core::cell::RefMut;

use alloc::sync::Arc;

use crate::task::{
    check_hanging,
    initproc::BUSYBOX,
    manager::{check_futex_interupt_or_expire, fetch_task},
    recycle_child_threads_res,
    switch::__switch,
    task::TaskStatus,
    unblock_task, TaskContext, TaskControlBlock,
};
use alloc::sync::Arc;
use core::cell::RefMut;

#[cfg(feature = "static_busybox")]
use crate::task::initproc::BUSYBOX;

/// Loop calling fetch_task until a task is successfully retrieved from the task manager,
/// and then prepare to execute it by task switching
pub fn run_tasks() {
    #[cfg(feature = "static_busybox")]
    {
        let busybox = BUSYBOX.read();
        drop(busybox);
    }
    loop {
        let processor = acquire_processor();

        recycle_child_threads_res();

        if let Some(hanging_task) = check_hanging() {
            run_task(hanging_task, processor);
        } else if let Some(interupt_task) = check_futex_interupt_or_expire() {
            unblock_task(interupt_task);
        } else if let Some(task) = fetch_task() {
            run_task(task, processor);
        }
    }
}

/// Switch to the idle task. Idle task runs on the startup stack of this CPU core.
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
