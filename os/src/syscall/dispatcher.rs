//!
//! 根据 SYS_id 分发具体系统调用
//!

use super::ids::*;

use super::fs::*;
use super::process::*;
// use super::sigset::*;

/// 系统调用分发函数
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYS_FORK => sys_fork(args[0], args[1], args[2], args[3], args[4]),
        // TODO: here
        SYS_CLONE => sys_clone(args[0], args[1], args[2], args[3], args[4]),

        SYS_EXEC => sys_exec(
            args[0] as *const u8,
            args[1] as *const usize,
            args[2] as *const usize,
        ),
        // TODO: here
        SYS_EXECVE => sys_execve(
            args[0] as *const u8,
            args[1] as *const u8,
            args[2] as *const u8,
        ),

        SYS_LINKAT => sys_linkat(
            args[0] as isize,
            args[0] as *const u8,
            args[0] as isize,
            args[0],
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
        SYS_FCNTL => sys_fcntl(args[0] as isize, args[1], Option::<usize>::from(args[2])),
        SYS_IOCTL => sys_ioctl(args[0], args[1], args[2] as *mut u8),
        SYS_MKDIRAT => sys_mkdirat(args[0] as isize, args[1] as *const u8, args[2] as u32),
        SYS_UNLINKAT => sys_unlinkat(args[0] as isize, args[1] as *const u8, args[2] as u32),
        SYS_UMOUNT2 => sys_umount(args[0] as *const u8, args[1]),
        SYS_MOUNT => sys_mount(
            args[0] as *const u8,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3],
            args[4] as *const u8,
        ),
        SYS_STATFS => sys_statfs(args[0] as *const u8, args[1] as *const u8),
        SYS_FACCESSAT => sys_faccessat(),
        SYS_CHDIR => sys_chdir(args[0] as *const u8),
        SYS_CLOSE => sys_close(args[0]),
        SYS_PIPE2 => sys_pipe2(args[0] as *mut i32, args[1] as usize),
        SYS_GETDENTS64 => sys_getdents64(args[0] as isize, args[1] as *mut u8, args[2]),
        SYS_LSEEK => sys_lseek(args[0], args[1], args[2]),
        SYS_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_READV => sys_readv(args[0], args[1] as *const usize, args[2]),
        SYS_WRITEV => sys_writev(args[0], args[1] as *const usize, args[2]),
        SYS_PREAD64 => sys_pread64(args[0], args[1] as *const u8, args[2], args[3]),
        SYS_SENDFILE => sys_sendfile(args[0], args[1], args[2], args[3]),
        SYS_PSELECT6 => sys_pselect(
            args[0] as usize,
            args[1] as *mut u8,
            args[2] as *mut u8,
            args[3] as *mut u8,
            args[4] as *mut usize,
        ),
        SYS_PPOLL => sys_ppoll(),
        SYS_READLINKAT => sys_readlinkat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3],
        ),
        SYS_NEWFSTATAT => sys_newfstatat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as *const usize,
            args[3],
        ),
        SYS_FSTAT => sys_fstat(args[0] as isize, args[1] as *mut u8),
        SYS_FSYNC => 0,
        SYS_UTIMENSAT => sys_utimensat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as *const usize,
            args[3],
        ),
        SYS_EXIT => sys_exit(args[0] as i32),
        SYS_EXIT_GROUP => sys_exit_group(args[0] as i32),
        SYS_SET_TID_ADDRESS => sys_set_tid_address(args[0] as *mut usize),
        SYS_NANOSLEEP => sys_nanosleep(args[0] as *const u8),
        SYS_SETITIMER => 0,
        SYS_CLOCK_GETTIME => sys_clock_gettime(args[0], args[1] as *mut u64),
        SYS_SYSLOG => 0,
        SYS_SCHED_YIELD => sys_sched_yield(),
        SYS_KILL => sys_kill(args[0], args[1] as u32),
        SYS_TGKILL => 0,
        SYS_TIMES => sys_times(args[0] as *const u8),
        SYS_SETPGID => sys_setpgid(),
        SYS_GETPGID => sys_getpgid(),
        SYS_UNAME => sys_uname(args[0] as *const u8),
        SYS_GETRUSAGE => sys_getrusage(args[0] as isize, args[1] as *mut u8),
        SYS_UMASK => sys_umask(),
        SYS_GETTIMEOFDAY => sys_gettimeofday(args[0] as *const u8),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_GETUID => sys_getuid(),
        SYS_GETEUID => sys_geteuid(),
        SYS_GETEGID => sys_getegid(),
        SYS_GETTID => sys_gettid(),
        SYS_SYSINFO => sys_sysinfo(),
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
        SYS_MPROTECT => 0,
        SYS_MSYNC => 0,
        SYS_MADVISE => sys_madvise(args[0] as *const u8, args[1], args[2]),
        SYS_WAIT4 => sys_wait4(args[0] as isize, args[1] as *mut i32),
        SYS_PRLIMIT64 => {
            sys_prlimit64(args[0], args[1], args[2] as *const u8, args[3] as *const u8)
        }
        SYS_RENAMEAT2 => sys_renameat2(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as isize,
            args[3] as *const u8,
            args[4] as u32,
        ),

        // HINT: 这里需要先保留，虽然文档没要求，但现有实现是可用的
        // SYS_EXEC => sys_exec(
        //     args[0] as *const u8,
        //     args[1] as *const usize,
        //     args[2] as *const usize,
        // ),
        _ => panic!("unsupported syscall: {:?}", syscall_id),
    }
}
