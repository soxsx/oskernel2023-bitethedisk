use core::ops::{Add, Sub};

pub const TICKS_PER_SEC: usize = 100;
pub const MSEC_PER_SEC: usize = 1000;
pub const USEC_PER_SEC: usize = 1000_000;
pub const NSEC_PER_SEC: usize = 1000_000_000;

/// Linux 时间格式
///
/// - `sec`：秒
/// - `usec`：微秒
/// - 两个值相加的结果是结构体表示的时间
///
/// # Note
///
/// [PartialOrd] 使用了默认实现，所以当前结构体的字段顺序不可改变
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeVal {
    pub sec: usize,  // 秒
    pub usec: usize, // 微秒
}

impl Add for TimeVal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut sec = self.sec + other.sec;
        let mut usec = self.usec + other.usec;
        sec += usec / USEC_PER_SEC;
        usec %= USEC_PER_SEC;
        Self { sec, usec }
    }
}

impl Sub for TimeVal {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if self.sec < other.sec {
            return Self { sec: 0, usec: 0 };
        } else if self.sec == other.sec {
            if self.usec < other.usec {
                return Self { sec: 0, usec: 0 };
            } else {
                return Self {
                    sec: 0,
                    usec: self.usec - other.usec,
                };
            }
        } else {
            let mut sec = self.sec - other.sec;
            let usec = if self.usec < other.usec {
                sec -= 1;
                USEC_PER_SEC + self.usec - other.usec
            } else {
                self.usec - other.usec
            };
            Self { sec, usec }
        }
    }
}

impl TimeVal {
    pub fn new() -> Self {
        Self { sec: 0, usec: 0 }
    }

    #[inline]
    pub fn zero() -> Self {
        Self { sec: 0, usec: 0 }
    }

    pub fn is_zero(&self) -> bool {
        self.sec == 0 && self.usec == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }

    pub fn from_ticks(tiks: usize) -> Self {
        let sec = tiks / TICKS_PER_SEC;
        let usec = (tiks % TICKS_PER_SEC) * USEC_PER_SEC / TICKS_PER_SEC;
        Self { sec, usec }
    }

    pub fn into_ticks(&self) -> usize {
        self.sec * TICKS_PER_SEC + self.usec / USEC_PER_SEC * TICKS_PER_SEC
    }
}

/// Linux 间隔计数
///
/// - `tms_utime`：用户态时间
/// - `tms_stime`：内核态时间
/// - `tms_cutime`：已回收子进程的用户态时间
/// - `tms_cstime`：已回收子进程的内核态时间
#[allow(non_camel_case_types)]
pub struct tms {
    /// 用户态时间
    pub tms_utime: isize,
    /// 内核态时间
    pub tms_stime: isize,
    /// 已回收子进程的用户态时间
    pub tms_cutime: isize,
    /// 已回收子进程的内核态时间
    pub tms_cstime: isize,
}

impl tms {
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

pub struct TimeSpec {
    pub tv_sec: u64,  // 秒
    pub tv_nsec: u64, // 纳秒
}
impl TimeSpec {
    pub fn new() -> Self {
        Self {
            tv_sec: 0,
            tv_nsec: 0,
        }
    }

    pub fn into_ticks(&self) -> usize {
        self.tv_sec as usize * TICKS_PER_SEC + self.tv_nsec as usize / NSEC_PER_SEC * TICKS_PER_SEC
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

/// https://github.com/torvalds/linux/blob/ffabf7c731765da3dbfaffa4ed58b51ae9c2e650/include/uapi/linux/time.h#L42-L44
pub enum IntervalTimerType {
    Real = 0,
    Virtual = 1,
    Profile = 2,
}

pub const ITIMER_REAL: i32 = 0;
pub const ITIMER_VIRTUAL: i32 = 1;
pub const ITIMER_PROF: i32 = 2;

impl TryFrom<i32> for IntervalTimerType {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            ITIMER_REAL => Ok(IntervalTimerType::Real),
            ITIMER_VIRTUAL => Ok(IntervalTimerType::Virtual),
            ITIMER_PROF => Ok(IntervalTimerType::Profile),
            _ => Err(()),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub struct itimerval {
    pub it_interval: TimeVal,
    pub it_value: TimeVal,
}

impl itimerval {
    pub fn empty() -> Self {
        Self {
            it_interval: TimeVal::zero(),
            it_value: TimeVal::zero(),
        }
    }
}
