use super::address::VirtAddr;
use super::{translated_bytes_buffer, FrameTracker, UserBuffer, VPNRange, VirtPageNum};
use crate::consts::PAGE_SIZE;
use crate::fs::File;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use nix::{MmapFlags, MmapProts};

/// mmap 块管理器
///
/// - `mmap_start`: 地址空间中 mmap 区块起始虚地址
/// - `mmap_top`: 地址空间中已用的最高的 mmap 区块的虚地址
/// - `mmap_map`: 虚拟页号 -> mmap页
/// - `frame_trackers`: 虚拟页号 -> 物理页帧
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
}

/// mmap 块
///
/// 用于记录 mmap 空间信息, mmap数据并不存放在此
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
        #[cfg(feature = "time_tracer")]
        time_trace!("mmap_read_from_file");
        let f = self.file.clone().unwrap();
        let old_offset = f.offset();
        f.seek(self.offset);
        if !f.readable() {
            return;
        }
        let file_size = f.file_size();
        let len = PAGE_SIZE.min(file_size - self.offset);
        let _read_len = f.read_to_ubuf(UserBuffer::wrap(translated_bytes_buffer(
            token,
            VirtAddr::from(self.vpn).0 as *const u8,
            len,
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
    #[allow(unused)]
    pub fn write_back(&mut self, token: usize) {
        let f = self.file.clone().unwrap();
        let old_offset = f.offset();
        f.seek(self.offset);
        if !f.writable() {
            return;
        }
        let _read_len = f.write_from_ubuf(UserBuffer::wrap(translated_bytes_buffer(
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
