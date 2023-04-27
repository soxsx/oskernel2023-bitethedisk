//! 根据 SYS_id 分发具体系统调用

use super::syscall_id::*;

use super::impls::*;

/// 系统调用分发函数
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYS_FORK => sys_fork(args[0], args[1], args[2], args[3], args[4]),
        // TODO: here
        // SYS_CLONE => sys_clone(args[0], args[1], args[2], args[3], args[4]),

        SYS_EXEC => sys_exec(
            args[0] as *const u8,
            args[1] as *const usize,
            args[2] as *const usize,
        ),
        // TODO: here
        // SYS_EXECVE => sys_execve(
        //     args[0] as *const u8,
        //     args[1] as *const u8,
        //     args[2] as *const u8,
        // ),

        SYS_LINKAT => sys_linkat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as isize,
            args[3] as *const u8,
            args[4] as u32,
        ),

        SYS_OPENAT => sys_openat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as u32,
            args[3] as u32,
        ),

        SYS_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SYS_DUP => sys_dup(args[0]),
        SYS_DUP3 => sys_dup3(args[0], args[1]),
        SYS_MKDIRAT => sys_mkdirat(args[0] as isize, args[1] as *const u8, args[2] as u32),
        SYS_UNLINKAT => sys_unlinkat(args[0] as isize, args[1] as *const u8, args[2] as u32),
        SYS_UMOUNT2 => sys_umount2(args[0] as *const u8, args[1]),
        SYS_MOUNT => sys_mount(
            args[0] as *const u8,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3],
            args[4] as *const u8,
        ),
        SYS_CHDIR => sys_chdir(args[0] as *const u8),
        SYS_CLOSE => sys_close(args[0]),
        SYS_PIPE2 => sys_pipe2(args[0] as *mut i32, args[1] as usize),
        SYS_GETDENTS64 => sys_getdents64(args[0] as isize, args[1] as *mut u8, args[2]),
        SYS_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_FSTAT => sys_fstat(args[0] as isize, args[1] as *mut u8),
        SYS_EXIT => sys_exit(args[0] as i32),
        SYS_NANOSLEEP => sys_nanosleep(args[0] as *const u8),
        SYS_SCHED_YIELD => sys_sched_yield(),
        SYS_TIMES => sys_times(args[0] as *const u8),
        SYS_UNAME => sys_uname(args[0] as *const u8),
        SYS_GETTIMEOFDAY => sys_gettimeofday(args[0] as *const u8),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_BRK => sys_brk(args[0]),
        SYS_MMAP => sys_mmap(
            args[0],
            args[1],
            args[2],
            args[3],
            args[4] as isize,
            args[5],
        ),
        SYS_MUNMAP => sys_munmap(args[0], args[1]),
        SYS_WAIT4 => sys_wait4(args[0] as isize, args[1] as *mut i32),

        _ => panic!("unsupported syscall, syscall id: {:?}", syscall_id),
    }
}
