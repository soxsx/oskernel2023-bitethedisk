pub mod processor;
pub mod schedule;

use alloc::sync::Arc;
use core::cell::RefMut;
use sync_cell::SyncRefCell;
mod processor;
mod schedule;
use super::{switch::__switch, task::TaskControlBlock, TaskContext};
use crate::trap::TrapContext;
pub use processor::*;
pub use schedule::*;

use crate::trap::TrapContext;

use super::{
    manager::CHILDREN_THREAD_MONITOR, switch::__switch, task::TaskControlBlock, TaskContext,
};

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().current().clone()
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
/// Switch to idle control flow and start a new task scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = acquire_processor();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);
    unsafe { __switch(switched_task_cx_ptr, idle_task_cx_ptr) }
}

pub static mut PROCESSOR: SyncRefCell<Processor> = SyncRefCell::new(Processor::new());
pub fn acquire_processor<'a>() -> RefMut<'a, Processor> {
    unsafe { PROCESSOR.borrow_mut() }
}
