//! 内存管理系统调用

use crate::{
    consts::PAGE_SIZE,
    mm::{MapPermission, MmapFlags, MmapProts, PTEFlags, PageTable, VPNRange, VirtAddr},
    task::{current_task, current_user_token},
};

use super::super::error::*;

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
pub fn sys_brk(brk: usize) -> Result<isize> {
    let task = current_task().unwrap();
    if brk == 0 {
        Ok(task.grow_proc(0) as isize)
    } else {
        let former_addr = task.grow_proc(0);
        let grow_size: isize = (brk - former_addr) as isize;
        Ok(current_task().unwrap().grow_proc(grow_size) as isize)
    }
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
pub fn sys_munmap(addr: usize, length: usize) -> Result<isize> {
    let task = current_task().unwrap();
    Ok(task.munmap(addr, length))
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
) -> Result<isize> {
    // println!(
    //     "[DEBUG] addr:{:?},length:{:?},prot:{:?},flags:{:?},fd:{:?},offset:{:?}",
    //     addr, length, prot, flags, fd, offset
    // );
    // {
    //     info!("#### debug sys_mmap ####");
    //     info!("fd: {}", fd);
    //     info!("prot: {}", prot);
    //     info!("addr: {:#x}", addr);
    //     info!("length: {:#x}", length);
    // }
    if length == 0 {
        return Err(SyscallError::MmapLengthNotBigEnough(-1));
    }

    let length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1); // 向上取整

    let task = current_task().unwrap();
    let inner = task.read();
    let prot = MmapProts::from_bits(prot).expect("unsupported mmap prot");
    let flags = MmapFlags::from_bits(flags).expect("unsupported mmap flags");

    if !flags.contains(MmapFlags::MAP_ANONYMOUS)
        && (fd as usize >= inner.fd_table.len() || inner.fd_table[fd as usize].is_none())
    {
        // {
        //     info!("#### debug sys_mmap ####");
        //     info!("fd: {}", fd);
        // }
        return Err(SyscallError::FdInvalid(-1, fd as usize));
    }
    // if fd as usize >= inner.fd_table.len() || inner.fd_table[fd as usize].is_none() {
    //     return Err(SyscallError::FdInvalid(-1, fd as usize));
    // }
    drop(inner);

    // {
    //     let map_flags = (((prot.bits() & 0b111) << 1) + (1 << 4)) as u16;
    //     let map_perm = MapPermission::from_bits(map_flags).unwrap();
    //     info!("#### debug sys_mmap ###");
    //     info!("prot: {:?}", prot.bits());
    //     info!("map_flags: {:#x}", map_flags);
    //     info!("map_perm: {:?}", map_perm.bits());
    //     info!("map_perm is readable: {}", map_perm.readable());
    //     info!("map_perm is writable: {}", map_perm.writable());
    //     info!("map_perm is executable: {}", map_perm.executable());
    // }
    let result_addr = task.mmap(addr, length, prot, flags, fd, offset);

    // {
    //     info!("#### debug sys_mmap ####");
    //     info!("result_addr: {:#x}", result_addr);
    // }

    Ok(result_addr as isize)
}

// #define SYS_mprotect 226
//
// 功能：修改内存区域的保护方式；
// 输入：指定内存区域的起始地址，长度，保护方式；
// 返回值：成功返回0
//        失败可能的返回值：EACCES，EINVAL，ENOMEM
// ```c
// void *addr, size_t len, int prot
// int ret = syscall(SYS_mprotect, addr, len, prot);
// ```
pub fn sys_mprotect(addr: usize, length: usize, prot: usize) -> Result<isize> {
    let token = current_user_token();
    let page_table = PageTable::from_token(token);

    let map_flags = (((prot & 0b111) << 1) + (1 << 4)) as u16;
    let map_perm = MapPermission::from_bits(map_flags).unwrap();
    let pte_flags = PTEFlags::from_bits(map_perm.bits()).unwrap();

    let start_va = VirtAddr::from(addr);
    let end_va = VirtAddr::from(addr + length);
    let vpn_range = VPNRange::from_va(start_va, end_va);

    // NOTE: 未修改 vm_area 的 map_perm
    for vpn in vpn_range {
        if let Some(pte) = page_table.find_pte(vpn) {
            pte.set_flags(pte_flags);
        } else {
            let va: VirtAddr = vpn.into();
            return Err(SyscallError::InvalidVirtAddress(-1, va));
        }
    }

    Ok(0)
}
