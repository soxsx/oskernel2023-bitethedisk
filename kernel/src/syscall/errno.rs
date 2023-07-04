//! Linux 错误码，系统调用的错误都存储于 [`errno`] 中
//!
//! [`errno`]: <https://man7.org/linux/man-pages/man3/errno.3.html>

#![allow(unused)]


pub const EPERM	 : isize =	 1;	/* Operation not permitted */
pub const ENOENT : isize =	 2;	/* No such file or directory */
pub const ESRCH	 : isize =	 3;	/* No such process */
pub const EINTR	 : isize =	 4;	/* Interrupted system call */
pub const EIO	 : isize =	 5;	/* I/O error */
pub const ENXIO	 : isize =	 6;	/* No such device or address */
pub const E2BIG	 : isize =	 7;	/* Argument list too long */
pub const ENOEXEC: isize =	 8;	/* Exec format error */
pub const EBADF	 : isize =	 9;	/* Bad file number */
pub const ECHILD : isize =	10;	/* No child processes */
pub const EAGAIN : isize =	11;	/* Try again */
pub const ENOMEM : isize =	12;	/* Out of memory */
pub const EACCES : isize =	13;	/* Permission denied */
pub const EFAULT : isize =	14;	/* Bad address */
pub const ENOTBLK: isize =	15;	/* Block device required */
pub const EBUSY	 : isize =	16;	/* Device or resource busy */
pub const EEXIST : isize =	17;	/* File exists */
pub const EXDEV	 : isize =	18;	/* Cross-device link */
pub const ENODEV : isize =	19;	/* No such device */
pub const ENOTDIR: isize =	20;	/* Not a directory */
pub const EISDIR : isize =	21;	/* Is a directory */
pub const EINVAL : isize =	22;	/* Invalid argument */
pub const ENFILE : isize =	23;	/* File table overflow */
pub const EMFILE : isize =	24;	/* Too many open files */
pub const ENOTTY : isize =	25;	/* Not a typewriter */
pub const ETXTBSY: isize =	26;	/* Text file busy */
pub const EFBIG	 : isize =	27;	/* File too large */
pub const ENOSPC : isize =	28;	/* No space left on device */
pub const ESPIPE : isize =	29;	/* Illegal seek */
pub const EROFS	 : isize =	30;	/* Read-only file system */
pub const EMLINK : isize =	31;	/* Too many links */
pub const EPIPE	 : isize =	32;	/* Broken pipe */
pub const EDOM	 : isize =	33;	/* Math argument out of domain of func */
pub const ERANGE : isize =	34;	/* Math result not representable */

