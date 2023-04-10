use alloc::vec::Vec;
use lazy_static::*;
use spin::Mutex;

/// ### 栈式进程标识符分配器
/// |成员变量|描述|
/// |--|--|
/// |`current`|当前可用的最小PID|
/// |`recycled`|以栈的形式存放着已经回收的PID|
/// ```
/// PidAllocator::new() -> Self
/// PidAllocator::alloc(&mut self) -> PidHandle
/// PidAllocator::dealloc(&mut self, pid: usize)
/// ```
struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
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
        // HINT: 可以用 HashSet
        assert!(
            !self.recycled.iter().any(|ppid| *ppid == pid),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

/// 进程标识符
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PidHandle(pub usize);

// 为 PidHandle 实现 Drop Trait 来允许编译器进行自动的资源回收
impl Drop for PidHandle {
    fn drop(&mut self) {
        // TODO 这里可能导致在从 pid map 里面清除对应 pcb 的时候多次释放，被 assert panic 了
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}

/// 从全局栈式进程标识符分配器 `PID_ALLOCATOR` 分配一个进程标识符
#[inline]
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}
