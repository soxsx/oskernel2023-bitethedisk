//! 根据 SYS_id 分发具体系统调用

use super::impls::*;

// 系统调用号
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
const SYS_SET_TID_ADDRESS: usize = 96;
const SYS_READV: usize = 65;
const SYS_WRITEV: usize = 66;
const SYS_EXIT_GROUP: usize = 94;
const SYS_GETUID: usize = 174;

/// 系统调用分发函数
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        // TODO: 检查完善
        SYS_CLONE => sys_do_fork(args[0], args[1], args[2], args[3], args[4]),

        SYS_EXECVE => sys_exec(
            args[0] as *const u8,
            args[1] as *const usize,
            args[2] as *const u8,
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
        _ => panic!("unsupported syscall, syscall id: {:?}", syscall_id),
    }
}
