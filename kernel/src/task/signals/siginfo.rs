use crate::trap::TrapContext;

pub const MAX_SIGNUM: u32 = 64;

pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[repr(u32)]
pub enum Signal {
    EMPTY = 0,

    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6,
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,
    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGSTKFLT = 16,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,
    SIGTTIN = 21,
    SIGTTOU = 22,
    SIGURG = 23,
    SIGXCPU = 24,
    SIGXFSZ = 25,
    SIGVTALRM = 26,
    SIGPROF = 27,
    SIGWINCH = 28,
    SIGIO = 29,
    SIGPWR = 30,
    SIGSYS = 31,
    /* --- other realtime signals --- */
    SIGTIMER = 32,
    SIGCANCEL = 33,
    SIGSYNCCALL = 34,
    SIGRT_3 = 35,
    SIGRT_4 = 36,
    SIGRT_5 = 37,
    SIGRT_6 = 38,
    SIGRT_7 = 39,
    SIGRT_8 = 40,
    SIGRT_9 = 41,
    SIGRT_10 = 42,
    SIGRT_11 = 43,
    SIGRT_12 = 44,
    SIGRT_13 = 45,
    SIGRT_14 = 46,
    SIGRT_15 = 47,
    SIGRT_16 = 48,
    SIGRT_17 = 49,
    SIGRT_18 = 50,
    SIGRT_19 = 51,
    SIGRT_20 = 52,
    SIGRT_21 = 53,
    SIGRT_22 = 54,
    SIGRT_23 = 55,
    SIGRT_24 = 56,
    SIGRT_25 = 57,
    SIGRT_26 = 58,
    SIGRT_27 = 59,
    SIGRT_28 = 60,
    SIGRT_29 = 61,
    SIGRT_30 = 62,
    SIGRT_31 = 63,
}

impl Signal {
    pub fn empty() -> Self {
        Signal::EMPTY
    }
}

bitflags! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    pub struct SigMask: usize {
        const SIGHUP    = 1 << 1;
        const SIGINT    = 1 << 2;
        const SIGQUIT   = 1 << 3;
        const SIGILL    = 1 << 4;
        const SIGTRAP   = 1 << 5;
        const SIGABRT   = 1 << 6;
        const SIGBUS    = 1 << 7;
        const SIGFPE    = 1 << 8;
        const SIGKILL   = 1 << 9;
        const SIGUSR1   = 1 << 10;
        const SIGSEGV   = 1 << 11;
        const SIGUSR2   = 1 << 12;
        const SIGPIPE   = 1 << 13;
        const SIGALRM   = 1 << 14;
        const SIGTERM   = 1 << 15;
        const SIGSTKFLT = 1 << 16;
        const SIGCHLD   = 1 << 17;
        const SIGCONT   = 1 << 18;
        const SIGSTOP   = 1 << 19;
        const SIGTSTP   = 1 << 20;
        const SIGTTIN   = 1 << 21;
        const SIGTTOU   = 1 << 22;
        const SIGURG    = 1 << 23;
        const SIGXCPU   = 1 << 24;
        const SIGXFSZ   = 1 << 25;
        const SIGVTALRM = 1 << 26;
        const SIGPROF   = 1 << 27;
        const SIGWINCH  = 1 << 28;
        const SIGIO     = 1 << 29;
        const SIGPWR    = 1 << 30;
        const SIGSYS    = 1 << 31;
        const   SIGTIMER    = 1 << 32;
        const   SIGCANCEL   = 1 << 33;
        const   SIGSYNCCALL = 1 << 34;
        /* --- other realtime signals --- */
        const   SIGRT_3     = 1 << 35;
        const   SIGRT_4     = 1 << 36;
        const   SIGRT_5     = 1 << 37;
        const   SIGRT_6     = 1 << 38;
        const   SIGRT_7     = 1 << 39;
        const   SIGRT_8     = 1 << 40;
        const   SIGRT_9     = 1 << 41;
        const   SIGRT_10    = 1 << 42;
        const   SIGRT_11    = 1 << 43;
        const   SIGRT_12    = 1 << 44;
        const   SIGRT_13    = 1 << 45;
        const   SIGRT_14    = 1 << 46;
        const   SIGRT_15    = 1 << 47;
        const   SIGRT_16    = 1 << 48;
        const   SIGRT_17    = 1 << 49;
        const   SIGRT_18    = 1 << 50;
        const   SIGRT_19    = 1 << 51;
        const   SIGRT_20    = 1 << 52;
        const   SIGRT_21    = 1 << 53;
        const   SIGRT_22    = 1 << 54;
        const   SIGRT_23    = 1 << 55;
        const   SIGRT_24    = 1 << 56;
        const   SIGRT_25    = 1 << 57;
        const   SIGRT_26    = 1 << 58;
        const   SIGRT_27    = 1 << 59;
        const   SIGRT_28    = 1 << 60;
        const   SIGRT_29    = 1 << 61;
        const   SIGRT_30    = 1 << 62;
        const   SIGRT_31    = 1 << 63;

    }
}

pub type SigSet = SigMask;

impl SigMask {
    pub fn add(&mut self, signum: u32) {
        if signum >= MAX_SIGNUM {
            panic!(
                "[Kernel] task/signals/siginfo.rs: invalid signum: {}",
                signum
            );
        }
        *self |= SigMask::from_bits_truncate(1 << signum);
    }

    pub fn sub(&mut self, signum: u32) {
        if signum >= MAX_SIGNUM {
            panic!(
                "[Kernel] task/signals/siginfo.rs: invalid signum: {}",
                signum
            );
        }
        *self -= SigMask::from_bits_truncate(1 << signum);
    }

    pub fn add_other(&mut self, other: SigMask) {
        *self |= other;
    }

    pub fn sub_other(&mut self, other: SigMask) {
        *self -= other;
    }

    pub fn if_contains(&self, signum: u32) -> bool {
        self.contains(SigMask::from_bits_truncate(1 << signum))
    }
}

//作为信号处理上下文压入栈中
// [man7: 关于 signal context 的要求](https://man7.org/linux/man-pages/man7/signal.7.html)
#[derive(Debug, Clone)]
#[repr(C)]
pub struct SignalContext {
    pub context: TrapContext,
    pub mask: SigMask,
}

impl SignalContext {
    pub fn from_another(cx: &TrapContext, mask: SigMask) -> Self {
        SignalContext {
            context: cx.clone(),
            mask: mask.clone(),
        }
    }
}

impl SigMask {
    pub fn fetch(&mut self) -> Option<u32> {
        let mut signum = 1;
        while signum < MAX_SIGNUM {
            if self.if_contains(signum) {
                return Some(signum);
            }
            signum += 1;
        }
        None
    }
}

#[repr(usize)]
#[allow(non_camel_case_types)]
// ```c
// #define SIG_BLOCK          0	/* for blocking signals */
// #define SIG_UNBLOCK        1	/* for unblocking signals */
// #define SIG_SETMASK        2	/* for setting the signal mask */
// ```
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum MaskFlags {
    SIG_BLOCK = 0,
    SIG_UNBLOCK = 1,
    SIG_SETMASK = 2,

    UNKNOWN,
}

impl MaskFlags {
    pub fn from_how(how: usize) -> Self {
        match how {
            0 => MaskFlags::SIG_BLOCK,
            1 => MaskFlags::SIG_UNBLOCK,
            2 => MaskFlags::SIG_SETMASK,

            _ => MaskFlags::UNKNOWN,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SigAction {
    pub sa_handler: usize,
    pub sa_sigaction: usize,
    pub sa_mask: SigMask,
    pub sa_flags: SAFlags,
    pub _sa_restorer: usize, // not used
}

impl SigAction {
    pub fn new() -> Self {
        Self {
            sa_handler: SIG_DFL,
            sa_sigaction: SIG_DFL,
            sa_mask: SigMask::empty(),
            sa_flags: SAFlags::empty(),
            _sa_restorer: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sa_handler == SIG_DFL
            && self.sa_sigaction == SIG_DFL
            && self.sa_mask.is_empty()
            && self.sa_flags.is_empty()
    }

    pub fn mask_block(&mut self, signum: u32) {
        self.sa_mask.add(signum);
    }

    pub fn mask_unblock(&mut self, signum: u32) {
        self.sa_mask.sub(signum);
    }

    pub fn mask_contains(&self, signum: u32) -> bool {
        self.sa_mask.if_contains(signum)
    }
}
bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct SAFlags: isize {
        const SA_NOCLDSTOP = 1;		 /* Don't send SIGCHLD when children stop.  */
        const SA_NOCLDWAIT = 2;		 /* Don't create zombie on child death.  */
        const SA_SIGINFO   = 4;  	 /* Invoke signal-catching function with
                                        three arguments instead of one.  */
        const SA_ONSTACK   = 0x08000000; /* Use signal stack by using `sa_restorer'. */
        const SA_RESTART   = 0x10000000; /* Restart syscall on signal return.  */
        const SA_NODEFER   = 0x40000000; /* Don't automatically block the signal when
                                            its handler is being executed.  */
        const SA_RESETHAND = 0x80000000; /* Reset to SIG_DFL on entry to handler.  */
    }
}

pub fn is_signal_valid(signum: u32) -> bool {
    signum < MAX_SIGNUM
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SigInfo {
    si_signo: i32,  /* Signal number */
    si_errno: i32,  /* An errno value */
    si_code: i32,   /* Signal code */
    si_trapno: i32, /* Trap number that caused hardware-generated signal (unused on most architectures) */
    si_pid: u32,    /* Sending process ID */
    si_uid: u32,    /* Real user ID of sending process */
    si_status: i32, /* Exit value or signal */
    si_utime: i32,  /* User time consumed */
    si_stime: i32,  /* System time consumed */
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct UContext {
    pub uc_flags: usize,
    pub uc_link: *mut UContext,
    pub uc_stack: SignalStack,
    pub sigmask: SigMask,
    pub __unused: [u8; 1024 / 8 - core::mem::size_of::<SigMask>()],
    pub uc_mcontext: MContext,
}

impl UContext {
    pub fn empty() -> Self {
        Self {
            uc_flags: 0,
            uc_link: core::ptr::null_mut(),
            uc_stack: SignalStack {
                ss_sp: 0,
                ss_flags: 0,
                ss_size: 0,
            },
            sigmask: SigMask::empty(),
            __unused: [0; 1024 / 8 - core::mem::size_of::<SigMask>()],
            uc_mcontext: MContext {
                greps: [0; 32],
                __reserved: [0; 528],
            },
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SignalStack {
    pub ss_sp: usize,
    pub ss_flags: i32,
    pub ss_size: usize,
}

// The mcontext_t type is machine-dependent and opaque.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MContext {
    pub greps: [usize; 32],    // general registers
    pub __reserved: [u8; 528], // size of mcontext_t is 784 bytes
}
/* ucontext.h

// arch/risv/include/uapi/asm/ucontext.h
struct ucontext {
    unsigned long	  uc_flags;
    struct ucontext	 *uc_link;
    stack_t		  uc_stack;
    sigset_t	  uc_sigmask;
    __u8		  __unused[1024 / 8 - sizeof(sigset_t)];
    struct sigcontext uc_mcontext;
};

// arch/risv/include/uapi/asm/sigcontext.h
struct sigcontext {
    struct user_regs_struct sc_regs;
    union __riscv_fp_state sc_fpregs;
};

*/
