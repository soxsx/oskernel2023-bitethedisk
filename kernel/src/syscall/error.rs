//! 系统调用产生的错误

use alloc::string::String;
use thiserror::Error;

use crate::{fs::AbsolutePath, mm::VirtAddr};

pub type Result<T> = core::result::Result<T, SyscallError>;

#[derive(Debug, Error)]
pub enum SyscallError {
    /// 这里失败会返回 0，这里的 0 是 NULL
    #[error("{1}")]
    ParamInvalid(isize, String),

    #[error("there are too many fd exist")]
    ReachFdLimit(isize),

    #[error("could not find given fd: {1}, or it is invalid")]
    FdInvalid(isize, usize),

    #[error("reach va: {1:?}, which is not accessible")]
    InvalidVirtAddress(isize, VirtAddr),

    #[error("can not write file with fd: {1}, filename: {2}")]
    FileCannotWrite(isize, isize, String),

    #[error("could not find path: {1:?}")]
    PathNotExisted(isize, AbsolutePath),

    #[error("could not enter path: {1:?}")]
    PathCannotReach(isize, String),

    #[error("mmap length should bigger than 0")]
    MmapLengthNotBigEnough(isize),

    #[error("failed to open {1:?}")]
    OpenInodeFailed(isize, AbsolutePath),

    #[error("task with pid: {1} not found")]
    PidNotFound(isize, isize),

    #[error("reach mount table size limit")]
    ReachMountLimit(isize),

    #[error("unmount failed")]
    UnmountFailed(isize),

    #[error("No such file or directory")]
    NoSuchFile(isize),
}

impl SyscallError {
    pub fn error_code(&self) -> isize {
        match self {
            SyscallError::ParamInvalid(error_code, _) => *error_code,
            SyscallError::ReachFdLimit(error_code) => *error_code,
            SyscallError::FdInvalid(error_code, _) => *error_code,
            SyscallError::InvalidVirtAddress(error_code, _) => *error_code,
            SyscallError::FileCannotWrite(error_code, _, _) => *error_code,
            SyscallError::PathNotExisted(error_code, _) => *error_code,
            SyscallError::PathCannotReach(error_code, _) => *error_code,
            SyscallError::MmapLengthNotBigEnough(error_code) => *error_code,
            SyscallError::OpenInodeFailed(error_code, _) => *error_code,
            SyscallError::PidNotFound(error_code, _) => *error_code,
            SyscallError::ReachMountLimit(error_code) => *error_code,
            SyscallError::UnmountFailed(error_code) => *error_code,
            SyscallError::NoSuchFile(error_code) => *error_code,
        }
    }
}
