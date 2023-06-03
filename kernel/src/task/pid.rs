//! 进程标识符

use alloc::vec::Vec;
use spin::Mutex;

use super::processor::PROCESSOR;

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

/// 栈式进程标识符分配器
struct PidAllocator {
    /// 当前可用的最小PID
    current: usize,

    /// 已回收的 PID
    recycled: Vec<usize>,
}

/// 进程标识符
pub struct PidHandle(pub usize);

impl PidAllocator {
    /// 返回一个初始化好的进程标识符分配器
    pub const fn new() -> Self {
        PidAllocator {
            current: 0,
            recycled: vec![],
        }
    }

    /// 分配一个进程标识符
    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    /// 释放一个进程标识符
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            !self.recycled.iter().any(|ppid| *ppid == pid),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}

/// 从全局栈式进程标识符分配器 `PID_ALLOCATOR` 分配一个进程标识符
pub fn pid_alloc() -> PidHandle {
    #[cfg(feature = "multi_harts")]
    {
        let mut pid_pool = &mut PROCESSORS[hartid!()].pid_pool;
        if Some(pid_handle) = pid_pool.pop() {
            pid_handle
        } else {
            let mut pid_allocator = PID_ALLOCATOR.lock();
            (0..512).for_each(|| pid_pool.push(pid_allocator.alloc()));
            pid_pool.pop()
        }
    }

    #[cfg(not(feature = "multi_harts"))]
    {
        PID_ALLOCATOR.lock().alloc()
    }
}
