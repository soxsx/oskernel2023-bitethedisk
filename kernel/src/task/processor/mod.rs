pub mod processor;
pub mod schedule;

use super::{switch::__switch, task::TaskControlBlock, TaskContext};
use crate::trap::TrapContext;
use alloc::sync::Arc;
use core::cell::RefMut;
pub use processor::*;
pub use schedule::*;

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().current().clone()
}

pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let memory_set = task.memory_set.read();
    let token = memory_set.token();
    drop(memory_set);
    token
}
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().inner_mut().trap_context()
}
/// Switch to idle control flow and start a new task scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = acquire_processor();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);
    unsafe { __switch(switched_task_cx_ptr, idle_task_cx_ptr) }
}

pub fn acquire_processor<'a>() -> RefMut<'a, Processor> {
    PROCESSORS[hartid!()].borrow_mut()
}
