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

#[no_mangle]
fn _start() -> ! {
    exit(main())
}

#[no_mangle]
fn main() -> isize {
    let pid = fork();
    if pid == 0 {
        execve(
            "./busybox\0".as_ptr() as *const i8,
            ["./busybox\0".as_ptr(), "sh\0".as_ptr()].as_ptr() as *const i8,
            ["\0".as_ptr()].as_ptr() as *const i8,
        );
    } else {
        let mut exit_code = 0_i32;
        let res = waitpid(pid as usize, &mut exit_code);
        println!("child proc exit_code: {}", exit_code);
        println!("waitpid result: {}", res);
    }
    0
}
