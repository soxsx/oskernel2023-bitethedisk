#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use libd::syscall::{execve, fork, wait};

#[no_mangle]
fn main() -> i32 {
    let pid = fork();
    if pid == 0 {
        execve(
            "./busybox\0",
            vec!["./busybox\0", "sh\0", "\0"].as_slice(),
            vec!["\0"].as_slice(),
        );
    } else {
        let mut exit_code = 0;
        wait(pid as usize, &mut exit_code);
    }
    0
}
