use crate::task::TaskControlBlock;
use alloc::sync::Arc;

pub struct HangingTask {
    wake_up_time: usize, // 单位: ns
    inner: Arc<TaskControlBlock>,
}
impl PartialEq for HangingTask {
    fn eq(&self, other: &Self) -> bool {
        self.wake_up_time == other.wake_up_time
    }
}
impl Eq for HangingTask {}
impl PartialOrd for HangingTask {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        if self.wake_up_time > other.wake_up_time {
            Some(core::cmp::Ordering::Less)
        } else if self.wake_up_time < other.wake_up_time {
            Some(core::cmp::Ordering::Greater)
        } else {
            Some(core::cmp::Ordering::Equal)
        }
    }
}
impl Ord for HangingTask {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl HangingTask {
    pub const fn new(sleep_time: usize, duration: usize, task: Arc<TaskControlBlock>) -> Self {
        Self {
            wake_up_time: sleep_time + duration,
            inner: task,
        }
    }
    pub const fn wake_up_time(&self) -> usize {
        self.wake_up_time
    }
    pub fn inner(&self) -> Arc<TaskControlBlock> {
        self.inner.clone()
    }
}
