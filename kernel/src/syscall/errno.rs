//! Linux 错误码，系统调用的错误都存储于 [`errno`] 中
//!
//! [`errno`]: <https://man7.org/linux/man-pages/man3/errno.3.html>

#![allow(unused)]

/// Bad address
pub const EFAULT: isize = 14;
/// Too many open files
pub const EMFILE: isize = 24;
