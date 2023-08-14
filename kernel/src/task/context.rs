use core::fmt::Debug;

use crate::trap::trap_return;

#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}
impl TaskContext {
    pub const fn empty() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    /// When each application gains CPU ownership for the first time and
    /// is about to enter user mode execution, its kernel stack top holds a task context
    /// constructed when we load the application in the kernel.
    /// When __switch switches to the task context of the application,
    /// the kernel will jump to `trap_return` and return to user mode to start executing the application.
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
