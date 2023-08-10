pub mod dispatcher;
pub mod errno;
pub mod futex;
pub mod impls;

macro_rules! gen_syscallid {
    ($($id:ident = $v:expr),*$(,)?) => {
        $(
            #[allow(unused)]
            pub const $id: usize = $v;
         )*
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        pub enum SyscallId {
            $(
                $id = $v,
             )*
        }
        impl core::fmt::Debug for SyscallId {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", match self {
                    $(
                        Self::$id => stringify!($id),
                     )*
                })
            }
        }
        impl core::fmt::Display for SyscallId {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", match self {
                    $(
                        Self::$id => stringify!($id),
                     )*
                })
            }
        }
        impl From<usize> for SyscallId {
            fn from(v: usize) -> SyscallId {
                match v {
                    $(
                        $v => SyscallId::$id,
                     )*
                    unknown => panic!("unsupported syscall id: {}", unknown),
                }
            }
        }
    }
}

gen_syscallid! {
    SYS_GETITIMER = 102,
    SYS_GETCWD = 17,
    SYS_DUP = 23,
    SYS_DUP3 = 24,
    SYS_FCNTL = 25,
    SYS_IOCTL = 29,
    SYS_MKDIRAT = 34,
    SYS_UNLINKAT = 35,
    SYS_LINKAT = 37,
    SYS_UMOUNT2 = 39,
    SYS_MOUNT = 40,
    SYS_STATFS = 43,
    SYS_FTRUNCATE64 = 46,
    SYS_FACCESSAT = 48,
    SYS_CHDIR = 49,
    SYS_OPENAT = 56,
    SYS_CLOSE = 57,
    SYS_PIPE2 = 59,
    SYS_GETDENTS64 = 61,
    SYS_LSEEK = 62,
    SYS_READ = 63,
    SYS_WRITE = 64,
    SYS_READV = 65,
    SYS_WRITEV = 66,
    SYS_PREAD64 = 67,
    SYS_PWRITE64 = 68,
    SYS_SENDFILE = 71,
    SYS_PSELECT6 = 72,
    SYS_PPOLL = 73,
    SYS_READLINKAT = 78,
    SYS_NEWFSTATAT = 79,
    SYS_FSTAT = 80,
    SYS_SYNC = 81,
    SYS_FSYNC = 82,
    SYS_UTIMENSAT = 88,
    SYS_EXIT = 93,
    SYS_EXIT_GROUP = 94,
    SYS_SET_TID_ADDRESS = 96,
    SYS_FUTEX = 98,
    SYS_SET_ROBUST_LIST = 99,
    SYS_GET_ROBUST_LIST = 100,
    SYS_NANOSLEEP = 101,
    SYS_SETITIMER = 103,
    SYS_CLOCK_GETTIME = 113,
    SYS_SYSLOG = 116,
    SYS_SCHED_YIELD = 124,
    SYS_KILL = 129,
    SYS_TILL = 130,
    SYS_TGKILL = 131,
    SYS_SIGTIMEDWAIT = 137,
    SYS_TIMES = 153,
    SYS_SETPGID = 154,
    SYS_GETPGID = 155,
    SYS_UNAME = 160,
    SYS_GETRUSAGE = 165,
    SYS_UMASK = 166,
    SYS_GETTIMEOFDAY = 169,
    SYS_GETPID = 172,
    SYS_GETPPID = 173,
    SYS_GETUID = 174,
    SYS_GETEUID = 175,
    SYS_GETGID = 176,
    SYS_GETEGID = 177,
    SYS_GETTID = 178,
    SYS_SYSINFO = 179,
    SYS_SHMGET = 194,
    SYS_SHMCTL = 195,
    SYS_SHMAT = 196,
    SYS_SHMDT = 197,
    SYS_BRK = 214,
    SYS_MUNMAP = 215,
    SYS_CLONE = 220,
    SYS_EXECVE = 221,
    SYS_MMAP = 222,
    SYS_MPROTECT = 226,
    SYS_MSYNC = 227,
    SYS_WAIT4 = 260,
    SYS_PRLIMIT64 = 261,
    SYS_RENAMEAT2 = 276,
    SYS_GETRANDOM = 278,
    SYS_MEMBARRIER = 283,
    SYS_SCHED_SETAFFINITY = 122,
    SYS_SCHED_GETAFFINITY = 123,
    SYS_SCHEED_GETSCHEDULER = 120,
    SYS_SCHED_GETPARAM = 121,
    SYS_SCHED_SETSCHEDULER = 119,
    SYS_CLOCK_GETRES = 114,
    SYS_SOCKETPAIR = 199,
    SYS_MADVISE = 233,
    SYS_CLOCK_NANOSLEEP = 115,
    SYS_SIGACTION = 134,
    SYS_SIGPROCMASK = 135,
    SYS_SIGRETURN = 139,
    SYS_SOCKET = 198,
    SYS_BIND = 200,
    SYS_LISTEN = 201,
    SYS_ACCEPT = 202,
    SYS_CONNECT = 203,
    SYS_GETSOCKNAME = 204,
    SYS_SENDTO = 206,
    SYS_RECVFROM = 207,
    SYS_SETSOCKOPT = 208,
    SYS_TIMER_SETTIME = 110,
    SYS_TIMER_GETOVERRUN = 109,
}
