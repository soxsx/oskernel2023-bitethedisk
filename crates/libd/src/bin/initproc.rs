#![no_std]
#![no_main]
#![allow(unused)]

#[macro_use]
extern crate alloc;

use alloc::ffi::CString;
use libd::{
    heap, println,
    syscall::{exec, execve, exit, fork, sys_exec, waitpid},
};

// 根据是否开启 static-busybox 来看, 内核默认开启, 见 Kernal Cargo.toml 文件;
// initporc pid = 0;
// 开启 static-busybox 时: static-busybox pid = 1; busybox sh pid = 2;
// 未开启 static-busybox 时: busybox sh pid = 1;
// BUSYBOX_SH_PID 用于 exit 退出 shell
const BUSYBOX_SH_PID: isize = 2;

#[no_mangle]
fn main() -> isize {
    let pid = fork();
    if pid == 0 {
        execve(
            "./busybox\0".as_ptr() as *const i8,
            ["./busybox\0".as_ptr(), "sh\0".as_ptr()].as_ptr() as *const i8,
            ["PATH=/\0".as_ptr()].as_ptr() as *const i8,
        );
    } else {
        let mut exit_code = 0_i32;
        while waitpid(-1, &mut exit_code) != BUSYBOX_SH_PID {}
        println!("child proc exit_code: {}", exit_code);
    }
    0
}
