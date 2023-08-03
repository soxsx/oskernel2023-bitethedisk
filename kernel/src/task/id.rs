//! 进程标识符
//!
//! 在多核环境下, 每个 [`Cpu`] 都缓存了一定数量的 PID, 以减少进程创建获取 PID 时
//! 频繁加锁所带来的性能损耗
//!
//! [`Cpu`]: crate::task::processor::cpu::Cpu

use sync_cell::SyncRefCell;

static PID_ALLOCATOR: SyncRefCell<PidAllocator> = SyncRefCell::new(PidAllocator::new());

/// 栈式进程标识符分配器
struct PidAllocator {
    current: usize,
}

/// 进程标识符
pub struct PidHandle(pub usize);

impl PidAllocator {
    /// 返回一个初始化好的进程标识符分配器
    pub const fn new() -> Self {
        PidAllocator { current: 0 }
    }

    fn fetch_add(&mut self) -> usize {
        let new_pid = self.current;
        self.current += 1;
        new_pid
    }

    /// 分配一个进程标识符
    pub fn alloc(&mut self) -> PidHandle {
        PidHandle(self.fetch_add())
    }
}

/// 从全局栈式进程标识符分配器 `PID_ALLOCATOR` 分配一个进程标识符
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.borrow_mut().alloc()
}
