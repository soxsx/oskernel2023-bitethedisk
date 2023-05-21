//! 控制台输出输出

use core::fmt::{self, Write};

use crate::sbi::console_putchar;

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments<'_>) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

// 红色 91
#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[91m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}

// 绿色 92
#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[92m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}

// 黄色 93
#[macro_export]
macro_rules! warn {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[93m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}

// 蓝色 94
#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[94m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}
