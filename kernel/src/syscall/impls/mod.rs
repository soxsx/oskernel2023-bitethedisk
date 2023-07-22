#![allow(unused)]

pub mod fs;
pub mod mm;
pub mod others;
pub mod process;

pub use fs::*;
pub use mm::*;
pub use others::*;
pub use process::*;

pub use super::errno::*;

#[macro_export]
macro_rules! return_errno {
    ($errno:expr $(, $fmt:literal $(, $($arg: tt)+)?)?) => {
        let time = crate::timer::get_time();
        warn!("[{:>8}] {}:{} syscall error: {}", time, file!(), line!(), $errno);
        $(warn!(concat!("[{:>8}] error info: ", $fmt), time $(, $($arg)+)?);)?
        return Err($errno);
    };
}
