//! 进程标识符
//!
//! 在多核环境下，每个 [`Cpu`] 都缓存了一定数量的 PID，以减少进程创建获取 PID 时
//! 频繁加锁所带来的性能损耗
//!
//! [`Cpu`]: crate::task::processor::cpu::Cpu

use alloc::vec::Vec;

use crate::cell::sync_cell::SyncRefCell;

static PID_ALLOCATOR: SyncRefCell<PidAllocator> = SyncRefCell::new(PidAllocator::new());

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
        PID_ALLOCATOR.borrow_mut().dealloc(self.0);
    }
}

/// 从全局栈式进程标识符分配器 `PID_ALLOCATOR` 分配一个进程标识符
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.borrow_mut().alloc()
}
