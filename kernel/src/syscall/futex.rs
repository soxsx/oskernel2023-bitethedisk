use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::RwLock;

use crate::task::TaskControlBlock;
use crate::timer::get_time_us;

pub struct FutexWaiter {
    pub task: Arc<TaskControlBlock>,
    expire_time: usize,
}

impl FutexWaiter {
    pub fn new(task: Arc<TaskControlBlock>, current_time: usize, timeout: usize) -> Self {
        Self {
            task,
            expire_time: if current_time <= usize::MAX - timeout {
                current_time + timeout
            } else {
                usize::MAX
            },
        }
    }

    pub fn check_expire(&self) -> bool {
        get_time_us() >= self.expire_time
    }
}

pub struct FutexQueue {
    pub waiters: RwLock<usize>,
    pub chain: RwLock<VecDeque<FutexWaiter>>,
}

impl FutexQueue {
    pub fn new() -> Self {
        Self {
            waiters: RwLock::new(0),
            chain: RwLock::new(VecDeque::new()),
        }
    }
    pub fn waiters(&self) -> usize {
        *self.waiters.read()
    }
    pub fn waiters_increase(&self) {
        let mut waiters = self.waiters.write();
        *waiters += 1;
    }
    pub fn waiters_decrease(&self) {
        let mut waiters = self.waiters.write();
        *waiters -= 1;
    }
}
