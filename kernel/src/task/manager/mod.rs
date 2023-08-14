use alloc::{collections::BTreeMap, sync::Arc};
use spin::Mutex;
use sync_cell::SyncRefCell;
mod hanging_task;
mod task_manager;
use super::TaskControlBlock;
pub use hanging_task::*;
pub use task_manager::*;

lazy_static! {
    pub static ref TASK_MANAGER: SyncRefCell<TaskManager> = SyncRefCell::new(TaskManager::new());
}
pub fn add_task(task: Arc<TaskControlBlock>) {
    PID2TCB.lock().insert(task.pid(), Arc::clone(&task));
    TASK_MANAGER.borrow_mut().add(task);
}
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.borrow_mut().fetch()
}
pub fn check_hanging() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.borrow_mut().check_hanging()
}
pub fn check_futex_interupt_or_expire() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.borrow_mut().check_futex_interupt_or_expire()
}
pub fn unblock_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.borrow_mut().unblock_task(task);
}
pub fn block_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.borrow_mut().block(task);
}

pub static THREAD_CLEANER: Mutex<CancelledThreads> = Mutex::new(CancelledThreads::new());
pub fn recycle_child_threads_res() {
    THREAD_CLEANER.lock().clear_all();
}
pub fn collect_cancelled_chiled_thread(child_thread: Arc<TaskControlBlock>) {
    THREAD_CLEANER.lock().push(child_thread);
}

lazy_static! {
    pub static ref PID2TCB: Mutex<BTreeMap<usize, Arc<TaskControlBlock>>> =
        Mutex::new(BTreeMap::new());
}
pub fn pid2task(pid: usize) -> Option<Arc<TaskControlBlock>> {
    let map = PID2TCB.lock();
    map.get(&pid).map(Arc::clone)
}
pub fn remove_from_pid2task(pid: usize) {
    let mut map = PID2TCB.lock();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task!", pid);
    }
}
