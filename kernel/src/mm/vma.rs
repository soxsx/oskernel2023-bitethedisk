use super::address::VirtAddr;
use super::{translated_bytes_buffer, UserBuffer};
use crate::consts::PAGE_SIZE;
use crate::fs::file::File;
use alloc::sync::Arc;
use alloc::vec::Vec;

bitflags! {
    pub struct MmapProts: usize {
        const PROT_NONE = 0;  // 不可读不可写不可执行，用于实现防范攻击的guard page等
        const PROT_READ = 1;
        const PROT_WRITE = 1 << 1;
        const PROT_EXEC  = 1 << 2;
        const PROT_GROWSDOWN = 0x01000000;
        const PROT_GROWSUP = 0x02000000;
    }
}

bitflags! {
    /// - MAP_FILE: 文件映射，使用文件内容初始化内存
    /// - MAP_SHARED: 共享映射，修改对所有进程可见，多进程读写同一个文件需要调用者提供互斥机制
    /// - MAP_PRIVATE: 私有映射，进程A的修改对进程B不可见的，利用 COW 机制，修改只会存在于内存中，不会同步到外部的磁盘文件上
    /// - MAP_FIXED: 将mmap空间放在addr指定的内存地址上，若与现有映射页面重叠，则丢弃重叠部分。如果指定的地址不能使用，mmap将失败。
    /// - MAP_ANONYMOU: 匿名映射，初始化全为0的内存空间
    pub struct MmapFlags: usize {
        const MAP_FILE = 0;
        const MAP_SHARED= 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED = 0x10;
        const MAP_ANONYMOUS = 0x20;
    }
}

/// mmap 块管理器
///
/// - `mmap_start` : 地址空间中mmap区块起始虚地址
/// - `mmap_top` : 地址空间中mmap区块当结束虚地址
/// - `mmap_set` : mmap块向量
//  管理上有缺陷: 内存碎片问题
#[derive(Clone, Debug)]
pub struct MmapManager {
    pub mmap_start: VirtAddr,
    pub mmap_top: VirtAddr,
    pub mmap_set: Vec<MmapInfo>,
}

impl MmapManager {
    pub fn new(mmap_start: VirtAddr, mmap_top: VirtAddr) -> Self {
        Self {
            mmap_start,
            mmap_top,
            mmap_set: Vec::new(),
        }
    }

    pub fn get_mmap_top(&mut self) -> VirtAddr {
        self.mmap_top
    }

    pub fn lazy_map_page(
        &mut self,
        va: VirtAddr,
        fd_table: Vec<Option<Arc<dyn File>>>,
        token: usize,
    ) {
        for mmap_space in self.mmap_set.iter_mut() {
            if va.0 >= mmap_space.oaddr.0 && va.0 < mmap_space.oaddr.0 + mmap_space.length {
                mmap_space.lazy_map_page(va, fd_table, token);
                return;
            }
        }
    }

    pub fn push(
        &mut self,
        start: usize,
        len: usize,
        prot: usize,
        flags: usize,
        fd: isize,
        offset: usize,
        _fd_table: Vec<Option<Arc<dyn File>>>,
        _token: usize,
    ) -> usize {
        let start_addr = start.into();

        let mmap_space = MmapInfo::new(start_addr, len, prot, flags, 0, fd, offset);

        // use lazy map
        self.mmap_set.push(mmap_space);

        // update mmap_top
        // if self.mmap_top == start_addr {
        if self.mmap_top <= start_addr {
            self.mmap_top = (start_addr.0 + len).into();
        }

        start_addr.0
    }

    pub fn remove(&mut self, start: usize, len: usize) -> isize {
        let pair = self
            .mmap_set
            .iter()
            .enumerate()
            .find(|(_, p)| p.oaddr.0 == start);
        if let Some((idx, _)) = pair {
            if self.mmap_top == VirtAddr::from(start + len) {
                self.mmap_top = VirtAddr::from(start);
            }
            self.mmap_set.remove(idx);
            0
        } else {
            panic! {"No matched Mmap Space!"}
        }
    }
}

/// mmap 块
///
/// 用于记录 mmap 空间信息，mmap数据并不存放在此
#[derive(Clone, Copy, Debug)]
pub struct MmapInfo {
    /// mmap 空间起始虚拟地址
    pub oaddr: VirtAddr,

    /// mmap 空间有效性
    pub valid: usize,

    /// mmap 空间长度
    pub length: usize,

    /// mmap 空间权限
    pub prot: usize,

    /// 映射方式
    pub flags: usize,

    /// 文件描述符
    pub fd: isize,

    /// 映射文件偏移地址
    pub offset: usize,
}

impl MmapInfo {
    pub fn new(
        oaddr: VirtAddr,
        length: usize,
        prot: usize,
        flags: usize,
        valid: usize,
        fd: isize,
        offset: usize,
    ) -> Self {
        Self {
            oaddr,
            length,
            prot,
            flags,
            valid,
            fd,
            offset,
        }
    }

    pub fn lazy_map_page(
        &mut self,
        va: VirtAddr,
        fd_table: Vec<Option<Arc<dyn File>>>,
        token: usize,
    ) {
        let offset: usize = self.offset - self.oaddr.0 + va.0;
        self.map_file(va, PAGE_SIZE, offset, fd_table, token);
    }

    pub fn map_file(
        &mut self,
        va_start: VirtAddr,
        len: usize,
        offset: usize,
        fd_table: Vec<Option<Arc<dyn File>>>,
        token: usize,
    ) {
        let flags = MmapFlags::from_bits(self.flags).unwrap();
        if flags.contains(MmapFlags::MAP_ANONYMOUS) && self.fd == -1 && offset == 0 {
            return;
        }

        if self.fd as usize >= fd_table.len() {
            return;
        }

        if let Some(file) = &fd_table[self.fd as usize] {
            let f = file.clone();
            f.seek(offset);
            if !f.readable() {
                return;
            }
            let _read_len = f.read(UserBuffer::wrap(translated_bytes_buffer(
                token,
                va_start.0 as *const u8,
                len,
            )));
        } else {
            return;
        };
        return;
    }
}
