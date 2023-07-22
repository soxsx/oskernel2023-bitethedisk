//! 根据 SYS_id 分发具体系统调用

use super::impls::*;

// 系统调用号
const SYS_GETCWD: usize = 17;
const SYS_DUP: usize = 23;
const SYS_DUP3: usize = 24;
const SYS_FCNTL: usize = 25;
const SYS_IOCTL: usize = 29;
const SYS_MKDIRAT: usize = 34;
const SYS_UNLINKAT: usize = 35;
const SYS_LINKAT: usize = 37;
const SYS_UMOUNT2: usize = 39;
const SYS_MOUNT: usize = 40;
const SYS_STATFS: usize = 43;
const SYS_FTRUNCATE64: usize = 46;
const SYS_FACCESSAT: usize = 48;
const SYS_CHDIR: usize = 49;
const SYS_OPENAT: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_PIPE2: usize = 59;
const SYS_GETDENTS64: usize = 61;
const SYS_LSEEK: usize = 62;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_READV: usize = 65;
const SYS_WRITEV: usize = 66;
const SYS_PREAD64: usize = 67;
const SYS_PWRITE64: usize = 68;
const SYS_SENDFILE: usize = 71;
const SYS_PSELECT6: usize = 72;
const SYS_PPOLL: usize = 73;
const SYS_READLINKAT: usize = 78;
const SYS_NEWFSTATAT: usize = 79;
const SYS_FSTAT: usize = 80;
const SYS_SYNC: usize = 81;
const SYS_FSYNC: usize = 82;
const SYS_UTIMENSAT: usize = 88;
const SYS_EXIT: usize = 93;
const SYS_EXIT_GROUP: usize = 94;
const SYS_SET_TID_ADDRESS: usize = 96;
const SYS_FUTEX: usize = 98;
const SYS_SET_ROBUST_LIST: usize = 99;
const SYS_NANOSLEEP: usize = 101;
const SYS_SETITIMER: usize = 103;
const SYS_CLOCK_GETTIME: usize = 113;
const SYS_SYSLOG: usize = 116;
const SYS_SCHED_YIELD: usize = 124;
const SYS_KILL: usize = 129;
const SYS_TGKILL: usize = 131;
const SYS_RT_SIGACTION: usize = 134;
const SYS_RT_SIGPROMASK: usize = 135;
const SYS_RT_SIGTIMEDWAIT: usize = 137;
const SYS_RT_SIGRETURN: usize = 139;
const SYS_TIMES: usize = 153;
const SYS_SETPGID: usize = 154;
const SYS_GETPGID: usize = 155;
const SYS_UNAME: usize = 160;
const SYS_GETRUSAGE: usize = 165;
const SYS_UMASK: usize = 166;
const SYS_GETTIMEOFDAY: usize = 169;
const SYS_GETPID: usize = 172;
const SYS_GETPPID: usize = 173;
const SYS_GETUID: usize = 174;
const SYS_GETEUID: usize = 175;
const SYS_GETGID: usize = 176;
const SYS_GETEGID: usize = 177;
const SYS_GETTID: usize = 178;
const SYS_SYSINFO: usize = 179;
const SYS_SHMGET: usize = 194;
const SYS_SHMCTL: usize = 195;
const SYS_SHMAT: usize = 196;
const SYS_SHMDT: usize = 197;
const SYS_BRK: usize = 214;
const SYS_MUNMAP: usize = 215;
const SYS_CLONE: usize = 220;
const SYS_EXECVE: usize = 221;
const SYS_MMAP: usize = 222;
const SYS_MPROTECT: usize = 226;
const SYS_MSYNC: usize = 227;
const SYS_WAIT4: usize = 260;
const SYS_PRLIMIT64: usize = 261;
const SYS_RENAMEAT2: usize = 276;
const SYS_GETRANDOM: usize = 278;
const SYS_MEMBARRIER: usize = 283;

// const SYS_SOCKET: usize = 198;
// const SYS_BIND: usize = 200;
// const SYS_LISTEN: usize = 201;
// const SYS_ACCEPT: usize = 202;
// const SYS_CONNECT: usize = 203;
// const SYS_GETSOCKNAME: usize = 204;
pub fn syscall_name(id: usize) -> &'static str {
    match id {
        SYS_GETCWD => "SYS_GETCWD",
        SYS_DUP => "SYS_DUP",
        SYS_DUP3 => "SYS_DUP3",
        SYS_FCNTL => "SYS_FCNTL",
        SYS_IOCTL => "SYS_IOCTL",
        SYS_MKDIRAT => "SYS_MKDIRAT",
        SYS_UNLINKAT => "SYS_UNLINKAT",
        SYS_LINKAT => "SYS_LINKAT",
        SYS_UMOUNT2 => "SYS_UMOUNT2",
        SYS_MOUNT => "SYS_MOUNT",
        SYS_STATFS => "SYS_STATFS",
        SYS_FTRUNCATE64 => "SYS_FTRUNCATE64",
        SYS_FACCESSAT => "SYS_FACCESSAT",
        SYS_CHDIR => "SYS_CHDIR",
        SYS_OPENAT => "SYS_OPENAT",
        SYS_CLOSE => "SYS_CLOSE",
        SYS_PIPE2 => "SYS_PIPE2",
        SYS_GETDENTS64 => "SYS_GETDENTS64",
        SYS_LSEEK => "SYS_LSEEK",
        SYS_READ => "SYS_READ",
        SYS_WRITE => "SYS_WRITE",
        SYS_READV => "SYS_READV",
        SYS_WRITEV => "SYS_WRITEV",
        SYS_PREAD64 => "SYS_PREAD64",
        SYS_PWRITE64 => "SYS_PWRITE64",
        SYS_SENDFILE => "SYS_SENDFILE",
        SYS_PSELECT6 => "SYS_PSELECT6",
        SYS_PPOLL => "SYS_PPOLL",
        SYS_READLINKAT => "SYS_READLINKAT",
        SYS_NEWFSTATAT => "SYS_NEWFSTATAT",
        SYS_FSTAT => "SYS_FSTAT",
        SYS_SYNC => "SYS_SYNC",
        SYS_FSYNC => "SYS_FSYNC",
        SYS_UTIMENSAT => "SYS_UTIMENSAT",
        SYS_EXIT => "SYS_EXIT",
        SYS_EXIT_GROUP => "SYS_EXIT_GROUP",
        SYS_SET_TID_ADDRESS => "SYS_SET_TID_ADDRESS",
        SYS_FUTEX => "SYS_FUTEX",
        SYS_SET_ROBUST_LIST => "SYS_SET_ROBUST_LIST",
        SYS_NANOSLEEP => "SYS_NANOSLEEP",
        SYS_SETITIMER => "SYS_SETITIMER",
        SYS_CLOCK_GETTIME => "SYS_CLOCK_GETTIME",
        SYS_SYSLOG => "SYS_SYSLOG",
        SYS_SCHED_YIELD => "SYS_SCHED_YIELD",
        SYS_KILL => "SYS_KILL",
        SYS_TGKILL => "SYS_TGKILL",
        SYS_RT_SIGACTION => "SYS_RT_SIGACTION",
        SYS_RT_SIGPROMASK => "SYS_RT_SIGPROMASK",
        SYS_RT_SIGTIMEDWAIT => "SYS_RT_SIGTIMEDWAIT",
        SYS_RT_SIGRETURN => "SYS_RT_SIGRETURN",
        SYS_TIMES => "SYS_TIMES",
        SYS_SETPGID => "SYS_SETPGID",
        SYS_GETPGID => "SYS_GETPGID",
        SYS_UNAME => "SYS_UNAME",
        SYS_GETRUSAGE => "SYS_GETRUSAGE",
        SYS_UMASK => "SYS_UMASK",
        SYS_GETTIMEOFDAY => "SYS_GETTIMEOFDAY",
        SYS_GETPID => "SYS_GETPID",
        SYS_GETPPID => "SYS_GETPPID",
        SYS_GETUID => "SYS_GETUID",
        SYS_GETEUID => "SYS_GETEUID",
        SYS_GETGID => "SYS_GETGID",
        SYS_GETEGID => "SYS_GETEGID",
        SYS_GETTID => "SYS_GETTID",
        SYS_SYSINFO => "SYS_SYSINFO",
        SYS_SHMGET => "SYS_SHMGET",
        SYS_SHMCTL => "SYS_SHMCTL",
        SYS_SHMAT => "SYS_SHMAT",
        SYS_SHMDT => "SYS_SHMDT",
        SYS_BRK => "SYS_BRK",
        SYS_MUNMAP => "SYS_MUNMAP",
        SYS_CLONE => "SYS_CLONE",
        SYS_EXECVE => "SYS_EXECVE",
        SYS_MMAP => "SYS_MMAP",
        SYS_MPROTECT => "SYS_MPROTECT",
        SYS_MSYNC => "SYS_MSYNC",
        SYS_WAIT4 => "SYS_WAIT4",
        SYS_PRLIMIT64 => "SYS_PRLIMIT64",
        SYS_RENAMEAT2 => "SYS_RENAMEAT2",
        SYS_GETRANDOM => "SYS_GETRANDOM",

        _ => "unknown",
    }
}

/// 系统调用分发函数
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    // println!(
    //     "[DEBUG] {}. pid:{:?}",
    //     syscall_name(syscall_id),
    //     current_task().unwrap().pid.0
    // );
    let ret = match syscall_id {

        SYS_CLONE => sys_do_fork(args[0], args[1], args[2], args[3], args[4]),

        SYS_TGKILL => sys_tgkill(args[0] as isize, args[1], args[2] as isize),

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
        SYS_LSEEK => sys_lseek(args[0], args[1] as isize, args[2]),
        SYS_GETEGID => Ok(0),
        SYS_GETGID => Ok(0),
        SYS_SET_ROBUST_LIST => Ok(-1),
        SYS_PRLIMIT64 => Ok(0),
        SYS_READLINKAT => sys_readlinkat(
            args[0] as isize,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3],
        ),
        SYS_GETRANDOM => sys_getrandom(args[0] as *const u8, args[1], args[2]),
        SYS_GETPGID => Ok(0),
        SYS_SETPGID => Ok(0),
        SYS_SYNC => sys_sync(),
        SYS_FTRUNCATE64 => sys_ftruncate64(args[0], args[1]),
        SYS_PSELECT6 => sys_pselect6(
            args[0] as usize,
            args[1] as *mut u8,
            args[2] as *mut u8,
            args[3] as *mut u8,
            args[4] as *mut usize,
        ),
        SYS_GETRUSAGE => sys_getrusage(args[0] as isize, args[1] as *mut u8),
        SYS_SETITIMER => Ok(0),
        SYS_TGKILL => Ok(0),
        SYS_UMASK => Ok(0),
        SYS_FSYNC => Ok(0),
        SYS_MSYNC => Ok(0),
        SYS_SHMGET => sys_shmget(args[0], args[1], args[2]),
        SYS_SHMCTL => sys_shmctl(args[0], args[1], args[2] as *const u8),
        SYS_SHMAT => sys_shmat(args[0], args[1], args[2]),
        SYS_SHMDT => sys_shmdt(args[0]),
        SYS_PREAD64 => sys_pread64(args[0], args[1] as *const u8, args[2], args[3]),
        SYS_PWRITE64 => sys_pwrite64(args[0], args[1] as *const u8, args[2], args[3]),
        SYS_STATFS => sys_statfs(args[0] as *const u8, args[1] as *const u8),
        SYS_FUTEX => Ok(0),
        SYS_RT_SIGTIMEDWAIT => Ok(0),
        SYS_RT_SIGRETURN => Ok(0),
        SYS_MPROTECT => sys_mprotect(args[0], args[1], args[2]),
        SYS_MEMBARRIER => Ok(0),
        _ => panic!("unsupported syscall, syscall id: {:?}", syscall_id),
    };
    match ret {
        Ok(data) => data,
        Err(err) => {
            let errno = err as isize;
            if errno > 0 {
                -errno
            } else {
                errno
            }
        }
    }
}
