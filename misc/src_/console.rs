use core::fmt::{self, Write};

use crate::sbi::legacy::console_putchar;

use spin::mutex::Mutex;

pub static STDOUT: Mutex<Stdout> = Mutex::new(Stdout);

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        s.chars().for_each(|ch| {
            console_putchar(ch as usize);
        });

        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    STDOUT.lock().write_fmt(args).unwrap();
}
