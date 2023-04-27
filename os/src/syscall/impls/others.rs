use crate::timer::TimeVal;
use crate::{
    mm::{translated_bytes_buffer, translated_ref, UserBuffer},
    task::{current_user_token, suspend_current_and_run_next},
    timer::{get_time_ms, get_timeval, tms},
};

use super::{Utsname, UTSNAME};

pub fn sys_times(buf: *const u8) -> isize {
    let sec = get_time_ms() as isize * 1000;
    let token = current_user_token();
    let buffers = translated_bytes_buffer(token, buf, core::mem::size_of::<tms>());
    let mut userbuf = UserBuffer::wrap(buffers);
    userbuf.write(
        tms {
            tms_stime: sec,
            tms_utime: sec,
            tms_cstime: sec,
            tms_cutime: sec,
        }
        .as_bytes(),
    );
    0
}

/// ### 获取系统utsname参数
/// - 参数
///     - `buf`：用户空间存放utsname结构体的缓冲区
/// - 返回值
///     - 0表示正常
/// - syscall_ID: 160
pub fn sys_uname(buf: *const u8) -> isize {
    let token = current_user_token();
    let uname = UTSNAME.lock();
    let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(
        token,
        buf,
        core::mem::size_of::<Utsname>(),
    ));
    userbuf.write(uname.as_bytes());
    0
}

/// ### 应用主动交出 CPU 所有权进入 Ready 状态并切换到其他应用
/// - 返回值：总是返回 0。
/// - syscall ID：124
pub fn sys_sched_yield() -> isize {
    suspend_current_and_run_next();

    0
}

/// ### 获取CPU上电时间 秒+微秒
/// syscall_id：169
/// - 输入参数
///     - `ts`：`TimeVal` 结构体在用户空间的地址
///     - `tz`：表示时区，这里无需考虑，始终为0
/// - 功能：内核根据时钟周期数和时钟频率换算系统运行时间，并写入到用户地址空间
/// - 返回值：正确执行返回 0，出现错误返回 -1。
pub fn sys_gettimeofday(buf: *const u8) -> isize {
    let token = current_user_token();
    let buffers = translated_bytes_buffer(token, buf, core::mem::size_of::<TimeVal>());
    let mut userbuf = UserBuffer::wrap(buffers);
    userbuf.write(get_timeval().as_bytes());

    0
}

/// ### sleep 给定时长（TimeVal格式）
/// - 返回值：总是返回 0。
/// - syscall ID：101
pub fn sys_nanosleep(buf: *const u8) -> isize {
    let tic = get_time_ms();

    let token = current_user_token();
    let len_timeval = translated_ref(token, buf as *const TimeVal);
    let len = len_timeval.sec * 1000 + len_timeval.usec / 1000;
    loop {
        let toc = get_time_ms();
        if toc - tic >= len {
            break;
        }
    }

    0
}
