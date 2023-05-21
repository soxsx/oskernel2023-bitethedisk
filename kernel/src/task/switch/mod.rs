//! 上下文切换

use super::TaskContext;

core::arch::global_asm!(include_str!("switch.S"));

extern "C" {
    /// 切换任务上下文
    ///
    /// * current_task_cx_ptr 当前任务上下文指针
    /// * next_task_cx_ptr    即将被切换到的任务上下文指针
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
