use super::errno::*;
use crate::fs::open_flags::CreateMode;
use crate::fs::{
    chdir, make_pipe, open, Dirent, FdSet, File, Kstat, OpenFlags, Statfs, Stdin, MNT_TABLE,
};
use crate::mm::{
    translated_bytes_buffer, translated_mut, translated_ref, translated_str, UserBuffer, VirtAddr,
};
use crate::task::{
    current_task, current_user_token, suspend_current_and_run_next, FD_LIMIT, RLIMIT_NOFILE,
};
use crate::timer::{get_timeval, TimeVal, Timespec};
use alloc::{sync::Arc, vec::Vec};
use core::mem::size_of;

const AT_FDCWD: isize = -100;

/// ### 写文件函数
/// - `fd` 表示待写入文件的文件描述符；
/// - `buf` 表示内存中缓冲区的起始地址；
/// - `len` 表示内存中缓冲区的长度。
/// - 返回值：成功写入的长度。
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    // println!(
    //     "[DEBUG] pid {} enter sys_write: fd:{}, buffer address:0x{:x}, len:{}",
    //     current_task().unwrap().getpid(),
    //     fd,
    //     buf as usize,
    //     len
    // );
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    // if len != 8192 {
    //     println!("buffer content:{:?}", UserBuffer::new(translated_byte_buffer(token, buf, len)));
    // }

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
                "[WARNING] sys_write: file can't write, return -1, filename: {}",
                file.name()
            );
            return -1;
        }

        let write_size =
            file.write(UserBuffer::wrap(translated_bytes_buffer(token, buf, len))) as isize;
        // debug!("[DEBUG] sys_write: return write_size: {}",write_size);
        write_size
    } else {
        // warn!("[WARNING] sys_write: fd {} is none, return -1", fd);
        -1
    }
}

/// ### 读文件函数
/// - `fd` 表示待读取文件的文件描述符；
/// - `buf` 表示内存中缓冲区的起始地址；
/// - `len` 表示读取字符个数。
/// - 返回值：读出的字符。
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    // println!(
    //     "[DEBUG] enter sys_read: fd:{}, buffer address:0x{:x}, len:{}",
    //     fd, buf as usize, len
    // );
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

/// 功能：打开或创建一个文件；
///
/// 输入：
/// - fd：文件所在目录的文件描述符。
/// - filename：要打开或创建的文件名。如为绝对路径，则忽略fd。如为相对路径，且fd是AT_FDCWD，则filename是相对于当前工作目录来说的。如为相对路径，且fd是一个文件描述符，则filename是相对于fd所指向的目录来说的。
/// - flags：必须包含如下访问模式的其中一种：O_RDONLY，O_WRONLY，O_RDWR。还可以包含文件创建标志和文件状态标志。
/// - mode：文件的所有权描述。详见`man 7 inode `。
///
/// 返回值：成功执行，返回新的文件描述符。失败，返回-1。
pub fn sys_openat(dirfd: isize, path: *const u8, flags: u32, mode: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.lock();

    let path = translated_str(token, path);

    let mode = CreateMode::from_bits(mode).map_or(CreateMode::empty(), |m| m);
    let flags = OpenFlags::from_bits(flags).map_or(OpenFlags::empty(), |f| f);

    if dirfd == AT_FDCWD {
        // 如果是当前工作目录
        if let Some(inode) = open(inner.get_work_path(), path.as_str(), flags, mode) {
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
        let dirfd = dirfd as usize;
        // dirfd 不合法
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[dirfd] {
            if let Some(tar_f) = open(file.name(), path.as_str(), flags, mode) {
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

/// ### 关闭文件函数
/// - `fd`：文件描述符
/// - 返回值
///     - 成功关闭：0
///     - 失败：-1
pub fn sys_close(fd: usize) -> isize {
    // println!("[DEBUG] enter sys_close: fd:{}",fd);
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
    // info!("[DEBUG] sys_close return 0");

    0
}

/// 功能：创建管道；
///
/// 输入：
///  fd\[2\]：用于保存2个文件描述符。其中
///
/// * fd\[0\]: 管道的读出端
/// * fd\[1\]: 管道的写入端。
///
/// ### 为当前进程打开一个管道。
/// - `pipe` 表示应用地址空间中的一个长度为 `2` 的 `usize` 数组的起始地址，
/// 内核需要按顺序将管道读端和写端的文件描述符写入到数组中。
/// - 返回值：如果出现了错误则返回 -1，否则返回 0 。可能的错误原因是：传入的地址不合法。
/// - syscall ID：59
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

    println!("fd0: {:?}, fd1: {:?}", fd0, fd1);
    println!("read_fd: {:?}, write_fd: {:?}", read_fd, write_fd);

    let fd0_phys_addr = translated_mut(token, fd0 as *mut _);
    let fd1_phys_addr = translated_mut(token, fd1 as *mut _);

    *fd0_phys_addr = read_fd as isize;
    *fd1_phys_addr = write_fd as isize;
    println!("fd0_phys: {:?}, fd1_phys: {:?}", fd0_phys_addr, fd1_phys_addr);

    0
}

/// ### 将进程中一个已经打开的文件描述符复制一份并分配到一个新的文件描述符中。
/// - 参数：fd 表示进程中一个已经打开的文件的文件描述符。
/// - 返回值：
///     - 能够访问已打开文件的新文件描述符。
///     - 如果出现了错误则返回 -1，可能的错误原因是：传入的 fd 并不对应一个合法的已打开文件。
/// - syscall ID：23

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.lock();

    // 做资源检查，目前只检查 RLIMIT_NOFILE 这一种
    let rlim_max = inner.resource[RLIMIT_NOFILE].rlim_max;
    if inner.fd_table.len() - 1 == rlim_max - 1 {
        return -EMFILE;
    }

    // 检查传入 fd 的合法性
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    if new_fd > FD_LIMIT {
        return -1;
    }
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));
    new_fd as isize
}

/// ### 将进程中一个已经打开的文件描述符复制一份并分配到一个指定的文件描述符中。
/// - 参数：
///     - old_fd 表示进程中一个已经打开的文件的文件描述符。
///     - new_fd 表示进程中一个指定的文件描述符中。
/// - 返回值：
///     - 能够访问已打开文件的新文件描述符。
///     - 如果出现了错误则返回 -1，可能的错误原因是：
///         - 传入的 old_fd 为空。
///         - 传入的 old_fd 不存在
///         - 传入的 new_fd 超出描述符数量限制 (典型值：128)
/// - syscall ID：24
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

    //let mut act_fd = new_fd;
    //if inner.fd_table[new_fd].is_some() {
    //    act_fd = inner.alloc_fd();
    //}
    //let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(inner.fd_table[old_fd].as_ref().unwrap().clone());
    new_fd as isize
}

pub fn sys_mkdirat(dirfd: isize, path: *const u8, _mode: u32) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    let path = translated_str(token, path);

    // println!("[DEBUG] enter sys_mkdirat: dirfd:{}, path:{}. mode:{:o}",dirfd,path,mode);
    if dirfd == AT_FDCWD {
        if let Some(_) = open(
            inner.get_work_path(),
            path.as_str(),
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
            if let Some(_) = open(
                file.name(),
                path.as_str(),
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

/// buf：用于保存当前工作目录的字符串。当 buf 设为 NULL，由系统来分配缓存区
pub fn sys_getcwd(buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();

    if buf as usize == 0 {
        unimplemented!();
    } else {
        let buf_vec = translated_bytes_buffer(token, buf, len);
        let mut userbuf = UserBuffer::wrap(buf_vec);
        let cwd = inner.current_path.as_bytes();
        userbuf.write(cwd);
        userbuf.write_at(cwd.len(), &[0]); // 添加字符串末尾的\0
        return buf as isize;
    }
}

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

pub fn sys_umount(p_special: *const u8, flags: usize) -> isize {
    let token = current_user_token();
    let special = translated_str(token, p_special);
    MNT_TABLE.lock().umount(special, flags as u32)
}

pub fn sys_unlinkat(fd: isize, path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.lock();
    // todo
    _ = flags;

    let path = translated_str(token, path);
    // println!("[DEBUG] enter sys_unlinkat: fd:{}, path:{}, flags:{}",fd,path,flags);
    if fd == AT_FDCWD {
        if let Some(file) = open(
            inner.get_work_path(),
            path.as_str(),
            OpenFlags::O_RDWR,
            CreateMode::empty(),
        ) {
            file.delete();
            0
        } else {
            -1
        }
    } else {
        unimplemented!();
    }
}

pub fn sys_chdir(path: *const u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.lock();
    let path = translated_str(token, path);

    // println!("[DEBUG] enter sys_chdir: path:{}",path);

    if let Some(new_cwd) = chdir(inner.current_path.as_str(), &path) {
        inner.current_path = new_cwd;
        0
    } else {
        -1
    }
}

pub fn sys_fstat(fd: isize, buf: *mut u8) -> isize {
    // info!("[DEBUG] enter sys_fstat: fd:{}, buf:0x{:x}", fd, buf as usize);
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
        // println!("kstat:{:?}",kstat);
        userbuf.write(kstat.as_bytes());
        0
    } else {
        -1
    }
}

pub fn sys_getdents64(fd: isize, buf: *mut u8, len: usize) -> isize {
    // println!("[DEBUG] enter sys_getdents64: fd:{}, buf:{}, len:{}", fd, buf as usize, len);
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
        if let Some(file) = open(
            "/",
            work_path.as_str(),
            OpenFlags::O_RDONLY,
            CreateMode::empty(),
        ) {
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
            return total_len as isize;
        } else {
            return -1;
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
            return total_len as isize;
        } else {
            return -1;
        }
    }
}

// 暂时放在这里
bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct SeekFlags: usize {
        const SEEK_SET = 0;   // 参数 offset 即为新的读写位置
        const SEEK_CUR = 1;   // 以目前的读写位置往后增加 offset 个位移量
        const SEEK_END = 2;   // 将读写位置指向文件尾后再增加 offset 个位移量
    }
}

pub fn sys_lseek(fd: usize, off_t: usize, whence: usize) -> isize {
    // println!("[DEBUG] enter sys_lseek: fd:{},off_t:{},whence:{}",fd,off_t,whence);

    let task = current_task().unwrap();
    let inner = task.lock();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &inner.fd_table[fd] {
        let flag = SeekFlags::from_bits(whence).unwrap();
        match flag {
            SeekFlags::SEEK_SET => {
                file.set_offset(off_t);
                off_t as isize
            }
            SeekFlags::SEEK_CUR => {
                let current_offset = file.offset();
                file.set_offset(off_t + current_offset);
                (off_t + current_offset) as isize
            }
            SeekFlags::SEEK_END => {
                let end = file.file_size();
                file.set_offset(end + off_t);
                (end + off_t) as isize
            }
            // flag wrong
            _ => panic!("sys_lseek: unsupported whence!"),
        }
    } else {
        // file not exists
        -3
    }
}

// 暂时放在这里
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
// 暂时放在这里
#[derive(Clone, Copy, Debug)]
pub struct Iovec {
    iov_base: usize,
    iov_len: usize,
}

pub fn sys_writev(fd: usize, iovp: *const usize, iovcnt: usize) -> isize {
    // println!("[DEBUG] enter sys_writev: fd:{}, iovp:0x{:x}, iovcnt:{}",fd,iovp as usize,iovcnt);
    // println!("time:{}",get_time_ms());
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
        let iovp_buf =
            translated_bytes_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>())
                .pop()
                .unwrap();
        let file = file.clone();
        let mut addr = iovp_buf.as_ptr() as *const _ as usize;
        let mut total_write_len = 0;
        drop(inner);
        for _ in 0..iovcnt {
            let iovp = unsafe { &*(addr as *const Iovec) };
            total_write_len += file.write(UserBuffer::wrap(translated_bytes_buffer(
                token,
                iovp.iov_base as *const u8,
                iovp.iov_len,
            )));
            addr += size_of::<Iovec>();
        }
        total_write_len as isize
    } else {
        -1
    }
}

pub fn sys_newfstatat(
    dirfd: isize,
    pathname: *const u8,
    satabuf: *const usize,
    _flags: usize,
) -> isize {
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

    if dirfd == AT_FDCWD {
        if let Some(inode) = open(
            inner.get_work_path(),
            path.as_str(),
            OpenFlags::O_RDONLY,
            CreateMode::empty(),
        ) {
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
            if let Some(inode) = open(
                file.name(),
                path.as_str(),
                OpenFlags::O_RDONLY,
                CreateMode::empty(),
            ) {
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

pub fn sys_utimensat(dirfd: isize, pathname: *const u8, time: *const usize, flags: usize) -> isize {
    // println!(
    //     "[DEBUG] enter sys_utimensat: dirfd:{}, pathname:{}, time:{}, flags:{}",
    //     dirfd, pathname as usize, time as usize, flags
    // );
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();

    _ = flags;

    if dirfd == AT_FDCWD {
        if pathname as usize == 0 {
            unimplemented!();
        } else {
            let pathname = translated_str(token, pathname);
            if let Some(_file) = open(
                inner.get_work_path(),
                pathname.as_str(),
                OpenFlags::O_RDWR,
                CreateMode::empty(),
            ) {
                unimplemented!(); // 记得重新制作文件镜像
            } else {
                -ENOENT
            }
        }
    } else {
        if pathname as usize == 0 {
            if dirfd >= inner.fd_table.len() as isize || dirfd < 0 {
                return 0;
            }
            if let Some(file) = &inner.fd_table[dirfd as usize] {
                let timespec_buf =
                    translated_bytes_buffer(token, time as *const u8, size_of::<Kstat>())
                        .pop()
                        .unwrap();
                let addr = timespec_buf.as_ptr() as *const _ as usize;
                let timespec = unsafe { &*(addr as *const Timespec) };
                file.set_time(timespec);
                0
            } else {
                -1
            }
        } else {
            unimplemented!();
        }
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
        let iovp_buf =
            translated_bytes_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>())
                .pop()
                .unwrap();
        let file = file.clone();
        let file_size = file.file_size();
        if file_size == 0 {
            warn!("[WARNING] sys_readv: file_size is zero!");
        }
        let mut addr = iovp_buf.as_ptr() as *const _ as usize;
        let mut total_read_len = 0;
        drop(inner);
        for _ in 0..iovcnt {
            let iovp = unsafe { &*(addr as *const Iovec) };
            let len = file_size.min(iovp.iov_len);
            total_read_len += file.read(UserBuffer::wrap(translated_bytes_buffer(
                token,
                iovp.iov_base as *const u8,
                len,
            )));
            addr += size_of::<Iovec>();
        }
        total_read_len as isize
    } else {
        -1
    }
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

pub fn sys_statfs(path: *const u8, buf: *const u8) -> isize {
    let token = current_user_token();

    _ = path;

    let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(token, buf, size_of::<Statfs>()));
    userbuf.write(Statfs::new().as_bytes());
    0
}

pub fn sys_pread64(fd: usize, buf: *const u8, count: usize, offset: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.lock();
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        let old_offset = file.offset();
        file.set_offset(offset);
        let readsize =
            file.read(UserBuffer::wrap(translated_bytes_buffer(token, buf, count))) as isize;
        file.set_offset(old_offset);
        readsize
    } else {
        -1
    }
}

pub fn sys_sendfile(out_fd: usize, in_fd: usize, offset: usize, _count: usize) -> isize {
    // println!(
    //     "[DEBUG] enter sys_sendfile: out_fd:{}, in_fd:{}, offset:{}, count:{}",
    //     out_fd, in_fd, offset, _count
    // );
    let task = current_task().unwrap();
    let inner = task.lock();
    let fd_table = inner.fd_table.clone();
    drop(inner);
    let mut total_write_size = 0usize;
    if offset as usize != 0 {
        unimplemented!();
    } else {
        let in_file = fd_table[in_fd].as_ref().unwrap();
        let out_file = fd_table[out_fd].as_ref().unwrap();
        let mut data_buffer;
        loop {
            data_buffer = in_file.read_kernel_space();
            // println!("data_buffer:{:?}",data_buffer);
            let len = data_buffer.len();
            if len == 0 {
                break;
            } else {
                out_file.write_kernel_space(data_buffer);
                total_write_size += len;
            }
        }
        total_write_size as isize
    }
}

// 目前仅支持同当前目录下文件名称更改
pub fn sys_renameat2(
    old_dirfd: isize,
    old_path: *const u8,
    new_dirfd: isize,
    new_path: *const u8,
    _flags: u32,
) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.lock();
    let old_path = translated_str(token, old_path);
    let new_path = translated_str(token, new_path);

    // println!(
    //     "[DEBUG] enter sys_renameat2: old_dirfd:{}, old_path:{}, new_dirfd:{}, new_path:{}, flags:0x{:x}",
    //     old_dirfd, old_path, new_dirfd, new_path, _flags
    // );
    if old_dirfd == AT_FDCWD {
        if let Some(old_file) = open(
            inner.get_work_path(),
            old_path.as_str(),
            OpenFlags::O_RDWR,
            CreateMode::empty(),
        ) {
            let flag = {
                if old_file.is_dir() {
                    OpenFlags::O_RDWR | OpenFlags::O_CREATE | OpenFlags::O_DIRECTROY
                } else {
                    OpenFlags::O_RDWR | OpenFlags::O_CREATE
                }
            };
            if new_dirfd == AT_FDCWD {
                if let Some(new_file) = open(
                    inner.get_work_path(),
                    new_path.as_str(),
                    flag,
                    CreateMode::empty(),
                ) {
                    let first_cluster = old_file.head_cluster();
                    new_file.set_head_cluster(first_cluster);
                    old_file.delete();
                    0
                } else {
                    panic!("can't find new file");
                }
            } else {
                unimplemented!();
            }
        } else {
            panic!("can't find old file");
        }
    } else {
        unimplemented!();
    }
}

pub fn sys_umask() -> isize {
    0
}

pub fn sys_readlinkat(dirfd: isize, pathname: *const u8, buf: *const u8, bufsiz: usize) -> isize {
    if dirfd == AT_FDCWD {
        let token = current_user_token();
        let path = translated_str(token, pathname);
        if path.as_str() != "/proc/self/exe" {
            panic!("sys_readlinkat: pathname not support");
        }
        let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(token, buf, bufsiz));
        let procinfo = "/lmbench_all\0";
        userbuf.write(procinfo.as_bytes());
        let len = procinfo.len() - 1;
        return len as isize;
    } else {
        panic!("sys_readlinkat: fd not support");
    }
}

pub fn sys_pselect(
    nfds: usize,
    readfds: *mut u8,
    writefds: *mut u8,
    exceptfds: *mut u8,
    timeout: *mut usize,
) -> isize {
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

    // println!("[DEBUG] enter sys_pselect: nfds:{}, readfds:{:?} ,writefds:{:?}, exceptfds:{:?}, timeout:{:?}",nfds,ubuf_rfds,ubuf_wfds,ubuf_efds,timer_interval);

    let mut r_has_nready = false;
    let mut w_has_nready = false;
    let mut r_all_ready = false;
    let mut w_all_ready = false;

    let mut rfd_vec = Vec::new();
    let mut wfd_vec = Vec::new();

    loop {
        /* handle read fd set */
        let task = current_task().unwrap();
        let inner = task.lock();
        let fd_table = &inner.fd_table;
        if readfds as usize != 0 && !r_all_ready {
            if rfd_vec.len() == 0 {
                rfd_vec = rfd_set.get_fd_vec();
                if rfd_vec[rfd_vec.len() - 1] >= nfds {
                    return -1; // invalid fd
                }
            }

            for i in 0..rfd_vec.len() {
                let fd = rfd_vec[i];
                if fd == 1024 {
                    continue;
                }
                if fd > fd_table.len() || fd_table[fd].is_none() {
                    return -1; // invalid fd
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

        /* handle write fd set */
        if writefds as usize != 0 && !w_all_ready {
            if wfd_vec.len() == 0 {
                wfd_vec = wfd_set.get_fd_vec();
                if wfd_vec[wfd_vec.len() - 1] >= nfds {
                    return -1; // invalid fd
                }
            }

            for i in 0..wfd_vec.len() {
                let fd = wfd_vec[i];
                if fd == 1024 {
                    continue;
                }
                if fd > fd_table.len() || fd_table[fd].is_none() {
                    return -1; // invalid fd
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
            let time_remain = get_timeval() - timer;
            if time_remain.is_zero() {
                // not reach timer (now < timer)
                drop(fd_table);
                drop(inner);
                drop(task);
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
    // println!("pselect return: r_ready_count:{}, w_ready_count:{}, e_ready_count:{}",r_ready_count,w_ready_count,e_ready_count);
    r_ready_count + w_ready_count + e_ready_count
}

/// 输入：
///
/// - olddirfd：原来的文件所在目录的文件描述符。
/// - oldpath：文件原来的名字。如果oldpath是相对路径，则它是相对于olddirfd目录而言的。如果oldpath是相对路径，且olddirfd的值为AT_FDCWD，则它是相对于当前路径而言的。如果oldpath是绝对路径，则olddirfd被忽略。
/// - newdirfd：新文件名所在的目录。
/// - newpath：文件的新名字。newpath的使用规则同oldpath。
/// - flags：在2.6.18内核之前，应置为0。其它的值详见`man 2 linkat`。
///
/// 返回值：成功执行，返回0。失败，返回-1。
pub fn sys_linkat(fd: isize, filename: *const u8, flags: isize, mode: usize) -> isize {
    todo!()
}
