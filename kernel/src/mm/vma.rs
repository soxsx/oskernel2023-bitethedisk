use super::address::VirtAddr;
use super::{translated_bytes_buffer, FrameTracker, UserBuffer, VPNRange, VirtPageNum};
use crate::consts::PAGE_SIZE;
use crate::fs::file::File;
use crate::mm::PageTable;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
bitflags! {
#[derive(Clone, Copy, Debug)]
    pub struct MmapProts: usize {
    // TODO do not use 0
        const PROT_NONE = 0;  // 不可读不可写不可执行，用于实现防范攻击的guard page等
        const PROT_READ = 1 <<0;
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
#[derive(Clone, Copy, Debug)]
    pub struct MmapFlags: usize {
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
//  TODO 管理上有缺陷: 内存碎片问题
#[derive(Clone)]
pub struct MmapManager {
    pub mmap_start: VirtAddr,
    pub mmap_top: VirtAddr,
    pub mmap_map: BTreeMap<VirtPageNum, MmapPage>,
    pub frame_trackers: BTreeMap<VirtPageNum, FrameTracker>,
}

impl MmapManager {
    pub fn new(mmap_start: VirtAddr, mmap_top: VirtAddr) -> Self {
        Self {
            mmap_start,
            mmap_top,
            mmap_map: BTreeMap::new(),
            frame_trackers: BTreeMap::new(),
        }
    }

    pub fn get_mmap_top(&mut self) -> VirtAddr {
        self.mmap_top
    }

    // pub fn lazy_map_page(
    //     &mut self,
    //     vpn: VirtPageNum,
    //     token:usize,
    // ) {
    // 	if let Some(mmap_page)=self.mmap_map.get(&vpn){
    // 	    mmap_page.lazy_map_page(token);
    // 	}
    // }

    pub fn push(
        &mut self,
        start_va: VirtAddr,
        len: usize,
        prot: MmapProts,
        flags: MmapFlags,
        offset: usize,
        file: Option<Arc<dyn File>>,
    ) -> usize {
        let end = VirtAddr(start_va.0 + len);

        // use lazy map
        let mut offset = offset;
        for vpn in VPNRange::from_va(start_va, end) {
            // println!("[DEBUG] mmap map vpn:{:x?}",vpn);
            let mmap_page = MmapPage::new(vpn, prot, flags, false, file.clone(), offset);
            self.mmap_map.insert(vpn, mmap_page);
            offset += PAGE_SIZE;
        }
        // update mmap_top
        // if self.mmap_top == start_addr {
        if self.mmap_top <= start_va {
            self.mmap_top = (start_va.0 + len).into();
        }
        start_va.0
    }

    pub fn remove(&mut self, start_va: VirtAddr, len: usize) {
        let end_va = VirtAddr(start_va.0 + len);
        for vpn in VPNRange::from_va(start_va, end_va) {
            self.mmap_map.remove(&vpn);
            self.frame_trackers.remove(&vpn);
        }
    }
    // pub fn remove_one_page(&mut self, vpn: VirtPageNum){
    // 	self.mmap_map.remove(&vpn);
    // }
}

/// mmap 块
///
/// 用于记录 mmap 空间信息，mmap数据并不存放在此
#[derive(Clone)]
pub struct MmapPage {
    /// mmap 空间起始虚拟地址
    pub vpn: VirtPageNum,

    /// mmap 空间有效性
    pub valid: bool,

    /// mmap 空间权限
    pub prot: MmapProts,

    /// 映射方式
    pub flags: MmapFlags,

    /// 文件描述符
    pub file: Option<Arc<dyn File>>,

    /// 映射文件偏移地址
    pub offset: usize,
}

impl MmapPage {
    pub fn new(
        vpn: VirtPageNum,
        prot: MmapProts,
        flags: MmapFlags,
        valid: bool,
        file: Option<Arc<dyn File>>,
        offset: usize,
    ) -> Self {
        Self {
            vpn,
            prot,
            flags,
            valid,
            file,
            offset,
        }
    }
    pub fn lazy_map_page(&mut self, token: usize) {
        if self.flags.contains(MmapFlags::MAP_ANONYMOUS) {
            self.read_from_zero(token);
        } else {
            self.read_from_file(token);
        }
        self.valid = true;
    }
    fn read_from_file(&mut self, token: usize) {
        let f = self.file.clone().unwrap();
        let old_offset = f.offset();
        f.seek(self.offset);
        if !f.readable() {
            return;
        }
        let _read_len = f.read(UserBuffer::wrap(translated_bytes_buffer(
            token,
            VirtAddr::from(self.vpn).0 as *const u8,
            PAGE_SIZE,
        )));
        f.seek(old_offset);
        return;
    }
    fn read_from_zero(&mut self, token: usize) {
        UserBuffer::wrap(translated_bytes_buffer(
            token,
            VirtAddr::from(self.vpn).0 as *const u8,
            PAGE_SIZE,
        ))
        .write_zeros();
    }
    pub fn write_back(&mut self, token: usize) {
        let f = self.file.clone().unwrap();
        let old_offset = f.offset();
        f.seek(self.offset);
        if !f.writable() {
            return;
        }
        let _read_len = f.write(UserBuffer::wrap(translated_bytes_buffer(
            token,
            VirtAddr::from(self.vpn).0 as *const u8,
            PAGE_SIZE,
        )));
        f.seek(old_offset);
        return;
    }
    pub fn set_prot(&mut self, new_prot: MmapProts) {
        self.prot = new_prot;
    }
}
