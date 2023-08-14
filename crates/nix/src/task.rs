#[derive(Clone, Copy, Debug)]
pub struct AuxEntry(pub usize, pub usize);

// ELF Auxiliary Vectors
// https://articles.manugarg.com/aboutelfauxiliaryvectors.html
// pub const AT_NULL: usize = 0; // end of vector
// pub const AT_IGNORE: usize = 1; // entry should be ignored
// pub const AT_EXECFD: usize = 2; // file descriptor of program
// pub const AT_NOTELF: usize = 10; // program is not ELF
// pub const AT_PLATFORM: usize = 15; // string identifying CPU for optimizations
// pub const AT_BASE_PLATFORM: usize = 24; // string identifying real platform, may differ from AT_PLATFORM.
// pub const AT_HWCAP2: usize = 26; // extension of AT_HWCAP
// pub const AT_EXECFN: usize = 31; // filename of program
pub const AT_PHDR: usize = 3; // program headers for program
pub const AT_PHENT: usize = 4; // size of program header entry
pub const AT_PHNUM: usize = 5; // number of program headers
pub const AT_PAGESZ: usize = 6; // system page size
pub const AT_BASE: usize = 7; // base address of interpreter
pub const AT_FLAGS: usize = 8; // flags
pub const AT_ENTRY: usize = 9; // entry point of program
pub const AT_UID: usize = 11; // real uid
pub const AT_EUID: usize = 12; // effective uid
pub const AT_GID: usize = 13; // real gid
pub const AT_EGID: usize = 14; // effective gid
pub const AT_HWCAP: usize = 16; // arch dependent hints at CPU capabilities
pub const AT_CLKTCK: usize = 17; // frequency at which times() increments
pub const AT_SECURE: usize = 23; // secure mode boolean
pub const AT_RANDOM: usize = 25; // address of 16 random bytes

pub const RUSAGE_SELF: isize = 0;

pub const SCHED_OTHER: isize = 0;
pub const SCHED_FIFO: isize = 1;
pub const SCHED_RR: isize = 2;
pub const SCHED_BATCH: isize = 3;
pub const SCHED_IDLE: isize = 5;
pub const SCHED_DEADLINE: isize = 6;

#[repr(C)]
pub struct SchedParam {
    sched_priority: isize,
}
impl SchedParam {
    pub fn new() -> Self {
        Self { sched_priority: 0 }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(&self.sched_priority as *const isize as *const u8, 8) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(&mut self.sched_priority as *mut isize as *mut u8, 8)
        }
    }
    pub fn set_priority(&mut self, priority: isize) {
        self.sched_priority = priority;
    }
    pub fn get_priority(&self) -> isize {
        self.sched_priority
    }
}

#[repr(C)]
pub struct SchedPolicy(isize);

pub struct CpuMask {
    mask: [u8; 1024 / (8 * core::mem::size_of::<u8>())],
}
impl CpuMask {
    pub fn new() -> Self {
        Self {
            mask: [0; 1024 / (8 * core::mem::size_of::<u8>())],
        }
    }
    pub fn set(&mut self, cpu: usize) {
        let index = cpu / (8 * core::mem::size_of::<u8>());
        let offset = cpu % (8 * core::mem::size_of::<u8>());
        self.mask[index] |= 1 << offset;
    }
    pub fn get(&self, cpu: usize) -> bool {
        let index = cpu / (8 * core::mem::size_of::<u8>());
        let offset = cpu % (8 * core::mem::size_of::<u8>());
        self.mask[index] & (1 << offset) != 0
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.mask
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.mask
    }
}

#[repr(C)]
pub struct CpuSet {
    mask: [usize; 1024 / (8 * core::mem::size_of::<usize>())],
}

impl CpuSet {
    pub fn new() -> Self {
        Self {
            mask: [0; 1024 / (8 * core::mem::size_of::<usize>())],
        }
    }
    pub fn set(&mut self, cpu: usize) {
        let index = cpu / (8 * core::mem::size_of::<usize>());
        let offset = cpu % (8 * core::mem::size_of::<usize>());
        self.mask[index] |= 1 << offset;
    }
    pub fn get(&self, cpu: usize) -> bool {
        let index = cpu / (8 * core::mem::size_of::<usize>());
        let offset = cpu % (8 * core::mem::size_of::<usize>());
        self.mask[index] & (1 << offset) != 0
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}
