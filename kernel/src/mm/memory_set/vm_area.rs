use alloc::{sync::Arc, vec::Vec};

use crate::{
    consts::PAGE_SIZE,
    fs::OSInode,
    mm::{
        address::Step, alloc_frame, page_table::PTEFlags, FrameTracker, PageTable, PhysPageNum,
        VPNRange, VirtAddr, VirtPageNum,
    },
};

use super::{MapPermission, MapType};

pub struct VmArea {
    pub vpn_range: VPNRange,

    pub vpn_table: Vec<VirtPageNum>,

    pub map_type: MapType,
    pub permission: MapPermission,

    pub frame_trackers: Vec<FrameTracker>,

    // TODO: handle this
    // pub related_file: Option<Arc<dyn File>>,
}

impl VmArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        permission: MapPermission,
    ) -> Self {
        Self {
            vpn_range: VPNRange::from_va(start_va, end_va),
            vpn_table: Vec::new(),
            map_type,
            permission,
            frame_trackers: Vec::new(),
        }
    }

    pub fn from_another(another: &Self) -> Self {
        Self {
            vpn_range: another.vpn_range,
            vpn_table: Vec::new(),
            map_type: another.map_type,
            permission: another.permission,
            frame_trackers: Vec::new(),
        }
    }

    pub fn inflate_pagetable(&mut self, page_table: &mut PageTable) {
        match self.map_type {
            MapType::Identical => {
                self.vpn_range.into_iter().for_each(|vpn| {
                    let ppn = PhysPageNum(vpn.0);
                    let flags = PTEFlags::from_bits(self.permission.bits()).unwrap();
                    page_table.map(vpn, ppn, flags);
                });
            }
            MapType::Framed => {
                self.vpn_range.into_iter().for_each(|vpn| {
                    let frame = alloc_frame().expect("out of memory");
                    let ppn = frame.ppn;
                    self.frame_trackers.push(frame);
                    let flags = PTEFlags::from_bits(self.permission.bits()).unwrap();
                    page_table.map(vpn, ppn, flags);
                });
            }
        }
    }

    /// 将当前逻辑段到物理内存的映射从传入的该逻辑段所属的地址空间的多级页表中删除
    pub fn erase_pagetable(&mut self, page_table: &mut PageTable) {
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
                    self.frame_trackers.push(frame);
                } else {
                    panic!("No more memory!");
                }
            }
        }
        let pte_flags = PTEFlags::from_bits(self.permission.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
}
