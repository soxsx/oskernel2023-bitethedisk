//! 全局任务管理器
//!
//! 维护一个全局的 rq
//! CPU 可以从这里获取就绪的进程以便执行
//!
use core::arch::global_asm;

use super::{TaskContext, TaskControlBlock};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;

/// 任务管理器
///
/// 目前只负责管理就绪的任务队列，该队列 `rq`(Ready queue) 是个双端队列，
/// 任务管理器将遵循 FIFO 来分配就绪的任务。
pub struct TaskManager {
    /// TaskManager 的全局等待队列
    rq: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    pub const fn new() -> Self {
        Self {
            rq: VecDeque::new(),
        }
    }

    /// 将一个任务加入队尾
    #[inline(always)]
    pub fn push_back(&mut self, task: Arc<TaskControlBlock>) {
        self.rq.push_back(task);
    }

    /// 拉取一个就绪的任务准备执行
    #[inline(always)]
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.rq.pop_front()
    }
}

lazy_static! {
    /// 全局任务管理器
    ///
    /// 每个 CPU 带有一个 Processor，Processor 自己维护一个本地 rq(等待运行的 Task)，
    /// 如果空闲，则可以通过 TASK_MANAGER 的全局 rq 中拿取一个 task 来运行，实现简单的多核调度，运用多核优势
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());

    /// 这是一个通过 PID 找 task 的哈希表
    /// 本质上是一个 BTreeMap，这里保留 UPSafeCell，因为单核情况下不太可能产生重入的情况。
    pub static ref PID_MAP: Mutex<BTreeMap<usize, Arc<TaskControlBlock>>> =
        Mutex::new(BTreeMap::new());
}

/// 将一个任务加入到全局 `FIFO 任务管理器` `TASK_MANAGER` 就绪队列的队尾
pub fn add_task(task: Arc<TaskControlBlock>) {
    PID_MAP.lock().insert(task.pid(), Arc::clone(&task));
    TASK_MANAGER.lock().push_back(task);
}

/// 从全局变量 `TASK_MANAGER` 就绪队列的队头中取出一个任务
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
}

/// 通过PID获取对应的进程控制块
#[inline]
pub fn get_task_by_pid(pid: usize) -> Option<Arc<TaskControlBlock>> {
    PID_MAP.lock().get(&pid).map(Arc::clone)
}

pub fn remove_task_with_pid(pid: usize) {
    let mut map = PID_MAP.lock();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {:?} in pid2task!", pid);
    }
}

global_asm!(include_str!("switch.S"));

// 将汇编代码中的全局符号 __switch 解释为一个 Rust 函数
extern "C" {
    /// 切换任务上下文
    ///
    /// * current_task_cx_ptr 当前任务上下文指针
    /// * next_task_cx_ptr    即将被切换到的任务上下文指针
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
