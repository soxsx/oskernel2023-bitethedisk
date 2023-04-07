//! # 控制台模块
//! `os/src/main.rs`
//! ## 功能
//! - 提供基于Stdout结构体的标准输出宏
//! ```
//! pub fn print(args: fmt::Arguments)
//! macro_rules! print
//! macro_rules! println
//! ```
//!
use crate::sbi::console_putchar;
use core::fmt::{self, Write};
use spin::Mutex;

pub struct Stdout;

pub static STDOUT: Mutex<Stdout> = Mutex::new(Stdout);

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

/// 向终端输出，fmt::Arguments 是一个编译期处理的类型
pub fn print(args: fmt::Arguments) {
    STDOUT.lock().write_fmt(args).unwrap();
}
