#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use libd::{
    println,
    syscall::{execve, fork, wait},
};

#[no_mangle]
fn main(_argc: usize, argv: &[&str]) -> i32 {
    println!("[initproc]: Hello, world!");
    let pid = fork();
    if pid == 0 {
        execve("./busybox\0", vec!["./busybox\0", "sh\0"].as_slice(), argv);
    } else {
        let mut exit_code = 0;
        wait(pid as usize, &mut exit_code);
    }
    0
}
