#[derive(Clone, Copy, Debug)]
pub struct RLimit {
    pub rlim_cur: usize, /* Soft limit */
    pub rlim_max: usize, /* Hard limit (ceiling for rlim_cur) */
}
impl RLimit {
    pub fn new(rlim_cur: usize, rlim_max: usize) -> Self {
        Self { rlim_cur, rlim_max }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Resource {
    CPU = 0,
    FSIZE = 1,
    DATA = 2,
    STACK = 3,
    CORE = 4,
    RSS = 5,
    NPROC = 6,
    NOFILE = 7,
    MEMLOCK = 8,
    AS = 9,
    LOCKS = 10,
    SIGPENDING = 11,
    MSGQUEUE = 12,
    NICE = 13,
    RTPRIO = 14,
    RTTIME = 15,
    NLIMITS = 16,
    ILLEAGAL = 100,
}
