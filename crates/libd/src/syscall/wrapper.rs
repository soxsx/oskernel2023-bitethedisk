//! 提供给用户的系统调用封装

use alloc::vec::Vec;

use super::syscall::*;

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: &str) -> isize {
    sys_exec(path.as_ptr() as *const u8, 0 as *const u8, 0 as *const u8)
}

pub fn execve(path: &str, argv: &[&str], envp: &[&str]) -> isize {
    let argv = to_cstr(argv);
    let envp = to_cstr(envp);
    sys_exec(path.as_ptr(), argv, envp)
}

fn to_cstr(rs_str: &[&str]) -> *const u8 {
    let mut cstr_heads = Vec::new();
    for &cstr in rs_str {
        cstr_heads.push(cstr.as_ptr());
    }
    cstr_heads.as_slice() as *const _ as *const _
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
