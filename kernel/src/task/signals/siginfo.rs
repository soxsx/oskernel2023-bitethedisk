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
    SIGUND1 = 32,
    SIGUND2 = 33,
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
        const SIGUND1    = 1 << 32;
        const SIGUND2    = 1 << 33;
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

// pub struct _MContext {
//     __gregs: [usize; 32],
// }

// pub struct _Signaltstack {
//     ss_sp: usize,
//     ss_flags: u32,
//     ss_size: usize,
// }

// #[repr(C)]
// pub struct UContext {
//     pub __bits: [usize; 25],
// }

// impl UContext {
//     pub fn new() -> Self {
//         Self { __bits: [0; 25] }
//     }

//     pub fn as_bytes(&self) -> &[u8] {
//         let size = core::mem::size_of::<Self>();
//         unsafe { core::slice::from_raw_parts(self as *const _ as usize as *mut u8, size) }
//     }

//     pub fn pc_offset() -> usize {
//         176
//     }

//     pub fn mc_pc(&mut self) -> &mut usize {
//         &mut self.__bits[Self::pc_offset() / size_of::<usize>()]
//     }
// }
