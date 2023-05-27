use alloc::vec::Vec;

use crate::mm::{
    alloc_frame, page_table::PTEFlags, FrameTracker, PageTable, PhysPageNum, VirtAddr, VirtPageNum,
};

use super::map_flags::{MapPermission, MapType};

/// 离散逻辑段
pub struct ChunkArea {
    pub(super) vpn_table: Vec<VirtPageNum>,
    pub(super) data_frames: Vec<FrameTracker>,
    pub(super) map_type: MapType,
    pub(super) map_perm: MapPermission,
    pub(super) start_va: VirtAddr,
    pub(super) end_va: VirtAddr,
}

impl ChunkArea {
    pub fn new(map_type: MapType, map_perm: MapPermission, start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            vpn_table: Vec::new(),
            data_frames: Vec::new(),
            map_type,
            map_perm,
            start_va: start,
            end_va: end,
        }
    }

    pub fn set_mmap_range(&mut self, start: VirtAddr, end: VirtAddr) {
        self.start_va = start;
        self.end_va = end;
        for (idx, vpn) in self.vpn_table.clone().iter_mut().enumerate() {
            if VirtAddr::from(*vpn) >= self.end_va {
                self.vpn_table.remove(idx);
                // todo:删除 data_frame 中超范围的物理页帧
            }
        }
    }

    pub fn push_vpn(&mut self, vpn: VirtPageNum, page_table: &mut PageTable) {
        self.vpn_table.push(vpn);
        self.map_one(page_table, vpn);
    }

    pub fn from_another(another: &ChunkArea) -> Self {
        Self {
            vpn_table: Vec::new(),
            data_frames: Vec::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            start_va: another.start_va,
            end_va: another.end_va,
        }
    }

    // Alloc and map one page
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                if let Some(frame) = alloc_frame() {
                    ppn = frame.ppn;
                    self.data_frames.push(frame);
                } else {
                    panic!("No more memory!");
                }
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    // Alloc and map all pages
    // pub fn map(&mut self, page_table: &mut PageTable) {
    //     for vpn in self.vpn_table {
    //         self.map_one(page_table, vpn);
    //     }
    // }

    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_table.clone() {
            page_table.unmap(vpn);
        }
    }
}
