//! 系统时间相关模块

#![allow(unused)]

use crate::{consts::CLOCK_FREQ, sbi::set_timer};
use core::ops::{Add, Sub};
use nix::TimeVal;
use riscv::register::time;

pub const TICKS_PER_SEC: usize = 100;
pub const MSEC_PER_SEC: usize = 1000;
pub const USEC_PER_SEC: usize = 1000_000;
pub const NSEC_PER_SEC: usize = 1000_000_000;

/// 取得当前 `mtime` 计数器的值
///
/// - `mtime`: 统计处理器自上电以来经过了多少个内置时钟的时钟周期,64bit
pub fn get_time() -> usize {
    time::read()
}

/// 获取CPU上电时间（单位：ms）
pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub fn get_time_ns() -> usize {
    (get_time() / (CLOCK_FREQ / USEC_PER_SEC)) * MSEC_PER_SEC
}

pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / USEC_PER_SEC)
}

pub fn get_time_s() -> usize {
    get_time() / CLOCK_FREQ
}

/// 获取 `TimeVal` 格式的时间信息
pub fn get_timeval() -> TimeVal {
    let ticks = get_time();
    let sec = ticks / CLOCK_FREQ;
    let usec = (ticks % CLOCK_FREQ) * USEC_PER_SEC / CLOCK_FREQ;
    TimeVal { sec, usec }
}

/// 设置下次触发时钟中断的时间
pub fn set_next_trigger() {
    set_timer((get_time() + CLOCK_FREQ / TICKS_PER_SEC) as u64);
}
