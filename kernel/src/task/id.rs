use spin::Mutex;

static PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());

/// Process identifier allocator.
struct PidAllocator {
    current: usize,
}

/// Process Identifier
pub struct PidHandle(pub usize);

impl PidAllocator {
    pub const fn new() -> Self {
        Self { current: 0 }
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

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}
