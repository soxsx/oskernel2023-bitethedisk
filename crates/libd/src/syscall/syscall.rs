#![allow(unused)]

use core::arch::asm;

const SYS_GETCWD: usize = 17;
const SYS_PIPE2: usize = 59;
const SYS_DUP: usize = 23;
const SYS_DUP3: usize = 24;
const SYS_CHDIR: usize = 49;
const SYS_OPENAT: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_GETDENTS64: usize = 61;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_LINKAT: usize = 37;
const SYS_UNLINKAT: usize = 35;
const SYS_MKDIRAT: usize = 34;
const SYS_UMOUNT2: usize = 39;
const SYS_MOUNT: usize = 40;
const SYS_FSTAT: usize = 80;
const SYS_CLONE: usize = 220;
const SYS_EXECVE: usize = 221;
const SYS_WAIT4: usize = 260;
const SYS_EXIT: usize = 93;
const SYS_GETPPID: usize = 173;
const SYS_GETPID: usize = 172;
const SYS_BRK: usize = 214;
const SYS_MUNMAP: usize = 215;
const SYS_MMAP: usize = 222;
const SYS_TIMES: usize = 153;
const SYS_UNAME: usize = 160;
const SYS_SCHED_YIELD: usize = 124;
const SYS_GETTIMEOFDAY: usize = 169;
const SYS_NANOSLEEP: usize = 101;

#[inline(always)]
pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

pub fn sys_fork() -> isize {
    syscall(SYS_CLONE, [0, 0, 0])
}

pub fn sys_exec(path: *const i8, argv: *const i8, envp: *const i8) -> isize {
    syscall(SYS_EXECVE, [path as usize, argv as usize, envp as usize])
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    syscall(SYS_WAIT4, [pid as usize, exit_code_ptr as usize, 0])
}

pub fn sys_exit(exit_code: isize) -> ! {
    syscall(SYS_EXIT, [exit_code as usize, 0, 0]);
    unreachable!("should not reach here(after sys_exit)!");
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    syscall(SYS_WRITE, [fd, buf as usize, len])
}

pub fn sys_sched_yield() {
    syscall(SYS_SCHED_YIELD, [0, 0, 0]);
}
