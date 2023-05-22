//! 内存管理系统调用

use crate::{
    mm::{translated_bytes_buffer, MmapFlags, MmapProts, UserBuffer},
    task::{current_task, current_user_token},
};

/// #define SYS_brk 214
///
/// 功能：修改数据段的大小；
///
/// 输入：指定待修改的地址；
///
/// 返回值：成功返回0，失败返回-1;
///
/// ```c
/// uintptr_t brk;
/// uintptr_t ret = syscall(SYS_brk, brk);
/// ```
pub fn sys_brk(brk_addr: usize) -> isize {
    let mut addr_new = 0;
    _ = addr_new;
    if brk_addr == 0 {
        addr_new = {
            let _is_shrink = 0;
            current_task().unwrap().grow_proc(0)
        };
    } else {
        let former_addr = current_task().unwrap().grow_proc(0);
        let grow_size: isize = (brk_addr - former_addr) as isize;
        addr_new = current_task().unwrap().grow_proc(grow_size);
    }

    addr_new as isize
}

/// #define SYS_munmap 215
///
/// 功能：将文件或设备取消映射到内存中；
///
/// 输入：映射的指定地址及区间；
///
/// 返回值：成功返回0，失败返回-1;
///
/// ```c
/// void *start, size_t len
/// int ret = syscall(SYS_munmap, start, len);
/// ```
pub fn sys_munmap(addr: usize, length: usize) -> isize {
    let task = current_task().unwrap();

    task.munmap(addr, length)
}

/// #define SYS_mmap 222
///
/// 功能：将文件或设备映射到内存中；
///
/// 输入：
///
/// - start: 映射起始位置，
/// - len: 长度，
/// - prot: 映射的内存保护方式，可取：PROT_EXEC, PROT_READ, PROT_WRITE, PROT_NONE
/// - flags: 映射是否与其他进程共享的标志，
/// - fd: 文件句柄，
/// - off: 文件偏移量；
///
/// 返回值：成功返回已映射区域的指针，失败返回-1;
///
/// ```c
/// void *start, size_t len, int prot, int flags, int fd, off_t off
/// long ret = syscall(SYS_mmap, start, len, prot, flags, fd, off);
/// ```
pub fn sys_mmap(
    addr: usize,
    length: usize,
    prot: usize,
    flags: usize,
    fd: isize,
    offset: usize,
) -> isize {
    if length == 0 {
        panic!("mmap:len == 0");
    }

    let prot = MmapProts::from_bits(prot).expect("unsupported mmap prot");
    let flags = MmapFlags::from_bits(flags).expect("unsupported mmap flags");

    let task = current_task().unwrap();
    let result_addr = task.mmap(addr, length, prot, flags, fd, offset);

    // crate::debug!("result_addr: 0x{:X}", result_addr);
    // let buf = translated_bytes_buffer(current_user_token(), result_addr as *const u8, length);
    // let content = buf
    //     .iter()
    //     .map(|x| unsafe { core::str::from_utf8_unchecked(x) })
    //     .collect::<alloc::string::String>();
    // debug!("content: {}", content);

    result_addr as isize
}
