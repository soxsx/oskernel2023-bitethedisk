//! 根据 SYS_id 分发具体系统调用

use super::{error::SyscallError, impls::*};

macro_rules! syscall_nums {
    ($($sysn:ident = $id:expr),*) => {
        $(
            #[allow(unused)]
            const $sysn: usize = $id;
        )*
    };
}

// 系统调用号
syscall_nums! {
    SYS_GETCWD = 17, SYS_PIPE2  = 59, SYS_DUP    = 23, SYS_DUP3       = 24,
    SYS_CHDIR  = 49, SYS_OPENAT = 56, SYS_CLOSE  = 57, SYS_GETDENTS64 = 61,
    SYS_READ   = 63, SYS_WRITE  = 64, SYS_LINKAT = 37, SYS_UNLINKAT   = 35,

    SYS_MKDIRAT = 34,  SYS_UMOUNT2 = 39,  SYS_MOUNT  = 40,  SYS_FSTAT  = 80,
    SYS_CLONE   = 220, SYS_EXECVE  = 221, SYS_WAIT4  = 260, SYS_EXIT   = 93,
    SYS_GETPPID = 173, SYS_GETPID  = 172, SYS_BRK    = 214, SYS_MUNMAP = 215,

    SYS_MMAP         = 222, SYS_TIMES      = 153, SYS_UNAME  = 160, SYS_SCHED_YIELD = 124,
    SYS_WRITEV       = 66,  SYS_EXIT_GROUP = 94,  SYS_GETUID = 174, SYS_RT_SIGPROMASK = 135,
    SYS_RT_SIGACTION = 134, SYS_IOCTL      = 29,  SYS_FCNTL  = 25, SYS_GETEUID = 175,

    SYS_PPOLL    = 73,  SYS_NEWFSTATAT = 79,  SYS_CLOCK_GETTIME = 113, SYS_GETTID  = 178,
    SYS_SENDFILE = 71,  SYS_SYSLOG     = 116, SYS_FACCESSAT     = 48,  SYS_SYSINFO = 179,
    SYS_KILL     = 129, SYS_UTIMENSAT  = 88,  SYS_RENAMEAT2     = 276, SYS_LSEEK   = 62,

    SYS_GETEGID      = 177, SYS_GETGID    = 176, SYS_SET_ROBUST_LIST = 99,  SYS_PRLIMIT64 = 261,
    SYS_READLINKAT   = 78,  SYS_GETRANDOM = 278, SYS_MPROTECT        = 226, SYS_GETPGID   = 155,
    SYS_GETTIMEOFDAY = 169, SYS_NANOSLEEP = 101, SYS_SET_TID_ADDRESS = 96,  SYS_READV     = 65,

    SYS_SETPGID = 154
}

/// 系统调用分发函数
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    // println!("[DEBUG] syscall:{:?}",syscall_id);
    let ret: core::result::Result<isize, SyscallError> = match syscall_id {
        // TODO: 检查完善
        SYS_CLONE => sys_do_fork(args[0], args[1], args[2], args[3], args[4]),

        SYS_EXECVE => sys_exec(
            args[0] as *const u8,
            args[1] as *const usize,
            args[2] as *const usize,
        ),

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
        SYS_SET_TID_ADDRESS => sys_set_tid_address(args[0] as *mut usize),
        SYS_READV => sys_readv(args[0], args[1] as *const usize, args[2]),
        SYS_WRITEV => sys_writev(args[0], args[1] as *const usize, args[2]),
        SYS_EXIT_GROUP => sys_exit_group(args[0] as i32),
        SYS_GETUID => sys_getuid(),
        SYS_RT_SIGPROMASK => sys_rt_sigprocmask(
            args[0] as i32,
            args[1] as *const usize,
            args[2] as *const usize,
            args[3],
        ),
        SYS_RT_SIGACTION => sys_rt_sigaction(),
        SYS_IOCTL => sys_ioctl(args[0], args[1], args[2] as *mut u8),
        SYS_FCNTL => sys_fcntl(
            args[0] as isize,
            args[1] as usize,
            Option::<usize>::from(args[2]),
        ),
        SYS_GETEUID => sys_geteuid(),
        SYS_PPOLL => sys_ppoll(),
        SYS_NEWFSTATAT => sys_newfstatat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as *const usize,
            args[3],
        ),
        SYS_CLOCK_GETTIME => sys_clock_gettime(args[0], args[1] as *mut u64),
        SYS_GETTID => sys_gettid(),
        SYS_SENDFILE => sys_sendfile(args[0], args[1], args[2], args[3]),
        SYS_SYSLOG => Ok(0),
        SYS_FACCESSAT => Ok(0),
        SYS_SYSINFO => Ok(0),
        SYS_KILL => sys_kill(args[0], args[1] as u32),
        SYS_UTIMENSAT => sys_utimensat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as *const usize,
            args[3],
        ),
        SYS_RENAMEAT2 => sys_renameat2(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as isize,
            args[3] as *const u8,
            args[4] as u32,
        ),
        SYS_LSEEK => sys_lseek(args[0], args[1], args[2]),

        _ => panic!("unsupported syscall, syscall id: {:?}", syscall_id),
    };
    // println!("syscall end");
    match ret {
        Ok(success) => success,
        Err(err) => {
            let error_code = err.error_code();
            warn!("{}", err);
            error_code
        }
    }
}
