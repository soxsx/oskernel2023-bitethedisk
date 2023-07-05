use crate::timer::get_time_ms;
use sync_cell::SyncRefCell;

use super::TaskControlBlock;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use spin::Mutex;

/// FIFO 任务管理器
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    hq: VecDeque<HangingTask>,
}

pub struct HangingTask {
    /// ms
    sleep_time: usize,
    duration: usize,
    inner: Arc<TaskControlBlock>,
}

impl HangingTask {
    pub fn new(sleep_time: usize, duration: usize, task: Arc<TaskControlBlock>) -> Self {
        Self {
            sleep_time,
            duration,
            inner: task,
        }
    }

    pub const fn limit_time(&self) -> usize {
        self.sleep_time + self.duration
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            hq: VecDeque::new(),
        }
    }

    /// 将一个任务加入队尾
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    /// 从队头中取出一个任务
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }

    pub fn hang(&mut self, sleep_time: usize, duration: usize, task: Arc<TaskControlBlock>) {
        self.hq
            .push_back(HangingTask::new(sleep_time, duration, task));
    }

    fn check_sleep(&self, hanging_task: &HangingTask) -> bool {
        let limit = hanging_task.limit_time();
        let current_time = get_time_ms();
        current_time >= limit
    }

    pub fn check_hanging(&mut self) -> Option<Arc<TaskControlBlock>> {
        if self.hq.is_empty() || !self.check_sleep(self.hq.front().unwrap()) {
            None
        } else {
            Some(self.hq.pop_front().unwrap().inner)
        }
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: SyncRefCell<TaskManager> = SyncRefCell::new(TaskManager::new());
    pub static ref PID2TCB: Mutex<BTreeMap<usize, Arc<TaskControlBlock>>> =
        Mutex::new(BTreeMap::new());
}

/// 将一个任务加入到全局 `FIFO 任务管理器` `TASK_MANAGER` 就绪队列的队尾
pub fn add_task(task: Arc<TaskControlBlock>) {
    PID2TCB.lock().insert(task.pid(), Arc::clone(&task));
    TASK_MANAGER.borrow_mut().add(task);
}

/// 从全局变量 `TASK_MANAGER` 就绪队列的队头中取出一个任务
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.borrow_mut().fetch()
}

pub fn check_hanging() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.borrow_mut().check_hanging()
}

/// 通过PID获取对应的进程控制块
#[allow(unused)]
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
