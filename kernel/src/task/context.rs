use core::fmt::Debug;

use crate::trap::trap_return;

/// 任务上下文
///
/// - `s`: s\[0\]~s\[11\]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    /// 获取一个空的 TaskContext
    pub const fn empty() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// 当每个应用第一次获得 CPU 使用权即将进入用户态执行的时候，它的内核栈顶放置着我们在
    /// 内核加载应用的时候构造的一个任务上下文,在 `__switch` 切换到该应用的任务上下文的时候，
    /// 内核将会跳转到 `trap_return` 并返回用户态开始该应用的启动执行
    pub fn readied_for_switching(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}

impl Debug for TaskContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaskContext")
            .field("ra", &self.ra)
            .field("sp", &self.sp)
            .field("s0", &self.s[0])
            .field("s1", &self.s[1])
            .field("s2", &self.s[2])
            .field("s3", &self.s[3])
            .field("s4", &self.s[4])
            .field("s5", &self.s[5])
            .field("s6", &self.s[6])
            .field("s7", &self.s[7])
            .field("s8", &self.s[8])
            .field("s9", &self.s[9])
            .field("s10", &self.s[10])
            .field("s11", &self.s[11])
            .finish()
    }
}
