//! SBI
//!
//! 由于文档中给出的是 C 代码，所以需要对应到相应的 Rust 类型
//! 对于具体的 C 类型位宽在 riscv-spec Chapter 18 Calling Convention
//!
//! long - isize
//! unsigned long - usize
//!
//! 当前 u740 所使用的 SBI 信息
//! SBI impl name: OpenSBI
//! SBI impl version: 65536
//! SBI spec version: 3

use core::arch::asm;

use thiserror::Error;

pub struct SBIRet {
    error: isize,
    value: isize,
}

impl SBIRet {
    pub const fn new() -> Self {
        Self {
            error: isize::MIN,
            value: -1,
        }
    }
    pub fn get_sbi_error(&self) -> SBIError {
        self.error.into()
    }
}

#[allow(unused)]
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SBIError {
    #[error("Completed successfully")]
    Success = SBI_SUCCESS,
    #[error("Failed")]
    Failed = SBI_ERR_FAILED,
    #[error("Not supported")]
    NotSupported = SBI_ERR_NOT_SUPPORTED,
    #[error("Invalid parameter(s)")]
    InvalidParam = SBI_ERR_INVALID_PARAM,
    #[error("Denied or not allowed")]
    Denied = SBI_ERR_DENIED,
    #[error("Invalid address(s)")]
    InvalidAddress = SBI_ERR_INVALID_ADDRESS,
    #[error("Already available")]
    AlreadyAvailable = SBI_ERR_ALREADY_AVAILABLE,
    #[error("Already started")]
    AlreadyStarted = SBI_ERR_ALREADY_STARTED,
    #[error("Already stopped")]
    AlreadyStopped = SBI_ERR_ALREADY_STOPPED,
    #[error("Shared memory not available")]
    NoShmem = SBI_ERR_NO_SHMEM,
}

impl From<isize> for SBIError {
    fn from(value: isize) -> Self {
        match value {
            0 => SBIError::Success,
            -1 => SBIError::Failed,
            -2 => SBIError::NotSupported,
            -3 => SBIError::InvalidParam,
            -4 => SBIError::Denied,
            -5 => SBIError::InvalidAddress,
            -6 => SBIError::AlreadyAvailable,
            -7 => SBIError::AlreadyStarted,
            -8 => SBIError::AlreadyStopped,
            -9 => SBIError::NoShmem,
            _ => panic!("invalid value of SBIError"),
        }
    }
}

/// Completed successfully
pub const SBI_SUCCESS: isize = 0;
/// Failed
pub const SBI_ERR_FAILED: isize = -1;
/// Not supported
pub const SBI_ERR_NOT_SUPPORTED: isize = -2;
/// Invalid parameter(s)
pub const SBI_ERR_INVALID_PARAM: isize = -3;
/// Denied or not allowed
pub const SBI_ERR_DENIED: isize = -4;
/// Invalid address(s)
pub const SBI_ERR_INVALID_ADDRESS: isize = -5;
/// Already available
pub const SBI_ERR_ALREADY_AVAILABLE: isize = -6;
/// Already started
pub const SBI_ERR_ALREADY_STARTED: isize = -7;
/// Already stopped
pub const SBI_ERR_ALREADY_STOPPED: isize = -8;
/// Shared memory not available
pub const SBI_ERR_NO_SHMEM: isize = -9;

#[inline(always)]
fn sbi_call(eid: usize, fid: usize, mut a0: usize, mut a1: usize, a2: usize) -> SBIRet {
    unsafe {
        asm! {
            "ecall",
            inlateout("a0") a0,
            inlateout("a1") a1,
            in("a2") a2,
            in("a7") eid,
            in("a6") fid,
        }
    }
    SBIRet {
        error: a0 as isize,
        value: a1 as isize,
    }
}

macro_rules! return_sbi_result {
    ($sbiret:expr) => {{
        let error = $sbiret.get_sbi_error();
        let value = $sbiret.value;
        if matches!(error, SBIError::Success) {
            Ok(value.into())
        } else {
            Err(error)
        }
    }};
}

// ===== Base Extension EID #0x10 =====
const BASE_EXTENSION_EID: usize = 0x10;
/// 获取当前 SBI 实现依赖的 SBI spec 版本
pub fn get_sbi_spec_version() -> Result<isize, SBIError> {
    const FID: usize = 0x0;
    return_sbi_result!(sbi_call(BASE_EXTENSION_EID, FID, 0, 0, 0))
}
#[derive(Debug)]
pub enum SBIImplement {
    BerkeleyBootLoader = 0,
    OpenSBI = 1,
    Xvisor = 2,
    KVM = 3,
    RustSBI = 4,
    Diosix = 5,
    Coffer = 6,
    XenProj = 7,
}

impl From<isize> for SBIImplement {
    fn from(value: isize) -> Self {
        match value {
            0 => SBIImplement::BerkeleyBootLoader,
            1 => SBIImplement::OpenSBI,
            2 => SBIImplement::Xvisor,
            3 => SBIImplement::KVM,
            4 => SBIImplement::RustSBI,
            5 => SBIImplement::Diosix,
            6 => SBIImplement::Coffer,
            7 => SBIImplement::XenProj,
            _ => panic!("unknown SBI implementation"),
        }
    }
}

/// 获取 SBI 实现的 id
pub fn get_sbi_impl_id() -> Result<isize, SBIError> {
    const FID: usize = 0x1;
    return_sbi_result!(sbi_call(BASE_EXTENSION_EID, FID, 0, 0, 0))
}
/// 获取 SBI 实现的版本
pub fn get_sbi_impl_version() -> Result<isize, SBIError> {
    const FID: usize = 0x2;
    return_sbi_result!(sbi_call(BASE_EXTENSION_EID, FID, 0, 0, 0))
}

pub fn echo_sbi_verbose_info<'a>() {
    let sbi_impl_version = get_sbi_impl_version().unwrap();
    let sbi_spec_version = get_sbi_spec_version().unwrap();
    let sbi_impl_name = SBIImplement::from(get_sbi_impl_id().unwrap());
    println!(
        "===== SBI Info =====\nSBI impl name: {:?}\nSBI impl version: {}\nSBI spec version: {}",
        sbi_impl_name, sbi_impl_version, sbi_spec_version
    );
}

/// 检测当前实现相应的 extension 是否可用，0 为不可用
#[allow(unused)]
pub fn probe_sbi_extension(eid: isize) -> Result<isize, SBIError> {
    const FID: usize = 0x3;
    let ret = sbi_call(BASE_EXTENSION_EID, FID, eid as usize, 0, 0);
    if matches!(ret.get_sbi_error(), SBIError::Success) || ret.value == 0 {
        Err(SBIError::NotSupported)
    } else {
        Ok(ret.value)
    }
}
// ===== Hart State Management Extension (EID #0x48534D "HSM") =====
const HSM_EXTENSION_EID: usize = 0x48534D;

#[derive(Debug, PartialEq, Eq)]
pub enum HartStatus {
    /// hart 已上电并正常执行
    Started = 0,
    /// hart 没有运行在 S 或者更低的特权级中，可能运行在 M 态或者被硬件平台
    /// power down 关机
    Stopped = 1,
    StartPending = 2,
    StopPending = 3,
    Suspended = 4,
    SuspendPending = 5,
    ResumePending = 6,
}

impl From<isize> for HartStatus {
    fn from(value: isize) -> Self {
        match value {
            0 => HartStatus::Started,
            1 => HartStatus::Stopped,
            2 => HartStatus::StartPending,
            3 => HartStatus::StopPending,
            4 => HartStatus::Suspended,
            5 => HartStatus::SuspendPending,
            6 => HartStatus::ResumePending,
            _ => panic!("unknown hart status: {}", value),
        }
    }
}

/// 通知 SBI 将 hart 以 S 态从指定地址开始运行
///
/// opaque 会在 hart 在 start_addr 开始运行时放到 a1 寄存器中
pub fn sbi_start_hart(hartid: usize, start_addr: usize, opaque: usize) -> Result<(), isize> {
    const FID: usize = 0x0;
    let sbiret = sbi_call(HSM_EXTENSION_EID, FID, hartid, start_addr, opaque);
    if sbiret.get_sbi_error() != SBIError::Success {
        Err(sbiret.error)
    } else {
        Ok(())
    }
}

/// 停止在 S 态执行该函数的 hart，并将其交由 SBI 处理
pub fn sbi_stop_hart() -> Result<(), isize> {
    const FID: usize = 0x1;
    let sbiret = sbi_call(HSM_EXTENSION_EID, FID, 0, 0, 0);
    if sbiret.get_sbi_error() != SBIError::Success {
        Err(sbiret.error)
    } else {
        Ok(())
    }
}

pub fn get_hart_status(hartid: usize) -> Result<HartStatus, SBIError> {
    const FID: usize = 0x2;
    return_sbi_result!(sbi_call(HSM_EXTENSION_EID, FID, hartid, 0, 0))
}

// ===== legacy SBI call =====

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_SHUTDOWN: usize = 8;

#[inline(always)]
fn legacy_sbi_call(eid: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") eid,
        );
    }
    ret
}

#[cfg_attr(
    target_arch = "riscv32",
    warn(
        deprecated,
        reason = "SBI v0.1 spec counld not been found, current implementation may not work on rv32"
    )
)]
pub fn set_timer(stime_value: u64) -> isize {
    #[cfg(target_arch = "riscv64")]
    {
        legacy_sbi_call(SBI_SET_TIMER, stime_value as usize, 0, 0)
    }
    #[cfg(target_arch = "riscv32")]
    {
        let u32_high = stime_value & 0xFFFF_FFFF_0000_0000;
        let u32_low = stime_value & 0x0000_0000_FFFF_FFFF;
        legacy_sbi_call(eid, u32_low, u32_high, 0)
    }
}

pub fn console_putchar(c: i32) -> isize {
    legacy_sbi_call(SBI_CONSOLE_PUTCHAR, c as usize, 0, 0)
}

pub fn console_getchar() -> isize {
    legacy_sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() -> ! {
    legacy_sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    unreachable!("kernel has already shutdown");
}
