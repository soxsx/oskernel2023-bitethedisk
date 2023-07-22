use alloc::{collections::BTreeMap, sync::Arc};

use crate::{
    consts::PAGE_SIZE,
    error::Error,
    fs::file::File,
    mm::{
        address::Step, alloc_frame, page_table::PTEFlags, FrameTracker, PageTable, PhysPageNum,
        VPNRange, VirtAddr, VirtPageNum,
    },
};

use super::{MapPermission, MapType};

pub struct VmArea {
    pub vpn_range: VPNRange,
    pub map_type: MapType,
    pub permission: MapPermission,

    pub file: Option<Arc<dyn File>>, // 被映射的文件
    pub file_offset: usize,          // 被映射的文件在文件中的偏移量

    pub frame_map: BTreeMap<VirtPageNum, FrameTracker>, // vpn -> frame_tracker
                                                        // pub start_va: VirtAddr,                                 // 该段的起始虚拟地址
                                                        // pub end_va: VirtAddr,                                   // 该段的结束虚拟地址
}

impl VmArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        permission: MapPermission,
        file: Option<Arc<dyn File>>,
        file_offset: usize,
    ) -> Self {
        Self {
            vpn_range: VPNRange::from_va(start_va, end_va),
            map_type,
            permission,
            frame_map: BTreeMap::new(),
            file,
            file_offset,
        }
    }

    pub fn from_another(another: &Self) -> Self {
        Self {
            vpn_range: another.vpn_range,
            frame_map: BTreeMap::new(),
            map_type: another.map_type,
            permission: another.permission,
            file: another.file.clone(),
            file_offset: another.file_offset,
        }
    }

    pub fn start_vpn(&self) -> VirtPageNum {
        self.vpn_range.get_start()
    }

    pub fn end_vpn(&self) -> VirtPageNum {
        self.vpn_range.get_end()
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
                    self.frame_map.insert(vpn, frame);
                    let flags = PTEFlags::from_bits(self.permission.bits()).unwrap();
                    page_table.map(vpn, ppn, flags);
                });
            }
        }
    }

    /// 将当前逻辑段到物理内存的映射从传入的该逻辑段所属的地址空间的多级页表中删除
    /// 写回文件(如果 map_perm 包含 W)
    pub fn write_back(&self, page_table: &mut PageTable) -> Result<(), Error> {
        if !self.permission.contains(MapPermission::W) {
            return Ok(());
        }
        if self.file.is_none() {
            return Ok(());
        }
        let file = self.file.as_ref().unwrap();
        if !file.writable() {
            return Ok(());
        }
        for vpn in self.vpn_range {
            match page_table.translate(vpn) {
                None => {}
                Some(pte) => {
                    if !pte.is_valid() {
                        continue;
                    }
                    let data = pte.ppn().as_bytes_array();
                    let offset =
                        (vpn.0 - self.vpn_range.get_start().0) * PAGE_SIZE + self.file_offset;
                    file.seek(offset);
                    file.write_kernel_space(data.to_vec());
                }
            }
        }
        Ok(())
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
        data_start: usize,
        mut data_len: usize,
        mut page_offset: usize,
    ) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut offset: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let file = self.file.as_ref().unwrap();
        loop {
            let data = file.read_to_vec((data_start + offset) as isize, data_len.min(PAGE_SIZE));
            let data_slice = data.as_slice();

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

    pub fn lazy_map_vpn(&mut self, vpn: VirtPageNum, page_table: &mut PageTable) {
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
                    self.frame_map.insert(vpn, frame);
                } else {
                    panic!("No more memory!");
                }
            }
        }
        let pte_flags = PTEFlags::from_bits(self.permission.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
}
