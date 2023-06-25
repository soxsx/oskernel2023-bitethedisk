//! 文件相关的系统调用

use super::super::errno::*;
use crate::fs::open_flags::CreateMode;
use crate::fs::{chdir, file::File, make_pipe, open, Dirent, Kstat, OpenFlags, MNT_TABLE,Stdin};
use crate::mm::{translated_bytes_buffer, translated_mut, translated_str, UserBuffer, VirtAddr};
use crate::task::{current_task, current_user_token, FD_LIMIT};

use alloc::{sync::Arc, vec::Vec};
use core::mem::size_of;

const AT_FDCWD: isize = -100;

#[derive(Clone, Copy, Debug)]
pub struct Iovec {
    iov_base: usize,
    iov_len: usize,
}


/// #define SYS_getcwd 17
///
/// 功能：获取当前工作目录；
///
/// 输入：
///
/// - char *buf：一块缓存区，用于保存当前工作目录的字符串。当buf设为NULL，由系统来分配缓存区。
/// - size：buf 缓存区的大小。
///
/// 返回值：
///
/// - 成功：返回当前工作目录的字符串的指针。
/// - 失败：则返回NULL。
///
/// ```c
/// char *buf, size_t size;
/// long ret = syscall(SYS_getcwd, buf, size);
/// ```
pub fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();

    if buf as usize == 0 {
        unimplemented!();
    } else {
        let buf_vec = translated_bytes_buffer(token, buf, size);
        let mut userbuf = UserBuffer::wrap(buf_vec);
        let cwd = inner.current_path.to_string();
        let cwd_str = cwd.as_bytes();
        userbuf.write(cwd_str);
        userbuf.write_at(cwd_str.len(), &[0]); // 添加字符串末尾的\0

        buf as isize
    }
}

/// #define SYS_pipe2 59
///
/// 功能：创建管道；
///
/// 输入：
///
/// - fd\[2\]：用于保存2个文件描述符。
///     - fd\[0\] 为管道的读出端
///     - fd\[1\] 为管道的写入端。
///
/// 返回值：
///
/// - 成功执行，返回0。
/// - 失败，返回-1。
///
/// ```c
/// int fd[2];
/// int ret = syscall(SYS_pipe2, fd, 0);
/// ```
pub fn sys_pipe2(pipe: *mut i32, _flag: usize) -> isize {
    let fd0 = pipe;
    let fd1 = unsafe { pipe.add(1) };

    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.lock();

    let (pipe_read, pipe_write) = make_pipe();

    let read_fd = inner.alloc_fd();
    if read_fd == FD_LIMIT {
        return -EMFILE;
    }
    inner.fd_table[read_fd] = Some(pipe_read);

    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);

    let fd0_phys_addr = translated_mut(token, fd0 as *mut _);
    let fd1_phys_addr = translated_mut(token, fd1 as *mut _);

    *fd0_phys_addr = read_fd as isize;
    *fd1_phys_addr = write_fd as isize;

    0
}

/// #define SYS_dup 23
///
/// 功能：复制文件描述符；
///
/// 输入：
///
/// - fd：被复制的文件描述符。
///
/// 返回值：
///
/// - 成功：返回新的文件描述符。
/// - 失败：返回-1。
///
/// ```c
/// int fd;
/// int ret = syscall(SYS_dup, fd);
/// ```
pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.lock();

    // 检查传入 fd 的合法性
    if fd >= inner.fd_table.len() || inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    if new_fd > FD_LIMIT {
        return -1;
    }
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));

    new_fd as isize
}

/// #define SYS_dup3 24
///
/// 功能：复制文件描述符，并指定了新的文件描述符；
///
/// 输入：
///
/// - old：被复制的文件描述符。
/// - new：新的文件描述符。
///
/// 返回值：
///
/// - 成功：返回新的文件描述符。
/// - 失败：返回-1。
///
/// ```c
/// int old, int new;
/// int ret = syscall(SYS_dup3, old, new, 0);
/// ```
pub fn sys_dup3(old_fd: usize, new_fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.lock();

    if old_fd >= inner.fd_table.len() || new_fd > FD_LIMIT {
        return -1;
    }
    if inner.fd_table[old_fd].is_none() {
        return -1;
    }
    if new_fd >= inner.fd_table.len() {
        for _ in inner.fd_table.len()..(new_fd + 1) {
            inner.fd_table.push(None);
        }
    }

    inner.fd_table[new_fd] = Some(inner.fd_table[old_fd].as_ref().unwrap().clone());
    new_fd as isize
}

/// #define SYS_chdir 49
///
/// 功能：切换工作目录；
///
/// 输入：
///
/// - path：需要切换到的目录。
///
/// 返回值：
///
/// - 成功：返回0。
/// - 失败：返回-1。
///
/// ```c
/// const char *path;
/// int ret = syscall(SYS_chdir, path);
/// ```
pub fn sys_chdir(path: *const u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.lock();
    let path = translated_str(token, path);
    let current_path = inner.current_path.clone();
    if let Some(new_path) = current_path.cd(path) {
        if chdir(new_path.clone()) {
            inner.current_path = new_path.clone();
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

/// #define SYS_openat 56
///
/// 功能：打开或创建一个文件；
///
/// 输入：
///
/// - fd：文件所在目录的文件描述符。
/// - filename：要打开或创建的文件名。如为绝对路径，则忽略fd。
///   如为相对路径，且fd是AT_FDCWD，则filename是相对于当前工作目录来说的。
///   如为相对路径，且fd是一个文件描述符，则filename是相对于fd所指向的目录来说的。
/// - flags：必须包含如下访问模式的其中一种：O_RDONLY，O_WRONLY，O_RDWR。还可以包含文件创建标志和文件状态标志。
/// - mode：文件的所有权描述。详见`man 7 inode `。
///
/// 返回值：成功执行，返回新的文件描述符。失败，返回-1。
///
/// ```c
/// int fd, const char *filename, int flags, mode_t mode;
/// int ret = syscall(SYS_openat, fd, filename, flags, mode);
/// ```
pub fn sys_openat(fd: isize, filename: *const u8, flags: u32, mode: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.lock();

    let path = translated_str(token, filename);

    let mode = CreateMode::from_bits(mode).map_or(CreateMode::empty(), |m| m);
    let flags = OpenFlags::from_bits(flags).map_or(OpenFlags::empty(), |f| f);

    if fd == AT_FDCWD {
        // 相对路径, 在当前工作目录
        let open_path = inner.get_work_path().join_string(path);
        if let Some(inode) = open(open_path, flags, mode) {
            let fd = inner.alloc_fd();
            if fd == FD_LIMIT {
                return -EMFILE;
            }
            inner.fd_table[fd] = Some(inode);

            fd as isize
        } else {
            -1
        }
    } else {
        let dirfd = fd as usize;
        // dirfd 不合法
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[dirfd] {
            let open_path = file.path().join_string(path.clone());
            if let Some(tar_f) = open(open_path, flags, mode) {
                let fd = inner.alloc_fd();
                if fd == FD_LIMIT {
                    return -EMFILE;
                }
                inner.fd_table[fd] = Some(tar_f);
                // info!("[DEBUG] sys_openat return new fd:{}", fd);
                fd as isize
            } else {
                warn!("[WARNING] sys_openat: can't open file:{}, return -1", path);

                -1
            }
        } else {
            // dirfd 对应条目为 None
            warn!("[WARNING] sys_read: fd {} is none, return -1", dirfd);

            -1
        }
    }
}

/// #define SYS_close 57
///
/// 功能：关闭一个文件描述符；
///
/// 输入：
///
/// - fd：要关闭的文件描述符。
///
/// 返回值：
///
/// - 成功执行，返回0。
/// - 失败，返回-1。
///
/// ```c
/// int fd;
/// int ret = syscall(SYS_close, fd);
/// ```
pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    // 把 fd 对应的值取走，变为 None
    inner.fd_table[fd].take();

    0
}

/// #define SYS_getdents64 61
///
/// 功能：获取目录的条目;
///
/// 输入：
///
/// - fd：所要读取目录的文件描述符。
/// - buf：一个缓存区，用于保存所读取目录的信息。
/// - len：buf的大小。
///
/// 缓存区的结构如下：
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
/// 返回值：
///
/// - 成功执行，返回读取的字节数。当到目录结尾，则返回0。
/// - 失败，则返回-1。
///
/// ```c
/// int fd, struct dirent *buf, size_t len
/// int ret = syscall(SYS_getdents64, fd, buf, len);
/// ```
pub fn sys_getdents64(fd: isize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    let work_path = inner.current_path.clone();
    let buf_vec = translated_bytes_buffer(token, buf, len);
    let mut userbuf = UserBuffer::wrap(buf_vec);
    let mut dirent = Dirent::new();
    let dent_len = size_of::<Dirent>();
    let mut total_len: usize = 0;

    if fd == AT_FDCWD {
        if let Some(file) = open(work_path, OpenFlags::O_RDONLY, CreateMode::empty()) {
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

            total_len as isize
        } else {
            -1
        }
    } else {
        if let Some(file) = &inner.fd_table[fd as usize] {
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

            total_len as isize
        } else {
            -1
        }
    }
}

/// #define SYS_read 63
///
/// 功能：从一个文件描述符中读取；
///
/// 输入：
///
/// - fd：要读取文件的文件描述符。
/// - buf：一个缓存区，用于存放读取的内容。
/// - count：要读取的字节数。
///
/// 返回值：
///
/// - 成功执行，返回读取的字节数。如为0，表示文件结束。
/// - 错误，则返回-1。
///
/// ```c
/// int fd, void *buf, size_t count;
/// ssize_t ret = syscall(SYS_read, fd, buf, count);
/// ```
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();

    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        warn!("[WARNING] sys_read: fd >= inner.fd_table.len, return -1");
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        // 文件不可读
        if !file.readable() {
            warn!("[WARNING] sys_read: file can't read, return -1");
            return -1;
        }
        let file = file.clone();

        drop(inner); // 释放以避免死锁
        drop(task); // 需要及时释放减少引用数

        // 对 /dev/zero 的处理，暂时先加在这里
        if file.name() == "zero" {
            let mut userbuffer = UserBuffer::wrap(translated_bytes_buffer(token, buf, len));
            let zero: Vec<u8> = (0..userbuffer.buffers.len()).map(|_| 0).collect();
            userbuffer.write(zero.as_slice());
            return userbuffer.buffers.len() as isize;
        }

        let file_size = file.file_size();
        if file_size == 0 {
            warn!("[WARNING] sys_read: file_size is zero!");
        }
        let len = file_size.min(len);
        let readsize =
            file.read(UserBuffer::wrap(translated_bytes_buffer(token, buf, len))) as isize;
        // println!("[DEBUG] sys_read: return readsize: {}",readsize);
        readsize
    } else {
        warn!("[WARNING] sys_read: fd {} is none, return -1", fd);
        -1
    }
}

/// #define SYS_write 64
///
/// 功能：从一个文件描述符中写入；
///
/// 输入：
///
/// - fd：要写入文件的文件描述符。
/// - buf：一个缓存区，用于存放要写入的内容。
/// - count：要写入的字节数。
///
/// 返回值：成功执行，返回写入的字节数。错误，则返回-1。
///
/// ```c
/// int fd, const void *buf, size_t count;
/// ssize_t ret = syscall(SYS_write, fd, buf, count);
/// ```
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();

    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        warn!("[WARNING] sys_write: fd >= inner.fd_table.len, return -1");
        return -1;
    }

    let is_va_range_valid = inner
        .memory_set
        .check_va_range(VirtAddr::from(buf as usize), len);
    if !is_va_range_valid {
        return -EFAULT;
    }

    if let Some(file) = &inner.fd_table[fd] {
        // 文件不可写
        if !file.writable() {
            warn!(
                "sys_write: file can't write, return -1, filename: {}",
                file.name()
            );
            return -1;
        }

        let write_size =
            file.write(UserBuffer::wrap(translated_bytes_buffer(token, buf, len))) as isize;

        write_size
    } else {
        -1
    }
}

/// #define SYS_linkat 37
/// 功能：创建文件的链接；
///
/// 输入：
///
/// - olddirfd：原来的文件所在目录的文件描述符。
/// - oldpath：文件原来的名字。如果oldpath是相对路径，则它是相对于olddirfd目录而言的。如果oldpath是相对路径，且olddirfd的值为AT_FDCWD，则它是相对于当前路径而言的。如果oldpath是绝对路径，则olddirfd被忽略。
/// - newdirfd：新文件名所在的目录。
/// - newpath：文件的新名字。newpath的使用规则同oldpath。
/// - flags：在2.6.18内核之前，应置为0。其它的值详见`man 2 linkat`。
///
/// 返回值：成功执行，返回0。失败，返回-1。
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
) -> isize {
    todo!()
}

/// #define SYS_unlinkat 35
///
/// 功能：移除指定文件的链接(可用于删除文件)；
///
/// 输入：
///
/// - dirfd：要删除的链接所在的目录。
/// - path：要删除的链接的名字。如果path是相对路径，则它是相对于dirfd目录而言的。如果path是相对路径，且dirfd的值为AT_FDCWD，则它是相对于当前路径而言的。如果path是绝对路径，则dirfd被忽略。
/// - flags：可设置为0或AT_REMOVEDIR。
///
/// 返回值：
///
/// - 成功执行，返回0。
/// - 失败，返回-1。
///
/// ```c
/// int dirfd, char *path, unsigned int flags;
/// syscall(SYS_unlinkat, dirfd, path, flags);
/// ```
pub fn sys_unlinkat(fd: isize, path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.lock();
    // TODO
    _ = flags;
    let path = translated_str(token, path);
    let open_path = inner.get_work_path().join_string(path);

    if fd == AT_FDCWD {
        if let Some(file) = open(open_path, OpenFlags::O_RDWR, CreateMode::empty()) {
            file.delete();
            0
        } else {
            -1
        }
    } else {
        unimplemented!();
    }
}

/// #define SYS_mkdirat 34
///
/// 功能：创建目录；
///
/// 输入：
///
/// - dirfd：要创建的目录所在的目录的文件描述符。
/// - path：要创建的目录的名称。如果path是相对路径，则它是相对于dirfd目录而言的。如果path是相对路径，且dirfd的值为AT_FDCWD，则它是相对于当前路径而言的。如果path是绝对路径，则dirfd被忽略。
/// - mode：文件的所有权描述。详见`man 7 inode `。
///
/// 返回值：
///
/// - 成功执行，返回0。
/// - 失败，返回-1。
///
/// ```c
/// int dirfd, const char *path, mode_t mode;
/// int ret = syscall(SYS_mkdirat, dirfd, path, mode);
/// ```
pub fn sys_mkdirat(dirfd: isize, path: *const u8, _mode: u32) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    let path = translated_str(token, path);

    if dirfd == AT_FDCWD {
        let open_path = inner.get_work_path().join_string(path);
        if let Some(_) = open(
            open_path,
            OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
            CreateMode::empty(),
        ) {
            0
        } else {
            -1
        }
    } else {
        let dirfd = dirfd as usize;
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[dirfd] {
            let open_path = file.path().join_string(path);

            if let Some(_) = open(
                open_path,
                OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
                CreateMode::empty(),
            ) {
                0
            } else {
                -1
            }
        } else {
            // dirfd 对应条目为 None
            -1
        }
    }
}

/// #define SYS_umount2 39
///
/// 功能：卸载文件系统；
///
/// 输入：指定卸载目录，卸载参数；
///
/// 返回值：成功返回0，失败返回-1；
///
/// ```c
/// const char *special, int flags;
/// int ret = syscall(SYS_umount2, special, flags);
/// ```
pub fn sys_umount2(p_special: *const u8, flags: usize) -> isize {
    let token = current_user_token();
    let special = translated_str(token, p_special);

    MNT_TABLE.lock().umount(special, flags as u32)
}

/// #define SYS_mount 40
///
/// 功能：挂载文件系统；
///
/// 输入：
///
/// - special: 挂载设备；
/// - dir: 挂载点；
/// - fstype: 挂载的文件系统类型；
/// - flags: 挂载参数；
/// - data: 传递给文件系统的字符串参数，可为NULL；
///
/// 返回值：成功返回0，失败返回-1；
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
) -> isize {
    let token = current_user_token();
    let special = translated_str(token, special);
    let dir = translated_str(token, dir);
    let fstype = translated_str(token, fstype);

    _ = data;

    MNT_TABLE.lock().mount(special, dir, fstype, flags as u32)
}

/// #define SYS_fstat 80
///
/// 功能：获取文件状态；
///
/// 输入：
///
/// - fd: 文件句柄；
/// - kst: 接收保存文件状态的指针；
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
/// 返回值：成功返回0，失败返回-1；
///
/// ```c
/// int fd;
/// struct kstat kst;
/// int ret = syscall(SYS_fstat, fd, &kst);
/// ```
pub fn sys_fstat(fd: isize, buf: *mut u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let buf_vec = translated_bytes_buffer(token, buf, size_of::<Kstat>());
    let inner = task.lock();

    let mut userbuf = UserBuffer::wrap(buf_vec);
    let mut kstat = Kstat::new();

    let dirfd = fd as usize;
    if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
        return -1;
    }
    if let Some(file) = &inner.fd_table[dirfd] {
        file.fstat(&mut kstat);
        userbuf.write(kstat.as_bytes());

        0
    } else {
        -1
    }
}


pub fn sys_readv(fd: usize, iovp: *const usize, iovcnt: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.readable() {
            return -1;
        }
        let iovp_buf_p = translated_bytes_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>()).as_ptr();
        let file = file.clone();
        let file_size = file.file_size();
        if file_size == 0 {
            warn!("[WARNING] sys_readv: file_size is zero!");
        }
        let mut addr = iovp_buf_p as *const _ as usize;
        let mut total_read_len = 0;
        drop(inner);
        for _ in 0..iovcnt {
            let iovp = unsafe { &*(addr as *const Iovec) };
            let len = iovp.iov_len.min(file_size-total_read_len);
            total_read_len += file.read(UserBuffer::wrap(translated_bytes_buffer(token, iovp.iov_base as *const u8, len)));
            addr += size_of::<Iovec>();
        }
        total_read_len as isize
    } else {
        -1
    }
}

pub fn sys_writev(fd: usize, iovp: *const usize, iovcnt: usize) -> isize {
//    println!("[DEBUG] enter sys_writev: fd:{}, iovp:0x{:x}, iovcnt:{}",fd,iovp as usize,iovcnt);
//    println!("time:{}",get_time_ms());
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        // 文件不可写
        if !file.writable() {
            return -1;
        }
//       	println!("name:{}",file.name());
//	println!("!!{:?}",translated_bytes_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>()));
        let iovp_buf_p = translated_bytes_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>())[0].as_ptr();
        let mut addr = iovp_buf_p as *const _ as usize;
        let mut total_write_len = 0;
        for _ in 0..iovcnt {
            let iovp = unsafe { &*(addr as *const Iovec) };
//	    println!("iovp:{:?}",iovp);
	    if iovp.iov_len<=0{
		addr += size_of::<Iovec>();
		continue;
	    }
            total_write_len += file.write(UserBuffer::wrap(translated_bytes_buffer(
                token,
                iovp.iov_base as *const u8,
                iovp.iov_len,
            )));
            addr += size_of::<Iovec>();
        }
        drop(inner);
//	println!("size:{:?}",total_write_len);
        total_write_len as isize
    } else {
        -1
    }
}


const TCGETS: usize = 0x5401;
const TCSETS: usize = 0x5402;
const TIOCGPGRP: usize = 0x540f;
const TIOCSPGRP: usize = 0x5410;
const TIOCGWINSZ: usize = 0x5413;
const RTC_RD_TIME: usize = 0xffffffff80247009; // 这个值还需考量

pub fn sys_ioctl(fd: usize, request: usize, argp: *mut u8) -> isize {
    // println!("enter sys_ioctl: fd:{}, request:0x{:x}, argp:{}", fd, request, argp as usize);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    match request {
        TCGETS => {}
        TCSETS => {}
        TIOCGPGRP => *translated_mut(token, argp) = 0 as u8,
        TIOCSPGRP => {}
        TIOCGWINSZ => *translated_mut(token, argp) = 0 as u8,
        RTC_RD_TIME => {}
        _ => panic!("sys_ioctl: unsupported request!"),
    }
    0
}


// 暂时写在这里

bitflags! {
#[derive(PartialEq, Eq)]
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

pub fn sys_fcntl(fd: isize, cmd: usize, arg: Option<usize>) -> isize {
    // println!("[DEBUG] enter sys_fcntl: fd:{}, cmd:{}, arg:{:?}", fd, cmd, arg);
    let task = current_task().unwrap();
    let cmd = FcntlFlags::from_bits(cmd).unwrap();
    match cmd {
        FcntlFlags::F_SETFL => {
            let inner = task.lock();
            if let Some(file) = &inner.fd_table[fd as usize] {
                file.set_flags(OpenFlags::from_bits(arg.unwrap() as u32).unwrap());
            } else {
                panic!("sys_fcntl: fd is not an open file descriptor");
            }
        }
        // Currently, only one such flag is defined: FD_CLOEXEC (value: 1)
        FcntlFlags::F_GETFD => {
            // Return (as the function result) the file descriptor flags; arg is ignored.
            let inner = task.lock();
            if let Some(file) = &inner.fd_table[fd as usize] {
                return file.available() as isize;
            } else {
                panic!("sys_fcntl: fd is not an open file descriptor");
            }
        }
        FcntlFlags::F_SETFD => {
            // Set the file descriptor flags to the value specified by arg.
            let inner = task.lock();
            if let Some(file) = &inner.fd_table[fd as usize] {
                if arg.unwrap() != 0 {
                    file.set_cloexec();
                }
            } else {
                panic!("sys_fcntl: fd is not an open file descriptor");
            }
        }
        FcntlFlags::F_GETFL => {
            // Return (as the function result) the file access mode and the file status flags; arg is ignored.
            // todo
            return 04000;
        }
        FcntlFlags::F_DUPFD_CLOEXEC => {
            let mut inner = task.lock();
            let start_num = arg.unwrap();
            let mut new_fd = 0;
            _ = new_fd;
            let mut tmp_fd = Vec::new();
            loop {
                new_fd = inner.alloc_fd();
                inner.fd_table[new_fd] = Some(Arc::new(Stdin));
                if new_fd >= start_num {
                    break;
                } else {
                    tmp_fd.push(new_fd);
                }
            }
            for i in tmp_fd {
                inner.fd_table[i].take();
            }
            inner.fd_table[new_fd] = Some(Arc::clone(
                inner.fd_table[fd as usize]
                    .as_ref()
                    .expect("sys_fcntl: fd is not an open file descriptor"),
            ));
            inner.fd_table[new_fd].as_ref().unwrap().set_cloexec();
            return new_fd as isize;
        }
        _ => panic!("sys_ioctl: unsupported request!"),
    }
    0
}

pub const ENOENT: isize = 2;
pub const EFAULT: isize = 14;
pub const EMFILE: isize = 24;
pub fn sys_newfstatat(dirfd: isize, pathname: *const u8, satabuf: *const usize, _flags: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    let path = translated_str(token, pathname);

    // println!(
    //     "[DEBUG] enter sys_newfstatat: dirfd:{}, pathname:{}, satabuf:0x{:x}, flags:0x{:x}",
    //     dirfd, path, satabuf as usize, _flags
    // );

    let buf_vec = translated_bytes_buffer(token, satabuf as *const u8, size_of::<Kstat>());
    let mut userbuf = UserBuffer::wrap(buf_vec);
    let mut kstat = Kstat::new();

    // 相对路径, 在当前工作目录
    if dirfd == AT_FDCWD {
        let open_path = inner.get_work_path().join_string(path);
        if let Some(inode) = open(open_path, OpenFlags::O_RDONLY, CreateMode::empty()) {
            inode.fstat(&mut kstat);
            userbuf.write(kstat.as_bytes());
            // panic!();
            0
        } else {
            -ENOENT
        }
    } else {
        let dirfd = dirfd as usize;
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }

        if let Some(file) = &inner.fd_table[dirfd] {
            let open_path = inner.get_work_path().join_string(path);
            if let Some(inode) = open(open_path, OpenFlags::O_RDONLY, CreateMode::empty()) {
                inode.fstat(&mut kstat);
                userbuf.write(kstat.as_bytes());
                0
            } else {
                -1
            }
        } else {
            -ENOENT
        }
    }
}
