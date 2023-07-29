//! 提供给用户的系统调用封装

use super::syscall::*;

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: *const i8) -> isize {
    sys_exec(path, 0 as *const i8, 0 as *const i8)
}

pub fn execve(path: *const i8, argv: *const i8, envp: *const i8) -> isize {
    sys_exec(path as *const i8, argv, envp)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf.as_ptr(), buf.len())
}

pub fn exit(exit_code: isize) -> ! {
    sys_exit(exit_code);
}

pub fn yield_() {
    sys_sched_yield()
}

pub fn waitpid(pid: isize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid, exit_code)
}
