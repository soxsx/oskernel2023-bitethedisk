use spin::Mutex;

/// # 控制台模块
/// `os/src/main.rs`
/// ## 功能
/// - 提供基于Stdout结构体的标准输出宏
/// ```
/// pub fn print(args: fmt::Arguments)
/// macro_rules! print
/// macro_rules! println
/// ```
///

use crate::sbi::console_putchar;
use core::fmt::{self, Write};

pub struct Stdout; //类单元结构体，用于格式化输出

pub static STDOUT: Mutex<Stdout> = Mutex::new(Stdout);

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

/// 采用Stdout结构体的方式向终端输出
pub fn print(args: fmt::Arguments) {
    STDOUT.lock().write_fmt(args);
}
