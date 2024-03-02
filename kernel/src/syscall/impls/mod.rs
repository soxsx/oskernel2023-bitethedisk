pub mod fs;
pub mod futex;
pub mod gui;
pub mod mm;
pub mod others;
pub mod process;

pub use fs::*;
pub use futex::*;
pub use gui::*;
pub use mm::*;
pub use others::*;
pub use process::*;

pub use super::errno::*;

#[macro_export]
macro_rules! return_errno {
    ($errno:expr $(, $fmt:literal $(, $($arg: tt)+)?)?) => {{
        #[cfg(feature = "dev")]
        {
            let time = crate::timer::get_time();
            println!("\x1B[93m[{:>16}] {}:{} Errno: {}\x1B[0m", time, file!(), line!(), $errno);
            $(
                println!(concat!("\x1B[32m[{:>16}] Reason: ", $fmt, "\n\x1B[0m"), time $(, $($arg)+)?);
            )?
        }
        return Err($errno);
    }};
}
