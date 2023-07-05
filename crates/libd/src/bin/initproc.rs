#![no_std]
#![no_main]

use libd::syscall::{exec, fork, wait};

static TESTS: [&str; 33] = [
    "brk\0",          // 1
    "chdir\0",        // 2
    "clone\0",        // 3
    "close\0",        // 4
    "dup\0",          // 5
    "dup2\0",         // 6
    "execve\0",       // 7
    "exit\0",         // 8
    "fork\0",         // 9
    "fstat\0",        // 10
    "getcwd\0",       // 11
    "getdents\0",     // 12
    "getpid\0",       // 13
    "getppid\0",      // 14
    "gettimeofday\0", // 15
    "mkdir_\0",       // 16
    "mmap\0",         // 17
    "mount\0",        // 18
    "munmap\0",       // 19
    "open\0",         // 20
    "openat\0",       // 21
    "pipe\0",         // 22
    "read\0",         // 23
    "sleep\0",        // 24
    "test_echo\0",    // 25
    "times\0",        // 26
    "umount\0",       // 27
    "uname\0",        // 28
    "unlink\0",       // 29
    "wait\0",         // 30
    "waitpid\0",      // 31
    "write\0",        // 32
    "yield\0",        // 33
];

#[no_mangle]
fn main() -> i32 {
    for i in 0..TESTS.len() {
        let pid = fork();
        if pid == 0 {
            exec(TESTS[i]);
        } else {
            let mut exit_code = 0;
            wait(pid as usize, &mut exit_code);
        }
    }
    0
}
