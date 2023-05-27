use alloc::{sync::Arc, vec::Vec};

use crate::{
    consts::PAGE_SIZE,
    fs::OSInode,
    mm::{
        address::{Step, VPNRange},
        alloc_frame,
        page_table::PTEFlags,
        FrameTracker, PageTable, PhysPageNum, VirtAddr, VirtPageNum,
    },
};

use super::map_flags::{MapPermission, MapType};

pub struct MapArea {
    pub vpn_range: VPNRange,

    pub data_frames: Vec<FrameTracker>,

    pub map_type: MapType,

    pub map_perm: MapPermission,
}

impl MapArea {
    /// 根据虚拟地址生成一块逻辑段
    ///
    /// 起始地址和结束地址会按页取整，起始地址向下，结束地址向上
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: Vec::new(),
            map_type,
            map_perm,
        }
    }

    /// 从一个逻辑段复制得到一个虚拟地址区间、映射方式和权限控制均相同的逻辑段
    ///
    /// 不同的是由于它还没有真正被映射到物理页帧上，所以 data_frames 字段为空
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end()),
            data_frames: Vec::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
        }
    }

    /// 在多级页表中为逻辑块分配空间
    pub fn map(&mut self, page_table: &mut PageTable) {
        match self.map_type {
            MapType::Identical => {
                self.vpn_range.into_iter().for_each(|vpn| {
                    let ppn = PhysPageNum(vpn.0);
                    let flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
                    page_table.map(vpn, ppn, flags);
                });
            }
            MapType::Framed => {
                self.vpn_range.into_iter().for_each(|vpn| {
                    let frame = alloc_frame().expect("out of memory");
                    let ppn = frame.ppn;
                    self.data_frames.push(frame);
                    let flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
                    page_table.map(vpn, ppn, flags);
                });
            }
        }
    }

    /// 将当前逻辑段到物理内存的映射从传入的该逻辑段所属的地址空间的多级页表中删除
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            page_table.unmap(vpn);
        }
    }

    pub fn copy_data(
        &mut self,
        page_table: &mut PageTable,
        elf_file: Arc<OSInode>,
        data_start: usize,
        data_len: usize,
        page_offset: usize,
    ) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut offset: usize = 0;
        let mut page_offset: usize = page_offset;
        let mut current_vpn = self.vpn_range.get_start();
        let mut data_len = data_len;
        loop {
            let data;
            let data_slice;

            data = elf_file.read_vec((data_start + offset) as isize, data_len.min(PAGE_SIZE));
            data_slice = data.as_slice();

            let src = &data_slice[0..data_len.min(PAGE_SIZE - page_offset)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .as_bytes_array()[page_offset..page_offset + src.len()];
            dst.copy_from_slice(src);
            offset += PAGE_SIZE - page_offset;

            page_offset = 0;
            data_len -= src.len();
            if data_len == 0 {
                break;
            }
            current_vpn.step();
        }
    }
}
