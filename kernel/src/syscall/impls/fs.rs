//! 文件相关的系统调用

use super::super::errno::*;
use crate::fs::CreateMode;
use crate::fs::FdSet;
use crate::fs::{chdir, make_pipe, open, Dirent, File, Kstat, OpenFlags, Stdin, MNT_TABLE};
use crate::fs::{Statfs, TimeInfo};
use crate::mm::{
    translated_bytes_buffer, translated_mut, translated_ref, translated_str, UserBuffer, VirtAddr,
};
use crate::return_errno;
use crate::task::{current_task, current_user_token};
use crate::task::{suspend_current_and_run_next, TaskControlBlock};
use crate::timer::{get_time, get_timeval};

use alloc::borrow::ToOwned;
use alloc::string::ToString;
use alloc::{sync::Arc, vec::Vec};
use core::mem::size_of;
use fat32::sync_all;
use nix::time::{TimeSpec, TimeVal};
use nix::Iovec;
use spin::RwLock;

use super::*;

const AT_FDCWD: isize = -100;

use time_tracer::{time_trace, TimeTracer};

/// #define SYS_getcwd 17
///
/// 功能: 获取当前工作目录;
///
/// 输入:
///
/// - char *buf: 一块缓存区, 用于保存当前工作目录的字符串.当buf设为NULL, 由系统来分配缓存区.
/// - size: buf 缓存区的大小.
///
/// 返回值:
///
/// - 成功: 返回当前工作目录的字符串的指针.
/// - 失败: 则返回NULL.
///
/// ```c
/// char *buf, size_t size;
/// long ret = syscall(SYS_getcwd, buf, size);
/// ```
pub fn sys_getcwd(buf: *mut u8, size: usize) -> Result {
    // 不要使用  `.is_null`, 可能会由于运行时的 const 评估造成错误的结果?
    if buf as usize == 0 {
        return_errno!(Errno::EFAULT, "buf is NULL");
    }
    if buf as usize != 0 && size == 0 {
        return_errno!(Errno::EINVAL, "buf is not NULL but size is zero");
    }
    let token = current_user_token();
    let task = current_task();
    let inner = task.inner_mut();

    let buf_vec = translated_bytes_buffer(token, buf, size);
    let mut userbuf = UserBuffer::wrap(buf_vec);
    let cwd = inner.cwd.to_string();
    let cwd_str = cwd.as_bytes();
    userbuf.write(cwd_str);
    userbuf.write_at(cwd_str.len(), &[0]); // 添加字符串末尾的\0
    Ok(buf as isize)
}

/// #define SYS_pipe2 59
///
/// 功能: 创建管道;
///
/// 输入:
///
/// - fd\[2\]: 用于保存2个文件描述符.
///     - fd\[0\] 为管道的读出端
///     - fd\[1\] 为管道的写入端.
///
/// 返回值:
///
/// - 成功执行, 返回0.
/// - 失败, 返回-1.
///
/// ```c
/// int fd[2];
/// int ret = syscall(SYS_pipe2, fd, 0);
/// ```
pub fn sys_pipe2(pipe: *mut i32, _flag: i32) -> Result {
    let fd0 = pipe;
    let fd1 = unsafe { pipe.add(1) };

    let task = current_task();
    let token = current_user_token();

    let (pipe_read, pipe_write) = make_pipe();

    // fd_table mut borrow
    let mut fd_table = task.fd_table.write();
    let fd_limit = task.inner_ref().rlimit_nofile.rlim_cur;
    let read_fd = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
    if read_fd >= fd_limit {
        return_errno!(Errno::EMFILE);
    }
    fd_table[read_fd] = Some(pipe_read);

    let write_fd = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
    if write_fd >= fd_limit {
        return_errno!(Errno::EMFILE);
    }
    fd_table[write_fd] = Some(pipe_write);

    drop(fd_table);

    let fd0_phys_addr = translated_mut(token, fd0);
    let fd1_phys_addr = translated_mut(token, fd1);
    *fd0_phys_addr = read_fd as i32;
    *fd1_phys_addr = write_fd as i32;

    Ok(0)
}

/// #define SYS_dup 23
///
/// 功能: 复制文件描述符;
///
/// 输入:
///
/// - fd: 被复制的文件描述符.
///
/// 返回值:
///
/// - 成功: 返回新的文件描述符.
/// - 失败: 返回-1.
///
/// ```c
/// int fd;
/// int ret = syscall(SYS_dup, fd);
/// ```
pub fn sys_dup(old_fd: usize) -> Result {
    let task = current_task();
    // fd_table mut borrow
    let mut fd_table = task.fd_table.write();
    let fd_limit = task.inner_ref().rlimit_nofile.rlim_cur;
    // 超出范围
    if old_fd >= fd_table.len() {
        return_errno!(Errno::EBADF, "oldfd is out of range, oldfd: {}", old_fd);
    }
    // oldfd 不存在
    if fd_table[old_fd].is_none() {
        return_errno!(Errno::EBADF, "oldfd is not exist, oldfd {}", old_fd);
    }

    let new_fd = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
    if new_fd >= fd_limit {
        return_errno!(Errno::EMFILE, "too many fd, newfd: {}", new_fd);
    }
    fd_table[new_fd] = Some(Arc::clone(fd_table[old_fd].as_ref().unwrap()));
    // println!("fd:{:?},limit:{:?}",new_fd,fd_limit);

    Ok(new_fd as isize)
}

/// #define SYS_dup3 24
///
/// 功能: 复制文件描述符, 并指定了新的文件描述符;
///
/// 输入:
///
/// - old: 被复制的文件描述符.
/// - new: 新的文件描述符.
///
/// 返回值:
///
/// - 成功: 返回新的文件描述符.
/// - 失败: 返回-1.
///
/// ```c
/// int old, int new;
/// int ret = syscall(SYS_dup3, old, new, 0);
/// ```
pub fn sys_dup3(old_fd: usize, new_fd: usize) -> Result {
    let task = current_task();
    let mut fd_table = task.fd_table.write();

    // 超出范围或 oldfd 不存在
    if old_fd >= fd_table.len() || fd_table[old_fd].is_none() {
        return_errno!(Errno::EBADF);
    }

    if new_fd >= fd_table.len() {
        for _ in fd_table.len()..(new_fd + 1) {
            fd_table.push(None);
        }
    }

    fd_table[new_fd] = Some(fd_table[old_fd].as_ref().unwrap().clone());
    Ok(new_fd as isize)
}

/// #define SYS_chdir 49
///
/// 功能: 切换工作目录;
///
/// 输入:
///
/// - path: 需要切换到的目录.
///
/// 返回值:
///
/// - 成功: 返回0.
/// - 失败: 返回-1.
///
/// ```c
/// const char *path;
/// int ret = syscall(SYS_chdir, path);
/// ```
pub fn sys_chdir(path: *const u8) -> Result {
    let token = current_user_token();
    let task = current_task();
    let mut inner = task.inner_mut();
    let path = translated_str(token, path);
    let current_path = inner.cwd.clone();
    let new_path = current_path.cd(path.clone());
    if chdir(new_path.clone()) {
        inner.cwd = new_path.clone();
        Ok(0)
    } else {
        return_errno!(Errno::ENOENT);
    }
}

/// #define SYS_openat 56
///
/// 功能: 打开或创建一个文件;
///
/// 输入:
///
/// - fd: 文件所在目录的文件描述符.
/// - filename: 要打开或创建的文件名.如为绝对路径, 则忽略fd.
///   如为相对路径, 且fd是AT_FDCWD, 则filename是相对于当前工作目录来说的.
///   如为相对路径, 且fd是一个文件描述符, 则filename是相对于fd所指向的目录来说的.
/// - flags: 必须包含如下访问模式的其中一种: O_RDONLY, O_WRONLY, O_RDWR.
///   还可以包含文件创建标志和文件状态标志.
/// - mode: 文件的所有权描述.详见`man 7 inode `.
///
/// 返回值: 成功执行, 返回新的文件描述符.失败, 返回-1.
///
/// ```c
/// int fd, const char *filename, int flags, mode_t mode;
/// int ret = syscall(SYS_openat, fd, filename, flags, mode);
/// ```
pub fn sys_openat(fd: i32, filename: *const u8, flags: u32, mode: u32) -> Result {
    time_trace!("sys_openat");
    let task = current_task();
    let token = current_user_token();
    let mut inner = task.inner_mut();
    let mut fd_table = task.fd_table.write();

    let path = translated_str(token, filename);
    let mode = CreateMode::from_bits(mode).unwrap_or(CreateMode::empty());
    let flags = OpenFlags::from_bits(flags).unwrap_or(OpenFlags::empty());
    let fd_limit = inner.rlimit_nofile.rlim_cur;
    if fd as isize == AT_FDCWD {
        let open_path = inner.get_work_path().cd(path);
        let inode = open(open_path.clone(), flags, mode)?;
        let fd = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
        if fd >= fd_limit {
            return_errno!(Errno::EMFILE);
        }
        fd_table[fd] = Some(inode);
        Ok(fd as isize)
        // } else {
        //     return_errno!(Errno::ENOENT, "try open path {:?}", open_path);
        // }
    } else {
        let dirfd = fd as usize;
        // dirfd 不合法
        if dirfd >= fd_table.len() {
            return_errno!(Errno::EINVAL);
        }
        if dirfd >= fd_limit {
            return_errno!(Errno::EMFILE);
        }
        if let Some(file) = &fd_table[dirfd] {
            let open_path = file.path().cd(path.clone());
            // target file 存在
            let tar_file = open(open_path.clone(), flags, mode)?;
            let fd = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
            if fd >= fd_limit {
                return_errno!(Errno::EMFILE);
            }
            fd_table[fd] = Some(tar_file);
            Ok(fd as isize)
            // } else {
            //     return_errno!(Errno::ENOENT, "try to open {:?}", path);
            // }
        } else {
            // dirfd 对应条目为 None
            return_errno!(Errno::ENOENT, "no such a file, fd: {}", dirfd);
        }
    }
}

/// #define SYS_close 57
///
/// 功能: 关闭一个文件描述符;
///
/// 输入:
///
/// - fd: 要关闭的文件描述符.
///
/// 返回值:
///
/// - 成功执行, 返回0.
/// - 失败, 返回-1.
///
/// ```c
/// int fd;
/// int ret = syscall(SYS_close, fd);
/// ```
pub fn sys_close(fd: usize) -> Result {
    let task = current_task();
    let mut fd_table = task.fd_table.write();
    if fd >= fd_table.len() {
        return_errno!(Errno::EBADF, "try to close fd out of range {}", fd);
    }
    if fd_table[fd].is_none() {
        return_errno!(Errno::EBADF, "try to close fd that is not exists {}", fd);
    }
    // 把 fd 对应的值取走, 变为 None
    fd_table[fd].take();
    Ok(0)
}

/// #define SYS_getdents64 61
///
/// 功能: 获取目录的条目;
///
/// 输入:
///
/// - fd: 所要读取目录的文件描述符.
/// - buf: 一个缓存区, 用于保存所读取目录的信息.
/// - len: buf的大小.
///
/// 缓存区的结构如下:
///
/// ```c
/// struct dirent {
///     uint64 d_ino;	// 索引结点号
///     int64 d_off;	// 到下一个dirent的偏移
///     unsigned short d_reclen;	// 当前dirent的长度
///     unsigned char d_type;	// 文件类型
///     char d_name[];	//文件名
/// };
/// ```
///
/// 返回值:
///
/// - 成功执行, 返回读取的字节数.当到目录结尾, 则返回0.
/// - 失败, 则返回-1.
///
/// ```c
/// int fd, struct dirent *buf, size_t len
/// int ret = syscall(SYS_getdents64, fd, buf, len);
/// ```
pub fn sys_getdents64(fd: isize, buf: *mut u8, len: usize) -> Result {
    let token = current_user_token();
    let task = current_task();
    let inner = task.inner_mut();
    let work_path = inner.cwd.clone();
    let buf_vec = translated_bytes_buffer(token, buf, len);
    let mut userbuf = UserBuffer::wrap(buf_vec);
    let mut dirent = Dirent::new();
    let dent_len = size_of::<Dirent>();
    let mut total_len: usize = 0;
    let fd_table = task.fd_table.read();

    if fd == AT_FDCWD {
        let file = open(work_path.clone(), OpenFlags::O_RDONLY, CreateMode::empty())?;
        loop {
            if total_len + dent_len > len {
                break;
            }
            if file.dirent(&mut dirent) > 0 {
                userbuf.write_at(total_len, dirent.as_bytes());
                total_len += dent_len;
            } else {
                break;
            }
        }
        Ok(total_len as isize)
        // } else {
        //     return_errno!(Errno::EBADF, "could not open {:?}", work_path);
        // }
    } else {
        let fd = fd as usize;
        if let Some(file) = &fd_table[fd] {
            loop {
                if total_len + dent_len > len {
                    break;
                }
                if file.dirent(&mut dirent) > 0 {
                    userbuf.write_at(total_len, dirent.as_bytes());
                    total_len += dent_len;
                } else {
                    break;
                }
            }
            Ok(total_len as isize)
        } else {
            return_errno!(Errno::EBADF, "could not find fd {}", fd);
        }
    }
}

/// #define SYS_read 63
///
/// 功能: 从一个文件描述符中读取;
///
/// 输入:
///
/// - fd: 要读取文件的文件描述符.
/// - buf: 一个缓存区, 用于存放读取的内容.
/// - count: 要读取的字节数.
///
/// 返回值:
///
/// - 成功执行, 返回读取的字节数.如为0, 表示文件结束.
/// - 错误, 则返回-1.
///
/// ```c
/// int fd, void *buf, size_t count;
/// ssize_t ret = syscall(SYS_read, fd, buf, count);
/// ```
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> Result {
    time_trace!("sys_read");
    let token = current_user_token();
    let task = current_task();
    let fd_table = task.fd_table.read();

    // 文件描述符不合法
    if fd >= fd_table.len() {
        return_errno!(
            Errno::EBADF,
            "fd is out of range, fd: {}, fd_table.len(): {}",
            fd,
            fd_table.len()
        );
    }
    if let Some(file) = &fd_table[fd] {
        // 文件不可读
        time_trace!("sys_read_2");
        if !file.readable() {
            return_errno!(Errno::EINVAL, "fd is not readable, fd: {}", fd);
        }
        let file = file.clone();

        drop(fd_table); // 释放以避免死锁
        drop(task); // 需要及时释放减少引用数

        // 对 /dev/zero 的处理, 暂时先加在这里
        if file.name() == "zero" || file.name() == "ZERO" {
            let mut userbuffer = UserBuffer::wrap(translated_bytes_buffer(token, buf, len));
            let zero: Vec<u8> = (0..userbuffer.buffers.len()).map(|_| 0).collect();
            userbuffer.write(zero.as_slice());
            return Ok(userbuffer.buffers.len() as isize);
        }

        time_trace!("sys_read_3");
        let file_size = file.file_size();
        let file_offset = file.offset();
        if file_size == 0 {
            warn!("sys_read: file_size is zero!");
        }
        let len = len.min(file_size - file_offset);
        let readsize =
            file.read(UserBuffer::wrap(translated_bytes_buffer(token, buf, len))) as isize;
        Ok(readsize as isize)
    } else {
        return_errno!(Errno::EBADF, "fd is not exist, fd: {}", fd);
    }
}

pub fn sys_pread64(fd: usize, buf: *const u8, len: usize, offset: usize) -> Result {
    time_trace!("sys_pread");
    let token = current_user_token();
    let task = current_task();
    let mut fd_table = task.fd_table.write();

    // 文件描述符不合法
    if fd >= fd_table.len() {
        return_errno!(
            Errno::EBADF,
            "fd {} is out of length of fd_table: {}",
            fd,
            fd_table.len()
        );
    }
    if let Some(file) = &fd_table[fd] {
        // 文件不可读
        if !file.readable() {
            return_errno!(Errno::EBADF, "file can't be read, fd: {}", fd);
        }
        let file = file.clone();

        drop(fd_table); // 释放以避免死锁
        drop(task); // 需要及时释放减少引用数

        // 对 /dev/zero 的处理, 暂时先加在这里
        if file.name() == "zero" || file.name() == "ZERO" {
            let mut userbuffer = UserBuffer::wrap(translated_bytes_buffer(token, buf, len));
            let zero: Vec<u8> = (0..userbuffer.buffers.len()).map(|_| 0).collect();
            userbuffer.write(zero.as_slice());
            return Ok(userbuffer.buffers.len() as isize);
        }

        let file_size = file.file_size();
        if file_size == 0 {
            warn!("sys_read: file_size is zero!");
        }
        let len = len.min(file_size - offset);
        let readsize = file.pread(
            UserBuffer::wrap(translated_bytes_buffer(token, buf, len)),
            offset,
        ) as isize;
        Ok(readsize)
    } else {
        return_errno!(Errno::EBADF, "couldn't find fd: {}", fd);
    }
}

/// #define SYS_write 64
///
/// 功能: 从一个文件描述符中写入;
///
/// 输入:
///
/// - fd: 要写入文件的文件描述符.
/// - buf: 一个缓存区, 用于存放要写入的内容.
/// - count: 要写入的字节数.
///
/// 返回值: 成功执行, 返回写入的字节数.错误, 则返回-1.
///
/// ```c
/// int fd, const void *buf, size_t count;
/// ssize_t ret = syscall(SYS_write, fd, buf, count);
/// ```
pub fn sys_write(fd: i32, buf: *const u8, len: usize) -> Result {
    time_trace!("sys_write");
    let token = current_user_token();
    let task = current_task();
    let fd_table = task.fd_table.read();
    let memory_set = task.memory_set.read();

    // 文件描述符不合法
    if fd as usize >= fd_table.len() {
        return_errno!(
            Errno::EBADF,
            "fd {} is out of length of fd_table: {}",
            fd,
            fd_table.len()
        );
    }

    let is_va_range_valid = memory_set.check_va_range(VirtAddr::from(buf as usize), len);
    if !is_va_range_valid {
        return_errno!(
            Errno::EFAULT,
            "buf is out of accessible address space, buf: {}",
            buf as usize
        );
    }

    if let Some(file) = &fd_table[fd as usize] {
        // 文件不可写
        if !file.writable() {
            return_errno!(
                Errno::EINVAL,
                "fd is not writable, fd: {}, filename: {}",
                fd,
                file.name()
            );
        }
        let file = file.clone();
        drop(fd_table);
        drop(memory_set);

        let write_size =
            file.write(UserBuffer::wrap(translated_bytes_buffer(token, buf, len))) as isize;
        Ok(write_size)
    } else {
        return_errno!(Errno::EBADF, "fd is not found, fd: {}", fd);
    }
}

pub fn sys_pwrite64(fd: i32, buf: *const u8, len: usize, offset: usize) -> Result {
    time_trace!("sys_pwrite");
    let token = current_user_token();
    let task = current_task();
    let fd_table = task.fd_table.read();
    let memory_set = task.memory_set.read();

    // 文件描述符不合法
    if fd as usize >= fd_table.len() {
        return_errno!(
            Errno::EBADF,
            "fd: {}, fd_table.len: {}, fd is out of range",
            fd,
            fd_table.len()
        );
    }

    let is_va_range_valid = memory_set.check_va_range(VirtAddr::from(buf as usize), len);
    if !is_va_range_valid {
        return_errno!(
            Errno::EFAULT,
            "buf is out of accessible address space, buf: {}",
            buf as usize
        );
    }

    if let Some(file) = &fd_table[fd as usize] {
        // 文件不可写
        if !file.writable() {
            return_errno!(
                Errno::EINVAL,
                "fd is not writable, fd: {}, filename: {}",
                fd,
                file.name()
            );
        }
        let file = file.clone();
        drop(fd_table);
        drop(memory_set);

        let write_size = file.pwrite(
            UserBuffer::wrap(translated_bytes_buffer(token, buf, len)),
            offset,
        ) as isize;
        Ok(write_size)
    } else {
        return_errno!(Errno::EBADF, "could not find fd: {}", fd);
    }
}

/// #define SYS_linkat 37
/// 功能: 创建文件的链接;
///
/// 输入:
///
/// - olddirfd: 原来的文件所在目录的文件描述符.
/// - oldpath: 文件原来的名字.如果oldpath是相对路径, 则它是相对于olddirfd目录而言的.如果oldpath是相对路径, 且olddirfd的值为AT_FDCWD, 则它是相对于当前路径而言的.如果oldpath是绝对路径, 则olddirfd被忽略.
/// - newdirfd: 新文件名所在的目录.
/// - newpath: 文件的新名字.newpath的使用规则同oldpath.
/// - flags: 在2.6.18内核之前, 应置为0.其它的值详见`man 2 linkat`.
///
/// 返回值: 成功执行, 返回0.失败, 返回-1.
///
/// ```c
/// int olddirfd, char *oldpath, int newdirfd, char *newpath, unsigned int flags
/// int ret = syscall(SYS_linkat, olddirfd, oldpath, newdirfd, newpath, flags);
/// ```
pub fn sys_linkat(
    _old_dirfd: isize,
    _old_path: *const u8,
    _new_dirfd: isize,
    _new_path: *const u8,
    _flags: u32,
) -> Result {
    todo!()
}

/// #define SYS_unlinkat 35
///
/// 功能: 移除指定文件的链接(可用于删除文件);
///
/// 输入:
///
/// - dirfd: 要删除的链接所在的目录.
/// - path: 要删除的链接的名字.如果path是相对路径, 则它是相对于dirfd目录而言的.如果path是相对路径, 且dirfd的值为AT_FDCWD, 则它是相对于当前路径而言的.如果path是绝对路径, 则dirfd被忽略.
/// - flags: 可设置为0或AT_REMOVEDIR.
///
/// 返回值:
///
/// - 成功执行, 返回0.
/// - 失败, 返回-1.
///
/// ```c
/// int dirfd, char *path, unsigned int flags;
/// syscall(SYS_unlinkat, dirfd, path, flags);
/// ```
pub fn sys_unlinkat(fd: isize, path: *const u8, flags: u32) -> Result {
    let task = current_task();
    let token = current_user_token();
    let inner = task.inner_mut();

    _ = flags;

    let path = translated_str(token, path);
    let open_path = inner.get_work_path().cd(path);

    if fd == AT_FDCWD {
        let file = open(open_path.clone(), OpenFlags::O_RDWR, CreateMode::empty())?;
        file.delete();
        Ok(0)
        // } else {
        //     return_errno!(Errno::ENOENT, "could not open: {:?}", open_path);
        // }
    } else {
        unimplemented!("in sys_unlinkat");
    }
}

/// #define SYS_mkdirat 34
///
/// 功能: 创建目录;
///
/// 输入:
///
/// - dirfd: 要创建的目录所在的目录的文件描述符.
/// - path: 要创建的目录的名称.如果path是相对路径, 则它是相对于dirfd目录而言的.如果path是相对路径, 且dirfd的值为AT_FDCWD, 则它是相对于当前路径而言的.如果path是绝对路径, 则dirfd被忽略.
/// - mode: 文件的所有权描述.详见`man 7 inode `.
///
/// 返回值:
///
/// - 成功执行, 返回0.
/// - 失败, 返回-1.
///
/// ```c
/// int dirfd, const char *path, mode_t mode;
/// int ret = syscall(SYS_mkdirat, dirfd, path, mode);
/// ```
pub fn sys_mkdirat(dirfd: i32, path: *const u8, _mode: u32) -> Result {
    let token = current_user_token();
    let task = current_task();
    let inner = task.inner_mut();
    let fd_table = task.fd_table.read();
    let path = translated_str(token, path);
    let fd_limit = inner.rlimit_nofile.rlim_cur;
    if dirfd as isize == AT_FDCWD {
        let open_path = inner.get_work_path().cd(path);
        let _ = open(
            open_path.clone(),
            OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
            CreateMode::empty(),
        );
        // ) {
        //     Ok(0)
        // } else {
        //     return_errno!(Errno::ENOENT, "could not open: {:?}", open_path);
        // }
        Ok(0)
    } else {
        let dirfd = dirfd as usize;
        if dirfd >= fd_table.len() && dirfd >= fd_limit {
            return_errno!(Errno::EBADF, "fd {} is out of range or reach limit", dirfd);
        }
        if let Some(file) = &fd_table[dirfd] {
            let open_path = file.path().cd(path);

            let _ = open(
                open_path.clone(),
                OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
                CreateMode::empty(),
            );
            Ok(0)
        // else {
        //     return_errno!(Errno::ENOENT, "could not open: {:?}", open_path);
        // }
        } else {
            return_errno!(Errno::EBADF, "could not find fd: {}", dirfd);
        }
    }
}

/// #define SYS_umount2 39
///
/// 功能: 卸载文件系统;
///
/// 输入: 指定卸载目录, 卸载参数;
///
/// 返回值: 成功返回0, 失败返回-1;
///
/// ```c
/// const char *special, int flags;
/// int ret = syscall(SYS_umount2, special, flags);
/// ```
pub fn sys_umount2(p_special: *const u8, flags: usize) -> Result {
    let token = current_user_token();
    let special = translated_str(token, p_special);

    match MNT_TABLE.lock().umount(special, flags as u32) {
        0 => Ok(0),
        -1 => return_errno!(Errno::EINVAL),

        _ => unreachable!(),
    }
}

/// #define SYS_mount 40
///
/// 功能: 挂载文件系统;
///
/// 输入:
///
/// - special: 挂载设备;
/// - dir: 挂载点;
/// - fstype: 挂载的文件系统类型;
/// - flags: 挂载参数;
/// - data: 传递给文件系统的字符串参数, 可为NULL;
///
/// 返回值: 成功返回0, 失败返回-1;
///
/// ```c
/// const char *special, const char *dir, const char *fstype, unsigned long flags, const void *data;
/// int ret = syscall(SYS_mount, special, dir, fstype, flags, data);
/// ```
pub fn sys_mount(
    special: *const u8,
    dir: *const u8,
    fstype: *const u8,
    flags: usize,
    data: *const u8,
) -> Result {
    let token = current_user_token();
    let special = translated_str(token, special);
    let dir = translated_str(token, dir);
    let fstype = translated_str(token, fstype);

    _ = data;

    match MNT_TABLE.lock().mount(special, dir, fstype, flags as u32) {
        0 => Ok(0),
        -1 => return_errno!(Errno::EMFILE, "mount too many"),
        _ => unreachable!(),
    }
}

/// #define SYS_fstat 80
///
/// 功能: 获取文件状态;
///
/// 输入:
///
/// - fd: 文件句柄;
/// - kst: 接收保存文件状态的指针;
///
/// ```c
/// struct kstat {
/// 	dev_t st_dev;
/// 	ino_t st_ino;
/// 	mode_t st_mode;
/// 	nlink_t st_nlink;
/// 	uid_t st_uid;
/// 	gid_t st_gid;
/// 	dev_t st_rdev;
/// 	unsigned long __pad;
/// 	off_t st_size;
/// 	blksize_t st_blksize;
/// 	int __pad2;
/// 	blkcnt_t st_blocks;
/// 	long st_atime_sec;
/// 	long st_atime_nsec;
/// 	long st_mtime_sec;
/// 	long st_mtime_nsec;
/// 	long st_ctime_sec;
/// 	long st_ctime_nsec;
/// 	unsigned __unused[2];
/// };
/// ```
///
/// 返回值: 成功返回0, 失败返回-1;
///
/// ```c
/// int fd;
/// struct kstat kst;
/// int ret = syscall(SYS_fstat, fd, &kst);
/// ```
pub fn sys_fstat(fd: i32, buf: *mut u8) -> Result {
    let token = current_user_token();
    let task = current_task();
    let buf_vec = translated_bytes_buffer(token, buf, size_of::<Kstat>());
    let fd_table = task.fd_table.read();

    let mut userbuf = UserBuffer::wrap(buf_vec);
    let mut kstat = Kstat::new();
    let fd_limit = task.inner_ref().rlimit_nofile.rlim_cur;
    let dirfd = fd as usize;
    if dirfd >= fd_table.len() {
        return_errno!(
            Errno::EBADF,
            "fd {} is out of range: {}",
            dirfd,
            fd_table.len()
        );
    }
    if dirfd >= fd_limit {
        return_errno!(Errno::EBADF, "fd {} reached limit", dirfd);
    }
    if let Some(file) = &fd_table[dirfd] {
        file.fstat(&mut kstat);
        if file.name() == "libc.so" || file.name() == "LIBC.SO" {
            kstat.st_ino = 173;
        } else if file.name() == "dlopen_dso.so" || file.name() == "DLOPEN_DSO.SO" {
            kstat.st_ino = 299;
        } else if file.name() == "tls_get_new-dtv_dso.so" || file.name() == "TLS_GET_NEW-DTV_DSO.SO"
        {
            kstat.st_ino = 514;
        }
        userbuf.write(kstat.as_bytes());
        Ok(0)
    } else {
        return_errno!(Errno::EBADF, "could not find fd: {}", dirfd);
    }
}

pub fn sys_readv(fd: usize, iovp: *const usize, iovcnt: usize) -> Result {
    time_trace!("sys_readv");
    let token = current_user_token();
    let task = current_task();
    let fd_table = task.fd_table.read();
    if fd >= fd_table.len() {
        return_errno!(Errno::EBADF, "fd {} out of range: {}", fd, fd_table.len());
    }
    if let Some(file) = &fd_table[fd] {
        if !file.readable() {
            return_errno!(
                Errno::EINVAL,
                "fd is not readable, fd: {}, filename: {}",
                fd,
                file.name()
            );
        }
        let mut addr = iovp as *const _ as usize;
        let file = file.clone();
        drop(fd_table);

        let file_size = file.file_size();
        let file_offset = file.offset();
        if file_size == 0 {
            warn!("sys_readv: file_size is zero!");
        }

        let mut total_read_len = 0;
        for _ in 0..iovcnt {
            let iov = translated_ref(token, addr as *const Iovec);

            let len = iov.iov_len.min(file_size - file_offset - total_read_len);
            // println!("[DEBUG] sys_readv iov_addr:{:x?} len:{:?},buffer_len:{:?}",iov.iov_base,iov.iov_len,len);
            total_read_len += file.read(UserBuffer::wrap(translated_bytes_buffer(
                token,
                iov.iov_base as *const u8,
                len,
            )));
            addr += size_of::<Iovec>();
        }
        Ok(total_read_len as isize)
    } else {
        return_errno!(Errno::EBADF, "fd {} is not found", fd);
    }
}

pub fn sys_writev(fd: usize, iovp: *const usize, iovcnt: usize) -> Result {
    let token = current_user_token();
    let task = current_task();
    let fd_table = task.fd_table.read();
    // 文件描述符不合法
    if fd >= fd_table.len() {
        return_errno!(
            Errno::EBADF,
            "fd {} is out of fd_table.len: {}",
            fd,
            fd_table.len()
        );
    }
    if let Some(file) = &fd_table[fd] {
        // 文件不可写
        if !file.writable() {
            return_errno!(
                Errno::EINVAL,
                "fd is not writable, fd: {}, filename: {}",
                fd,
                file.name()
            );
        }
        let mut addr = iovp as *const _ as usize;
        let mut total_write_len = 0;
        for _ in 0..iovcnt {
            let iov = translated_ref(token, addr as *const Iovec);
            if iov.iov_len <= 0 {
                addr += size_of::<Iovec>();
                continue;
            }
            total_write_len += file.write(UserBuffer::wrap(translated_bytes_buffer(
                token,
                iov.iov_base as *const u8,
                iov.iov_len,
            )));

            addr += size_of::<Iovec>();
        }

        Ok(total_write_len as isize)
    } else {
        return_errno!(Errno::EBADF, "fd {} did not exist", fd);
    }
}

const TCGETS: usize = 0x5401;
const TCSETS: usize = 0x5402;
const TIOCGPGRP: usize = 0x540f;
const TIOCSPGRP: usize = 0x5410;
const TIOCGWINSZ: usize = 0x5413;
const RTC_RD_TIME: usize = 0xffffffff80247009; // 这个值还需考量

pub fn sys_ioctl(fd: i32, request: usize, argp: *mut u8) -> Result {
    let token = current_user_token();
    let task = current_task();
    let inner = task.inner_mut();
    let fd_table = task.fd_table.read();
    // 文件描述符不合法
    if fd as usize >= fd_table.len() {
        return_errno!(
            Errno::EBADF,
            "fd {} is out of fd_table.len: {}",
            fd,
            fd_table.len()
        );
    }
    match request {
        TCGETS => {}
        TCSETS => {}
        TIOCGPGRP => *translated_mut(token, argp) = 0 as u8,
        TIOCSPGRP => {}
        TIOCGWINSZ => *translated_mut(token, argp) = 0 as u8,
        RTC_RD_TIME => {}
        _ => return_errno!(Errno::EINVAL, "request {} is not supported", request),
    }
    Ok(0)
}

// 暂时写在这里

bitflags! {
#[derive(PartialEq, Eq, Debug)]
    pub struct FcntlFlags:usize{
        const F_DUPFD = 0;
        const F_GETFD = 1;
        const F_SETFD = 2;
        const F_GETFL = 3;
        const F_SETFL = 4;
        const F_GETLK = 5;
        const F_SETLK = 6;
        const F_SETLKW = 7;
        const F_SETOWN = 8;
        const F_GETOWN = 9;
        const F_SETSIG = 10;
        const F_GETSIG = 11;
        const F_SETOWN_EX = 15;
        const F_GETOWN_EX = 16;
        const F_GETOWNER_UIDS = 17;

        // 发现 F_UNLCK = 2 , 这个标记分类待研究
        const F_DUPFD_CLOEXEC = 1030;
    }
}

pub fn sys_fcntl(fd: i32, cmd: usize, arg: Option<usize>) -> Result {
    let task = current_task();
    let cmd = FcntlFlags::from_bits(cmd).unwrap();
    let mut fd_table = task.fd_table.write();
    match cmd {
        FcntlFlags::F_SETFL => {
            let inner = task.inner_mut();
            if let Some(file) = &fd_table[fd as usize] {
                file.set_flags(OpenFlags::from_bits(arg.unwrap() as u32).unwrap());
            } else {
                return_errno!(Errno::EBADF, "fd {} is found", fd);
            }
        }
        // Currently, only one such flag is defined: FD_CLOEXEC (value: 1)
        FcntlFlags::F_GETFD => {
            // Return (as the function result) the file descriptor flags; arg is ignored.
            let inner = task.inner_mut();
            if let Some(file) = &fd_table[fd as usize] {
                return Ok(file.available() as isize);
            } else {
                return_errno!(Errno::EBADF, "fd {} is found", fd);
            }
        }
        FcntlFlags::F_SETFD => {
            // Set the file descriptor flags to the value specified by arg.
            let inner = task.inner_mut();
            if let Some(file) = &fd_table[fd as usize] {
                if arg.unwrap() != 0 {
                    file.set_cloexec();
                }
            } else {
                return_errno!(Errno::EBADF, "fd {} is found", fd);
            }
        }
        FcntlFlags::F_GETFL => {
            // Return (as the function result) the file access mode and the file status flags; arg is ignored.
            // todo
            return Ok(04000);
        }
        FcntlFlags::F_DUPFD_CLOEXEC => {
            let mut inner = task.inner_mut();
            let start_num = arg.unwrap();
            let mut new_fd = 0;
            _ = new_fd;
            let mut tmp_fd = Vec::new();
            let fd_limit = inner.rlimit_nofile.rlim_cur;
            loop {
                new_fd = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
                fd_table[new_fd] = Some(Arc::new(Stdin));
                if new_fd >= start_num {
                    break;
                } else {
                    tmp_fd.push(new_fd);
                }
            }
            for i in tmp_fd {
                fd_table[i].take();
            }
            fd_table[new_fd] = match &fd_table[fd as usize] {
                Some(fd) => Some(Arc::clone(fd)),
                None => return_errno!(Errno::EBADF, "fd {} is not exist", fd),
            };
            fd_table[new_fd].as_ref().unwrap().set_cloexec();
            drop(fd_table);
            return Ok(new_fd as isize);
        }
        _ => return_errno!(Errno::EINVAL, "cmd {:?} is not supported", cmd),
    }
    Ok(0)
}

pub fn sys_newfstatat(
    dirfd: isize,
    pathname: *const u8,
    satabuf: *const usize,
    _flags: usize,
) -> Result {
    let token = current_user_token();
    let task = current_task();
    let inner = task.inner_mut();
    let fd_table = task.fd_table.read();
    let path = translated_str(token, pathname);

    let buf_vec = translated_bytes_buffer(token, satabuf as *const u8, size_of::<Kstat>());
    let mut userbuf = UserBuffer::wrap(buf_vec);
    let mut kstat = Kstat::new();
    let fd_limit = inner.rlimit_nofile.rlim_cur;
    // 相对路径, 在当前工作目录
    if dirfd == AT_FDCWD {
        let open_path = inner.get_work_path().cd(path);
        let inode = open(open_path.clone(), OpenFlags::O_RDONLY, CreateMode::empty())?;
        inode.fstat(&mut kstat);
        userbuf.write(kstat.as_bytes());
        Ok(0)
        // } else {
        //     return_errno!(Errno::ENOENT, "could not open {:?}", open_path);
        // }
    } else {
        let dirfd = dirfd as usize;
        if dirfd >= fd_table.len() {
            return_errno!(Errno::EBADF, "fd {} is out of fd_table_len", dirfd);
        }
        if dirfd >= fd_limit {
            return_errno!(Errno::EBADF, "too many fd, fd: {}", dirfd);
        }

        if let Some(file) = &fd_table[dirfd] {
            let open_path = inner.get_work_path().cd(path);
            let inode = open(open_path, OpenFlags::O_RDONLY, CreateMode::empty())?;
            inode.fstat(&mut kstat);
            userbuf.write(kstat.as_bytes());
            Ok(0)
            // } else {
            //     return_errno!(Errno::UNCLEAR);
            // }
        } else {
            return_errno!(Errno::EBADF, "fd {} could not be found", dirfd);
        }
    }
}

pub fn sys_sendfile(out_fd: i32, in_fd: i32, offset: usize, _count: usize) -> Result {
    let task = current_task();
    let fd_table = task.fd_table.read();
    let mut total_write_size = 0usize;
    if offset as usize != 0 {
        unimplemented!();
    } else {
        let in_file = fd_table[in_fd as usize].as_ref().unwrap();
        let out_file = fd_table[out_fd as usize].as_ref().unwrap();
        let mut data_buffer;
        loop {
            data_buffer = in_file.read_kernel_space();
            let len = data_buffer.len();
            if len == 0 {
                break;
            } else {
                out_file.write_kernel_space(data_buffer);
                total_write_size += len;
            }
        }
        Ok(total_write_size as isize)
    }
}

const UTIME_NOW: u64 = 0x3fffffff;
const UTIME_OMIT: u64 = 0x3ffffffe;

pub fn sys_utimensat(
    dirfd: isize,
    pathname: *const u8,
    times: *const [TimeSpec; 2],
    flags: usize,
) -> Result {
    let token = current_user_token();
    let task = current_task();
    let inner = task.inner_mut();
    let fd_table = task.fd_table.read();

    let mut time0 = TimeSpec::empty();
    let mut time1 = TimeSpec::empty();

    let time = TimeSpec::from_ticks(get_time());
    let mut time_info = TimeInfo::empty();

    if times as usize != 0 {
        let times = translated_ref(token, times);
        time0 = times[0];
        time1 = times[1];
        // { info!("utimensat: {:?}, {:?}, {:x?}, {:x?}",dirfd, pathname, time0, time1); }
        match time0.tv_nsec {
            UTIME_NOW => {
                time_info.atime = time.tv_sec;
            }
            UTIME_OMIT => {
                time_info.atime = 0;
            }
            _ => {
                time_info.atime = time0.tv_sec;
            }
        }
        match time1.tv_nsec {
            UTIME_NOW => {
                time_info.mtime = time.tv_sec;
            }
            UTIME_OMIT => {
                time_info.mtime = 0;
            }
            _ => {
                time_info.mtime = time1.tv_sec;
            }
        }
    }

    _ = flags;

    if dirfd == AT_FDCWD {
        if pathname as usize == 0 {
            unimplemented!();
        } else {
            let pathname = translated_str(token, pathname);
            let path = inner.get_work_path().cd(pathname);
            let file = open(
                path,
                OpenFlags::O_RDWR | OpenFlags::O_CREATE,
                CreateMode::empty(),
            );
            // {
            //     Ok(0)
            // } else {
            //     return_errno!(Errno::UNCLEAR);
            //     // Ok(-ENOENT)
            // }
            Ok(0)
        }
    } else {
        if pathname as usize == 0 {
            if dirfd >= fd_table.len() as isize || dirfd < 0 {
                return Ok(0);
            }
            if let Some(file) = &fd_table[dirfd as usize] {
                file.set_time(time_info);
                Ok(0)
            } else {
                return_errno!(Errno::UNCLEAR);
            }
        } else {
            unimplemented!();
        }
    }
}

// 目前仅支持同当前目录下文件名称更改
pub fn sys_renameat2(
    old_dirfd: isize,
    old_path: *const u8,
    new_dirfd: isize,
    new_path: *const u8,
    _flags: u32,
) -> Result {
    let task = current_task();
    let token = current_user_token();
    let inner = task.inner_mut();
    let old_path = translated_str(token, old_path);
    let new_path = translated_str(token, new_path);

    let old_path = inner.get_work_path().cd(old_path);

    if old_dirfd == AT_FDCWD {
        let old_file = open(old_path, OpenFlags::O_RDWR, CreateMode::empty())?;
        let flag = {
            if old_file.is_dir() {
                OpenFlags::O_RDWR | OpenFlags::O_CREATE | OpenFlags::O_DIRECTROY
            } else {
                OpenFlags::O_RDWR | OpenFlags::O_CREATE
            }
        };
        if new_dirfd == AT_FDCWD {
            let new_path = inner.get_work_path().cd(new_path);
            old_file.rename(new_path, flag);
            Ok(0)
        } else {
            unimplemented!();
        }
        // } else {
        //     panic!("can't find old file");
        // }
    } else {
        unimplemented!();
    }
}

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct SeekFlags: usize {
        const SEEK_SET = 0;   // 参数 offset 即为新的读写位置
        const SEEK_CUR = 1;   // 以目前的读写位置往后增加 offset 个位移量
        const SEEK_END = 2;   // 将读写位置指向文件尾后再增加 offset 个位移量
    }
}

pub fn sys_lseek(fd: usize, off_t: isize, whence: usize) -> Result {
    let task = current_task();
    let inner = task.inner_mut();
    let fd_table = task.fd_table.read();
    // 文件描述符不合法
    if fd >= fd_table.len() {
        return_errno!(Errno::EBADF, "fd {} is out of bounds", fd_table.len());
    }

    if let Some(file) = &fd_table[fd] {
        let flag = SeekFlags::from_bits(whence).unwrap();
        match flag {
            SeekFlags::SEEK_SET => {
                if off_t < 0 {
                    return_errno!(Errno::EINVAL, "offset is negtive");
                }
                file.set_offset(off_t as usize);
                Ok(off_t as isize)
            }
            SeekFlags::SEEK_CUR => {
                let current_offset = file.offset() as isize;
                if current_offset + off_t < 0 {
                    return_errno!(Errno::EINVAL, "new offset is negtive");
                }

                file.set_offset((off_t + current_offset) as usize);
                Ok((off_t + current_offset) as isize)
            }
            SeekFlags::SEEK_END => {
                let end = file.file_size() as isize;
                if end + off_t < 0 {
                    return_errno!(Errno::EINVAL, "new offset is negtive");
                }

                file.set_offset((end + off_t) as usize);
                Ok((end + off_t) as isize)
            }
            // flag wrong
            _ => panic!("sys_lseek: unsupported whence!"),
        }
    } else {
        // file not exists
        return_errno!(Errno::UNCLEAR);
        // -3
    }
}

pub fn sys_readlinkat(dirfd: isize, pathname: *const u8, buf: *const u8, bufsiz: usize) -> Result {
    if dirfd == AT_FDCWD {
        let token = current_user_token();
        let path = translated_str(token, pathname);
        // println!("readlinkat path:{:?}", path);
        if path.as_str() != "/proc/self/exe" {
            panic!("sys_readlinkat: pathname not support");
        }
        let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(token, buf, bufsiz));
        let procinfo = "/lua\0";
        userbuf.write(procinfo.as_bytes());
        let len = procinfo.len() - 1;
        return Ok(len as isize);
    } else {
        panic!("sys_readlinkat: fd not support");
    }
}

pub fn sys_sync() -> Result {
    sync_all();
    Ok(0)
}

pub fn sys_ftruncate64(fd: usize, length: usize) -> Result {
    let task = current_task();
    let fd_table = task.fd_table.read();
    if let Some(file) = &fd_table[fd] {
        file.truncate(length);
        Ok(0)
    } else {
        return_errno!(Errno::UNCLEAR);
    }
}

pub fn sys_pselect6(
    nfds: usize,
    readfds: *mut u8,
    writefds: *mut u8,
    exceptfds: *mut u8,
    timeout: *mut usize,
) -> Result {
    let token = current_user_token();
    let mut r_ready_count = 0;
    let mut w_ready_count = 0;
    let mut e_ready_count = 0;

    let mut timer_interval = TimeVal::new();
    unsafe {
        let sec = translated_ref(token, timeout);
        let usec = translated_ref(token, timeout.add(1));
        timer_interval.sec = *sec;
        timer_interval.usec = *usec;
    }
    let timer = timer_interval + get_timeval();

    let mut rfd_set = FdSet::new();
    let mut wfd_set = FdSet::new();

    let mut ubuf_rfds = {
        if readfds as usize != 0 {
            UserBuffer::wrap(translated_bytes_buffer(token, readfds, size_of::<FdSet>()))
        } else {
            UserBuffer::empty()
        }
    };
    ubuf_rfds.read(rfd_set.as_bytes_mut());

    let mut ubuf_wfds = {
        if writefds as usize != 0 {
            UserBuffer::wrap(translated_bytes_buffer(token, writefds, size_of::<FdSet>()))
        } else {
            UserBuffer::empty()
        }
    };
    ubuf_wfds.read(wfd_set.as_bytes_mut());

    let mut ubuf_efds = {
        if exceptfds as usize != 0 {
            UserBuffer::wrap(translated_bytes_buffer(
                token,
                exceptfds,
                size_of::<FdSet>(),
            ))
        } else {
            UserBuffer::empty()
        }
    };

    let mut r_has_nready = false;
    let mut w_has_nready = false;
    let mut r_all_ready = false;
    let mut w_all_ready = false;

    let mut rfd_vec: Vec<usize> = rfd_set.get_fd_vec();
    let mut wfd_vec: Vec<usize> = wfd_set.get_fd_vec();

    loop {
        // handle read fd set
        let task = current_task();
        let fd_table = task.fd_table.read();
        if readfds as usize != 0 && !r_all_ready {
            for i in 0..rfd_vec.len() {
                let fd = rfd_vec[i];
                if fd >= nfds || fd == 1024 {
                    continue;
                }

                if fd > fd_table.len() {
                    return_errno!(
                        Errno::EBADF,
                        "fd {} is out of fd_table.len: {}",
                        fd,
                        fd_table.len()
                    );
                }
                if fd_table[fd].is_none() {
                    return_errno!(Errno::EBADF, "fd {} could not be found", fd);
                }
                let fdescript = fd_table[fd].as_ref().unwrap();
                if fdescript.r_ready() {
                    r_ready_count += 1;
                    rfd_set.set_fd(fd);
                    // marked for being ready
                    rfd_vec[i] = 1024;
                } else {
                    rfd_set.clear_fd(fd);
                    r_has_nready = true;
                }
            }
            if !r_has_nready {
                r_all_ready = true;
                ubuf_rfds.write(rfd_set.as_bytes());
            }
        }

        // handle write fd set
        if writefds as usize != 0 && !w_all_ready {
            if wfd_vec.len() == 0 {
                wfd_vec = wfd_set.get_fd_vec();
            }

            for i in 0..wfd_vec.len() {
                let fd = wfd_vec[i];
                if fd >= nfds || fd == 1024 {
                    continue;
                }
                if fd > fd_table.len() {
                    return_errno!(
                        Errno::EBADF,
                        "fd {} is out of fd_table.len: {}",
                        fd,
                        fd_table.len()
                    );
                }
                if fd_table[fd].is_none() {
                    return_errno!(Errno::EBADF, "fd {} could not be found", fd);
                }
                let fdescript = fd_table[fd].as_ref().unwrap();
                if fdescript.w_ready() {
                    w_ready_count += 1;
                    wfd_set.set_fd(fd);
                    wfd_vec[i] = 1024;
                } else {
                    wfd_set.clear_fd(fd);
                    w_has_nready = true;
                }
            }
            if !w_has_nready {
                w_all_ready = true;
                ubuf_wfds.write(wfd_set.as_bytes());
            }
        }

        /* Cannot handle exceptfds for now */
        if exceptfds as usize != 0 {
            let mut efd_set = FdSet::new();
            ubuf_efds.read(efd_set.as_bytes_mut());
            e_ready_count = efd_set.count() as isize;
            efd_set.clear_all();
            ubuf_efds.write(efd_set.as_bytes());
        }

        // return anyway
        // return r_ready_count + w_ready_count + e_ready_count;
        // if there are some fds not ready, just wait until time up
        if r_has_nready || w_has_nready {
            r_has_nready = false;
            w_has_nready = false;
            let mut time_remain = get_timeval() - timer;
            if time_remain.is_zero() {
                drop(fd_table);
                drop(task);
                // suspend may nerver end, pipe read, timer
                suspend_current_and_run_next();
            } else {
                ubuf_rfds.write(rfd_set.as_bytes());
                ubuf_wfds.write(wfd_set.as_bytes());
                break;
            }
        } else {
            break;
        }
    }
    Ok((r_ready_count + w_ready_count + e_ready_count) as isize)
}

pub fn sys_statfs(path: *const u8, buf: *const u8) -> Result {
    let token = current_user_token();
    let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(token, buf, size_of::<Statfs>()));
    userbuf.write(Statfs::new().as_bytes());
    Ok(0)
}
