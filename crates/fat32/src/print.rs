//! used for no_std environment of riscv64imac-unknown-none-elf

use core::{
    arch::asm,
    fmt::{self, Write},
};

#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x17") which,
        );
    }
    ret
}

const SBI_CONSOLE_PUTCHAR: usize = 1;

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fmt::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fmt::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fmt::print(format_args!("\x1b[91m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}

#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fmt::print(format_args!("\x1b[92m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}

#[macro_export]
macro_rules! warn {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fmt::print(format_args!("\x1b[93m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}
#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fmt::print(format_args!("\x1b[94m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}
