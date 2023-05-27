use alloc::{sync::Arc, vec::Vec};

use crate::{
    consts::PAGE_SIZE,
    fs::OSInode,
    mm::{
        address::VPNRange, alloc_frame, page_table::PTEFlags, FrameTracker, PageTable, PhysPageNum,
        StepByOne, VirtAddr, VirtPageNum,
    },
};

use super::map_flags::{MapPermission, MapType};

pub struct MapArea {
    /// 描述一段虚拟页号的连续区间，表示该逻辑段在地址区间中的位置和长度
    pub(super) vpn_range: VPNRange,
    /// 键值对容器 BTreeMap ,保存了该逻辑段内的每个虚拟页面的 VPN 和被映射到的物理页帧<br>
    /// 这些物理页帧被用来存放实际内存数据而不是作为多级页表中的中间节点
    pub(super) data_frames: Vec<FrameTracker>,
    /// 描述该逻辑段内的所有虚拟页面映射到物理页帧的方式
    pub(super) map_type: MapType,
    /// 控制该逻辑段的访问方式，它是页表项标志位 PTEFlags 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
    pub(super) map_perm: MapPermission,
}

impl MapArea {
    /// 根据起始 *(会被下取整)* 和终止 *(会被上取整)* 虚拟地址生成一块逻辑段
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

    /// 在多级页表中根据vpn分配空间
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                // 获取一个物理页帧
                let frame = alloc_frame().expect("out of memory");
                ppn = frame.ppn;
                // 将物理页帧生命周期捆绑到data_frames中，从而进程结束时可以自动释放
                self.data_frames.push(frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        // 在多级页表中建立映射
        page_table.map(vpn, ppn, pte_flags);
    }

    /// 在多级页表中为逻辑块分配空间
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
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
