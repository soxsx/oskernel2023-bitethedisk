//! Process Identifier

use sync_cell::SyncRefCell;

static PID_ALLOCATOR: SyncRefCell<PidPool> = SyncRefCell::new(PidPool::new());

/// Stack-based Process Identifier Allocator
struct PidAllocator {
    current: usize,
}

/// Process Identifier
pub struct PidHandle(pub usize);

impl PidAllocator {
    pub const fn new() -> Self {
        PidPool { current: 0 }
    }
    fn fetch_add(&mut self) -> usize {
        let new_pid = self.current;
        self.current += 1;
        new_pid
    }
    pub fn alloc(&mut self) -> PidHandle {
        PidHandle(self.fetch_add())
    }
}

/// Allocate a process identifier from the global stack process identifier allocator [`PID_ALLOCATOR`]
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.borrow_mut().alloc()
}
