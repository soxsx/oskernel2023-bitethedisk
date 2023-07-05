//! 提供给用户的系统调用封装

use super::syscall::*;

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: &str /* , argv: &[&str], envp: &[&str] */) -> isize {
    sys_exec(path.as_ptr() as *const u8, 0 as *const u8, 0 as *const u8)
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

pub fn wait(pid: usize, exit_code: &mut usize) -> isize {
    sys_waitpid(pid, exit_code)
}
