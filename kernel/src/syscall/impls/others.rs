use nix::info::Utsname;
use nix::{tms, TimeVal};

use crate::task::hanging_current_and_run_next;
use crate::{
    mm::{translated_bytes_buffer, translated_ref, UserBuffer},
    task::{current_user_token, suspend_current_and_run_next},
    timer::{get_time_ms, get_timeval},
};

use super::*;

/// #define SYS_times 153
///
/// 功能：获取进程时间；
///
/// 输入：tms结构体指针，用于获取保存当前进程的运行时间数据；
///
/// 返回值：成功返回已经过去的滴答数，失败返回-1;
///
/// ```c
/// struct tms *tms;
/// clock_t ret = syscall(SYS_times, tms);
/// ```
pub fn sys_times(buf: *const u8) -> Result<isize> {
    let sec = get_time_ms() as isize * 1000;
    let token = current_user_token();
    let buffers = translated_bytes_buffer(token, buf, core::mem::size_of::<tms>());
    let mut userbuf = UserBuffer::wrap(buffers);
    // TODO tms rusage
    userbuf.write(
        tms {
            tms_stime: sec,
            tms_utime: sec,
            tms_cstime: sec,
            tms_cutime: sec,
        }
        .as_bytes(),
    );
    Ok(0)
}

// TODO 2 tms 没有处理

/// struct utsname {
/// 	char sysname\[65\];
/// 	char nodename\[65\];
/// 	char release\[65\];
/// 	char version\[65\];
/// 	char machine\[65\];
/// 	char domainname\[65\];
/// };
///
/// #define SYS_uname 160
///
/// 功能：打印系统信息；
///
/// 输入：utsname结构体指针用于获得系统信息数据；
///
/// 返回值：成功返回0，失败返回-1;
///
/// ```c
/// struct utsname *uts;
/// int ret = syscall(SYS_uname, uts);
/// ```
pub fn sys_uname(buf: *const u8) -> Result<isize> {
    let token = current_user_token();
    let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(
        token,
        buf,
        core::mem::size_of::<Utsname>(),
    ));
    userbuf.write(Utsname::get().as_bytes());
    Ok(0)
}

/// 应用主动交出 CPU 所有权进入 Ready 状态并切换到其他应用
///
/// - 返回值：总是返回 0。
/// - syscall ID：124
pub fn sys_sched_yield() -> Result<isize> {
    suspend_current_and_run_next();
    Ok(0)
}

// TODO 1 规范里面写的 timeval

/// ```c
/// struct timespec {
/// 	time_t tv_sec;        /* 秒 */
/// 	long   tv_nsec;       /* 纳秒, 范围在0~999999999 */
/// };
///
/// ```
///
/// #define SYS_gettimeofday 169
///
/// 功能：获取时间；
///
/// 输入： timespec结构体指针用于获得时间值；
///
/// 返回值：成功返回0，失败返回-1;
///
/// ```c
/// struct timespec *ts;
/// int ret = syscall(SYS_gettimeofday, ts, 0);
/// ```
pub fn sys_gettimeofday(buf: *const u8) -> Result<isize> {
    let token = current_user_token();
    let buffers = translated_bytes_buffer(token, buf, core::mem::size_of::<TimeVal>());
    let mut userbuf = UserBuffer::wrap(buffers);
    userbuf.write(get_timeval().as_bytes());
    Ok(0)
}

/// #define SYS_nanosleep 101
///
/// 功能：执行线程睡眠，sleep()库函数基于此系统调用；
///
/// 输入：睡眠的时间间隔；
///
/// 返回值：成功返回0，失败返回-1;
///
/// ```c
/// const struct timespec *req, struct timespec *rem;
/// int ret = syscall(SYS_nanosleep, req, rem);
/// ```
pub fn sys_nanosleep(buf: *const u8) -> Result<isize> {
    let tic = get_time_ms();
    let token = current_user_token();
    let len_timeval = translated_ref(token, buf as *const TimeVal);
    let len = len_timeval.sec * 1000 + len_timeval.usec / 1000;
    hanging_current_and_run_next(tic, len);
    Ok(0)
}

pub fn sys_getrandom(buf: *const u8, buf_size: usize, flags: usize) -> Result<isize> {
    Ok(buf_size as isize)
}
