use alloc::vec::Vec;

use crate::mm::{
    address::VPNRange, alloc_frame, page_table::PTEFlags, FrameTracker, PageTable, PhysPageNum,
    VirtAddr, VirtPageNum,
};

use super::flags::{MapPermission, MapType};

/// 离散逻辑段
pub struct ChunkArea {
    pub vpn_range: VPNRange,
    pub vpn_table: Vec<VirtPageNum>,
    pub data_frames: Vec<FrameTracker>,
    pub map_type: MapType,
    pub map_perm: MapPermission,
}

impl ChunkArea {
    pub fn new(map_type: MapType, map_perm: MapPermission, start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            vpn_table: Vec::new(),
            data_frames: Vec::new(),
            map_type,
            map_perm,
            vpn_range: VPNRange::new(start.floor(), end.ceil()),
        }
    }

    pub fn from_another(another: &ChunkArea) -> Self {
        Self {
            vpn_table: Vec::new(),
            data_frames: Vec::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            vpn_range: another.vpn_range,
        }
    }

    pub fn push_vpn(&mut self, vpn: VirtPageNum, page_table: &mut PageTable) {
        self.vpn_table.push(vpn);
        self.map_one(page_table, vpn);
    }

    fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
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

    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_table.clone() {
            page_table.unmap(vpn);
        }
    }
}
