#![allow(unused)]

use super::vm_area::VmArea;
use super::{MapPermission, MapType};
use crate::consts::{
    CLOCK_FREQ, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_HEAP_SIZE, USER_STACK_SIZE,
};
use crate::fs::file::File;
use crate::mm::frame_allocator::enquire_refcount;
use crate::mm::page_table::PTEFlags;
use crate::mm::{
    alloc_frame, FrameTracker, MmapManager, PageTable, PageTableEntry, PhysAddr, PhysPageNum,
    VirtAddr, VirtPageNum,
};
use alloc::collections::BTreeMap;
use alloc::{sync::Arc, vec::Vec};

use crate::fs::open_flags::CreateMode;
use crate::fs::{open, AbsolutePath, OpenFlags};
use crate::mm::shared_memory::{
    shm_get_address_and_size, shm_get_nattch, SharedMemoryArea, SharedMemoryTracker,
};
pub const MMAP_BASE: usize = 0x60000000;
pub const MMAP_END: usize = 0x68000000; // mmap 区大小为 128 MiB
pub const SHM_BASE: usize = 0x70000000;
pub const LINK_BASE: usize = 0x20000000;

#[derive(Clone, Copy, Debug)]
pub struct AuxEntry(pub usize, pub usize);

pub const AT_NULL: usize = 0;
pub const AT_PHDR: usize = 3;
pub const AT_PHENT: usize = 4;
pub const AT_PHNUM: usize = 5;
pub const AT_PAGESZ: usize = 6;
pub const AT_BASE: usize = 7;
pub const AT_FLAGS: usize = 8;
pub const AT_ENTRY: usize = 9;
pub const AT_UID: usize = 11;
pub const AT_EUID: usize = 12;
pub const AT_GID: usize = 13;
pub const AT_EGID: usize = 14;
pub const AT_HWCAP: usize = 16;
pub const AT_CLKTCK: usize = 17;
pub const AT_SECURE: usize = 23;
pub const AT_RANDOM: usize = 25;
pub const AT_EXECFN: usize = 31;

pub fn new() -> Vec<AuxEntry> {
    let mut temp = Vec::new();
    temp.push(AuxEntry(AT_NULL, 0));
    temp.push(AuxEntry(AT_PAGESZ, PAGE_SIZE));
    temp.push(AuxEntry(AT_UID, 0));
    temp.push(AuxEntry(AT_EUID, 0));
    temp.push(AuxEntry(AT_GID, 0));
    temp.push(AuxEntry(AT_EGID, 0));
    temp.push(AuxEntry(AT_SECURE, 0));
    temp
}

/// 虚拟地址空间抽象
///
/// 比如，用户进程的虚拟地址空间抽象:
///
/// ```text
/// +--------------------+
/// |     trampoline     |
/// +--------------------+
/// |      trap_cx       |
/// +--------------------+
/// |     Guard Page     | <-- 保护页
/// +--------------------+
/// |                    |
/// |     User Stack     | <-- 用户虚拟地址空间(U-mode)中的用户栈
/// |                    |
/// +--------------------+
/// |       Unused       |
/// +--------------------+
/// |                    |
/// |     mmap Areas     | <-- mmap 区
/// |                    |
/// +--------------------+
/// |                    |
/// |        ...         |
/// |                    |
/// +--------------------+ <-- brk
/// |                    |
/// |                    |
/// |     User Heap      |
/// |                    |
/// |                    |
/// +--------------------+ <-- brk_start
/// |                    |
/// |    Data Segments   | <-- ELF 文件加载后所有 Segment 的集合
/// |                    |
/// +--------------------+
/// ```
pub struct MemorySet {
    pub page_table: PageTable,

    vm_areas: Vec<VmArea>,

    pub mmap_manager: MmapManager,

    heap_areas: VmArea,

    shm_areas: Vec<VmArea>,

    shm_trackers: BTreeMap<VirtAddr, SharedMemoryTracker>,
    pub shm_top: usize,

    pub brk_start: usize,
    pub brk: usize,
}

impl MemorySet {
    /// 新建一个空的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            vm_areas: Vec::new(),
            heap_areas: VmArea::new(
                0.into(),
                0.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
                None,
                0,
            ),
            mmap_manager: MmapManager::new(VirtAddr::from(MMAP_BASE), VirtAddr::from(MMAP_BASE)),
            shm_areas: Vec::new(),
            shm_trackers: BTreeMap::new(),
            shm_top: SHM_BASE,
            brk_start: 0,
            brk: 0,
        }
    }

    /// 获取当前页表的 token (符合 satp CSR 格式要求的多级页表的根节点所在的物理页号)
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// 在当前地址空间插入一个 `Framed` 方式映射到物理内存的逻辑段
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.insert(
            VmArea::new(start_va, end_va, MapType::Framed, permission, None, 0),
            None,
        );
    }

    /// 通过起始虚拟页号删除对应的逻辑段（包括连续逻辑段和离散逻辑段）
    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, vm_area)) = self
            .vm_areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            vm_area.erase_pagetable(&mut self.page_table);
            self.vm_areas.remove(idx);
        }
    }

    /// 在当前地址空间插入一个新的连续逻辑段
    ///
    /// - 物理页号是随机分配的
    /// - 如果是以 Framed 方式映射到物理内存,
    /// 还可以可选性地在那些被映射到的物理页帧上写入一些初始化数据
    /// - data:(osinode,offset,len,page_offset)
    pub fn insert(&mut self, mut map_area: VmArea, data: Option<(usize, usize, usize)>) {
        map_area.inflate_pagetable(&mut self.page_table);
        if let Some(data) = data {
            // 写入初始化数据，如果数据存在
            map_area.copy_data(&mut self.page_table, data.0, data.1, data.2);
        }
        self.vm_areas.push(map_area); // 将生成的数据段压入 areas 使其生命周期由areas控制
    }

    /// 在当前地址空间插入一段已被分配空间的连续逻辑段
    ///
    /// 主要用于 COW 创建时子进程空间连续逻辑段的插入，其要求指定物理页号
    fn push_mapped_area(&mut self, map_area: VmArea) {
        self.vm_areas.push(map_area);
    }

    /// 映射跳板的虚拟页号和物理物理页号
    pub fn map_trampoline(&mut self) {
        extern "C" {
            fn strampoline();
        }
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    fn map_trap_context(&mut self) {
        self.insert(
            VmArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
                None,
                0,
            ),
            None,
        );
    }

    /// 中 ELF 文件中构建出一个 [`MemorySet`]
    ///
    /// *因为我们是加载 ELF 文件，所以我们只关心执行视图(Execution View)*
    ///
    /// *需要注意的是，无论是什么视图，对于 ELF 文件来说只是划分标准不同而已，布局基本没有差别*
    ///
    /// ELF 文件在 x64 的布局(Execution View):
    ///
    /// ```text
    /// +----------------------+
    /// |    Program Header    | <-- ELF Header 包含了 ELF 文件的各个段的长度等基本信息等
    /// +----------------------+
    /// | Program Header Table | <-- ph_* 包含了运行时加载所需的基本信息
    /// +----------------------+
    /// |       Segment1       |
    /// +----------------------+
    /// |       Segment2       |
    /// +----------------------+
    /// |         ...          |
    /// +----------------------+
    /// | Section Header Table |
    /// |      (Optional)      |
    /// +----------------------+
    ///
    /// ```
    ///
    /// 当前实现中，ELF 加载后在内存中的布局(进程的地址空间布局):
    ///
    /// ```text
    /// +--------------------+
    /// |     trampoline     |
    /// +--------------------+
    /// |      trap_cx       |
    /// +--------------------+
    /// |     Guard Page     | <-- 保护页
    /// +--------------------+
    /// |                    |
    /// |     User Stack     | <-- 用户虚拟地址空间(U-mode)中的用户栈
    /// |                    |
    /// +--------------------+
    /// |     Guard Page     |
    /// +--------------------+
    /// |                    |
    /// |     mmap Areas     | <-- mmap 区
    /// |                    |
    /// +--------------------+
    /// |                    |
    /// |        ...         |
    /// |                    |
    /// +--------------------+ <-- brk
    /// |                    |
    /// |                    | <-- brk
    /// |                    | <-- brk
    /// |     User Heap      |
    /// |                    |
    /// |                    |
    /// +--------------------+ <-- brk_start
    /// |                    |
    /// |    Data Segments   | <-- ELF 文件加载后所有 Segment 的集合
    /// |                    |
    /// +--------------------+
    /// ```
    pub fn load_elf(elf_file: Arc<dyn File>) -> LoadedELF {
        let mut memory_set = Self::new_bare();
        let mut auxs = Vec::new();

        memory_set.map_trampoline();
        memory_set.map_trap_context();

        // 第一次读取前64字节确定程序表的位置与大小
        let elf_head_data = elf_file.read_to_vec(0, 64);
        let elf_head_data_slice = elf_head_data.as_slice();
        let elf = xmas_elf::ElfFile::new(elf_head_data_slice).unwrap();

        let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
        let ph_offset = elf.header.pt2.ph_offset() as usize;
        let ph_count = elf.header.pt2.ph_count() as usize;

        // 进行第二次读取，这样的elf对象才能正确解析程序段头的信息
        let elf_head_data = elf_file.read_to_vec(0, ph_offset + ph_count * ph_entry_size);
        let elf = xmas_elf::ElfFile::new(elf_head_data.as_slice()).unwrap();

        let mut head_va = None; // top va of ELF which points to ELF header

        // 记录目前涉及到的最大的虚拟地址
        let mut brk_start_va = VirtAddr(0);
        let mut dynamic_link = false;
        let mut entry_point = elf.header.pt2.entry_point() as usize;
        // 遍历程序段进行加载
        for i in 0..ph_count as u16 {
            let ph = elf.program_header(i).unwrap();
            let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
            let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
            // println!(
            //     "[DEBUG] start:0x{:x?},end:0x{:x?},type:{:?}",
            //     start_va,
            //     end_va,
            //     ph.get_type().unwrap()
            // );
            match ph.get_type().unwrap() {
                xmas_elf::program::Type::Load => {
                    let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                    let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                    if head_va.is_none() {
                        head_va = Some(start_va.0);
                    }
                    let mut map_perm = MapPermission::U;
                    let ph_flags = ph.flags();
                    if ph_flags.is_read() {
                        map_perm |= MapPermission::R;
                    }
                    if ph_flags.is_write() {
                        map_perm |= MapPermission::W;
                    }
                    if ph_flags.is_execute() {
                        map_perm |= MapPermission::X;
                    }
                    // println!("[DEBUG] rwx:{:?},{:?},{:?}",ph_flags.is_read(),ph_flags.is_write(),ph_flags.is_execute());
                    let map_area = VmArea::new(
                        start_va,
                        end_va,
                        MapType::Framed,
                        map_perm,
                        Some(Arc::clone(&elf_file)),
                        start_va.page_offset(),
                    );
                    brk_start_va = end_va;
                    memory_set.insert(
                        map_area,
                        Some((
                            ph.offset() as usize,
                            ph.file_size() as usize,
                            start_va.page_offset(),
                        )),
                    );
                }
                xmas_elf::program::Type::Phdr => {
                    // auxs.push(AuxEntry(AT_PHDR, ph.virtual_addr() as usize));
                }
                xmas_elf::program::Type::Interp => {
                    // println!("elf Interp");
                    dynamic_link = true;
                }
                _ => {
                    let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                    let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                    // println!(
                    //     "TYPE:{:?} start_va:{:?} end_va{:?}",
                    //     ph.get_type().unwrap(),
                    //     start_va,
                    //     end_va
                    // );
                }
            }
        }
        if dynamic_link {
            let path = AbsolutePath::from_str("/libc.so");
            let interpreter_file = open(path, OpenFlags::O_RDONLY, CreateMode::empty())
                .expect("can't find interpreter file");
            // 第一次读取前64字节确定程序表的位置与大小
            let interpreter_head_data = interpreter_file.read_to_vec(0, 64);
            let interp_elf = xmas_elf::ElfFile::new(interpreter_head_data.as_slice()).unwrap();

            let ph_entry_size = interp_elf.header.pt2.ph_entry_size() as usize;
            let ph_offset = interp_elf.header.pt2.ph_offset() as usize;
            let ph_count = interp_elf.header.pt2.ph_count() as usize;

            // 进行第二次读取，这样的elf对象才能正确解析程序段头的信息
            let interpreter_head_data =
                interpreter_file.read_to_vec(0, ph_offset + ph_count * ph_entry_size);
            let interp_elf = xmas_elf::ElfFile::new(interpreter_head_data.as_slice()).unwrap();
            auxs.push(AuxEntry(AT_BASE, LINK_BASE));
            entry_point = LINK_BASE + interp_elf.header.pt2.entry_point() as usize;
            // 获取 program header 的数目
            let ph_count = interp_elf.header.pt2.ph_count();
            for i in 0..ph_count {
                let ph = interp_elf.program_header(i).unwrap();
                if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                    let start_va: VirtAddr = (ph.virtual_addr() as usize + LINK_BASE).into();
                    let end_va: VirtAddr =
                        (ph.virtual_addr() as usize + ph.mem_size() as usize + LINK_BASE).into();
                    // println!("DYNAMIC LOAD start_va:{:x?},end_va:{:x?}", start_va, end_va);
                    let map_perm =
                        MapPermission::U | MapPermission::R | MapPermission::W | MapPermission::X;
                    let map_area = VmArea::new(
                        start_va,
                        end_va,
                        MapType::Framed,
                        map_perm,
                        Some(interpreter_file.clone()),
                        0,
                    );
                    memory_set.insert(
                        map_area,
                        Some((
                            ph.offset() as usize,
                            ph.file_size() as usize,
                            start_va.page_offset(),
                        )),
                    );
                }
            }
        } else {
            auxs.push(AuxEntry(AT_BASE, 0));
        }
        let user_stack_top = TRAP_CONTEXT - PAGE_SIZE;
        let user_stack_bottom = user_stack_top - USER_STACK_SIZE;

        // auxs.push(AuxEntry(AT_BASE, 0));

        let ph_head_addr = head_va.unwrap() + elf.header.pt2.ph_offset() as usize;
        // let ph_head_addr = elf.header.pt2.ph_offset() as usize;

        /* get auxv vector */
        auxs.push(AuxEntry(0x21, 0 as usize)); //no vdso
        auxs.push(AuxEntry(0x28, 0 as usize)); //AT_L1I_CACHESIZE:     0
        auxs.push(AuxEntry(0x29, 0 as usize)); //AT_L1I_CACHEGEOMETRY: 0x0
        auxs.push(AuxEntry(0x2a, 0 as usize)); //AT_L1D_CACHESIZE:     0
        auxs.push(AuxEntry(0x2b, 0 as usize)); //AT_L1D_CACHEGEOMETRY: 0x0
        auxs.push(AuxEntry(0x2c, 0 as usize)); //AT_L2_CACHESIZE:      0
        auxs.push(AuxEntry(0x2d, 0 as usize)); //AT_L2_CACHEGEOMETRY:  0x0
        auxs.push(AuxEntry(AT_HWCAP, 0 as usize));
        auxs.push(AuxEntry(AT_PAGESZ, PAGE_SIZE as usize));
        auxs.push(AuxEntry(AT_CLKTCK, CLOCK_FREQ as usize));
        auxs.push(AuxEntry(AT_PHDR, (ph_head_addr as usize)));
        auxs.push(AuxEntry(AT_PHENT, elf.header.pt2.ph_entry_size() as usize)); // ELF64 header 64bytes
        auxs.push(AuxEntry(AT_PHNUM, ph_count as usize));
        // Interp
        // auxs.push(AuxEntry( AT_BASE, 0));
        auxs.push(AuxEntry(AT_FLAGS, 0 as usize));
        auxs.push(AuxEntry(AT_ENTRY, elf.header.pt2.entry_point() as usize));
        auxs.push(AuxEntry(AT_UID, 0 as usize));
        auxs.push(AuxEntry(AT_EUID, 0 as usize));
        auxs.push(AuxEntry(AT_GID, 0 as usize));
        auxs.push(AuxEntry(AT_EGID, 0 as usize));
        auxs.push(AuxEntry(AT_SECURE, 0 as usize));
        // do not add this line, program will run additional check?
        auxs.push(AuxEntry(
            AT_RANDOM,
            user_stack_top - 2 * core::mem::size_of::<usize>(),
        ));
        // auxs.push(AuxEntry(AT_EXECFN, 32));
        // do not add this line, too wide
        // auxs.push(AuxEntry(AT_NULL, 0));

        // 分配用户栈
        memory_set.insert(
            VmArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
                Some(Arc::clone(&elf_file)),
                VirtAddr(user_stack_bottom).page_offset(),
            ),
            None,
        );

        // 分配用户堆，懒加载
        let user_heap_bottom: usize = usize::from(brk_start_va) + PAGE_SIZE;
        let user_heap_top: usize = user_heap_bottom + USER_HEAP_SIZE;
        // println!("[DEBUG] user heap:0x{:x?},0x{:x?}",user_heap_bottom,user_heap_top);
        memory_set.heap_areas = VmArea::new(
            user_heap_bottom.into(),
            user_heap_top.into(),
            MapType::Framed,
            MapPermission::R | MapPermission::W | MapPermission::U,
            Some(Arc::clone(&elf_file)),
            VirtAddr(user_heap_bottom).page_offset(),
        );

        memory_set.brk = user_heap_bottom;
        memory_set.brk_start = user_heap_bottom;

        LoadedELF {
            memory_set,
            user_stack_top,
            elf_entry: entry_point,
            auxs,
        }
    }

    /// 以COW的方式复制一个地址空间
    pub fn from_copy_on_write(user_space: &mut MemorySet) -> MemorySet {
        let mut new_memory_set = Self::new_bare(); // use 1 page (page_table root)

        // This part is not for Copy on Write.
        // Including:   Trampoline
        //              Trap_Context
        new_memory_set.map_trampoline(); // use 2 pages (page_table create ptes)
        for area in user_space.vm_areas.iter() {
            // use 1 page
            let start_vpn = area.vpn_range.get_start();
            if start_vpn == VirtAddr::from(TRAP_CONTEXT).floor() {
                let new_area = VmArea::from_another(area);
                new_memory_set.insert(new_area, None);
                for vpn in area.vpn_range {
                    let src_ppn = user_space.translate(vpn).unwrap().ppn();
                    let dst_ppn = new_memory_set.translate(vpn).unwrap().ppn();
                    // println!{"[COW TRAP_CONTEXT] mapping {:?} --- {:?}, src: {:?}", vpn, dst_ppn, src_ppn};
                    dst_ppn
                        .as_bytes_array()
                        .copy_from_slice(src_ppn.as_bytes_array());
                }
                break;
            }
        }
        // This part is for copy on write
        let parent_areas = &user_space.vm_areas;
        let parent_page_table = &mut user_space.page_table;
        for area in parent_areas.iter() {
            let start_vpn = area.vpn_range.get_start();
            if start_vpn != VirtAddr::from(TRAP_CONTEXT).floor() {
                let mut new_area = VmArea::from_another(area);
                // map the former physical address
                for vpn in area.vpn_range {
                    // change the map permission of both pagetable
                    // get the former flags and ppn
                    let pte = parent_page_table.translate(vpn).unwrap();
                    let pte_flags = pte.flags() & !PTEFlags::W;
                    let src_ppn = pte.ppn();
                    // frame_add_ref(src_ppn);
                    // change the flags of the src_pte
                    parent_page_table.set_flags(vpn, pte_flags);
                    parent_page_table.set_cow(vpn);
                    // map the cow page table to src_ppn
                    new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                    new_memory_set.page_table.set_cow(vpn);
                    new_area
                        .frame_map
                        .insert(vpn, FrameTracker::from_ppn(src_ppn));
                }
                new_memory_set.push_mapped_area(new_area);
            }
        }
        new_memory_set.mmap_manager = user_space.mmap_manager.clone();
        for (vpn, mmap_page) in user_space.mmap_manager.mmap_map.iter() {
            if (mmap_page.valid) {
                let vpn = vpn.clone();
                if let Some(pte) = parent_page_table.translate(vpn) {
                    // change the map permission of both pagetable
                    // get the former flags and ppn

                    // 只对已经map过的进行cow
                    let pte_flags = pte.flags() & !PTEFlags::W;
                    let src_ppn = pte.ppn();
                    // frame_add_ref(src_ppn);
                    // change the flags of the src_pte
                    parent_page_table.set_flags(vpn, pte_flags);
                    parent_page_table.set_cow(vpn);
                    // map the cow page table to src_ppn
                    new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                    new_memory_set.page_table.set_cow(vpn);
                }
            }
        }

        new_memory_set.heap_areas = VmArea::from_another(&user_space.heap_areas);
        for vpn in user_space.heap_areas.vpn_range.into_iter() {
            // change the map permission of both pagetable
            // get the former flags and ppn

            // 只对已经map过的进行cow
            if let Some(pte) = parent_page_table.translate(vpn) {
                let pte_flags = pte.flags() & !PTEFlags::W;
                let src_ppn = pte.ppn();
                // frame_add_ref(src_ppn);
                // change the flags of the src_pte
                parent_page_table.set_flags(vpn, pte_flags);
                parent_page_table.set_cow(vpn);
                // map the cow page table to src_ppn
                new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                new_memory_set.page_table.set_cow(vpn);

                new_memory_set
                    .heap_areas
                    .frame_map
                    .insert(vpn, FrameTracker::from_ppn(src_ppn));
            }
        }
        new_memory_set.brk_start = user_space.brk_start;
        new_memory_set.brk = user_space.brk;

        for shm_area in user_space.shm_areas.iter() {
            let mut new_shm_area = VmArea::from_another(shm_area);

            for vpn in shm_area.vpn_range.into_iter() {
                // change the map permission of both pagetable
                // get the former flags and ppn

                // 只对已经map过的进行cow
                if let Some(pte) = parent_page_table.translate(vpn) {
                    let pte_flags = pte.flags();
                    let src_ppn = pte.ppn();
                    new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                }
            }
            new_memory_set.shm_areas.push(new_shm_area);
        }
        new_memory_set.shm_top = user_space.shm_top;
        for (va, shm_tracker) in user_space.shm_trackers.iter() {
            let new_shm_tracker = SharedMemoryTracker::new(shm_tracker.key);
            new_memory_set
                .shm_trackers
                .insert(va.clone(), new_shm_tracker);
        }

        new_memory_set
    }

    #[no_mangle]
    pub fn cow_alloc(&mut self, vpn: VirtPageNum, former_ppn: PhysPageNum) -> isize {
        // 如果只有一个引用，那么改回 writable, 而不是重新分配 ppn
        if enquire_refcount(former_ppn) == 1 {
            self.page_table.reset_cow(vpn);
            // change the flags of the src_pte
            self.page_table.set_flags(
                vpn,
                self.page_table.translate(vpn).unwrap().flags() | PTEFlags::W,
            );
            return 0;
        }
        // 如果有多个引用，那么分配一个新的物理页，将内容复制过去
        let frame = alloc_frame().unwrap();
        let ppn = frame.ppn;
        self.remap_cow(vpn, ppn, former_ppn);

        // 注意: 这里通过 BTreeMap insert() 减少了引用计数
        for area in self.vm_areas.iter_mut() {
            let head_vpn = area.vpn_range.get_start();
            let tail_vpn = area.vpn_range.get_end();
            if vpn < tail_vpn && vpn >= head_vpn {
                // BTreeMap insert 之前, enqueue_refcount(former_ppn) > 1
                area.frame_map.insert(vpn, frame);
                // BTreeMap insert 之后, 由于在 from_copy_on_write() 时已经在 frame_map 中插入 vpn key,
                // insert 会返回旧的 value, 原有 FrameTracker drop, 减少 former_ppn 的引用计数
                return 0;
            }
        }
        if vpn >= VirtPageNum::from(self.mmap_manager.mmap_start)
            && vpn < VirtPageNum::from(self.mmap_manager.mmap_top)
        {
            self.mmap_manager.frame_trackers.insert(vpn, frame);
            return 0;
        }
        let head_vpn = self.heap_areas.vpn_range.get_start();
        let tail_vpn = self.heap_areas.vpn_range.get_end();
        if vpn < tail_vpn && vpn >= head_vpn {
            self.heap_areas.frame_map.insert(vpn, frame);
            return 0;
        }
        0
    }

    fn remap_cow(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, former_ppn: PhysPageNum) {
        self.page_table.remap_cow(vpn, ppn, former_ppn);
    }

    pub fn lazy_alloc_heap(&mut self, vpn: VirtPageNum) -> isize {
        self.heap_areas.lazy_map_vpn(vpn, &mut self.page_table);

        0
    }

    /// 根据多级页表和 vpn 查找页表项
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// 回收应用地址空间
    ///
    /// 将地址空间中的逻辑段列表 areas 清空（即执行 Vec 向量清空），
    /// 这将导致应用地址空间被回收（即进程的数据和代码对应的物理页帧都被回收），
    /// 但用来存放页表的那些物理页帧此时还不会被回收（会由父进程最后回收子进程剩余的占用资源）
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.vm_areas.clear();
    }

    /// 在地址空间中插入一个空的离散逻辑段
    ///
    /// - 已确定：
    ///     - 起止虚拟地址
    ///     - 映射方式：Framed
    ///     - map_perm
    /// - 留空：
    ///     - vpn_table
    ///     - data_frames

    pub fn check_va_range(&self, start_va: VirtAddr, len: usize) -> bool {
        let end_va = VirtAddr::from(start_va.0 + len);
        for area in self.vm_areas.iter() {
            if area.vpn_range.get_start() <= start_va.floor()
                && end_va.ceil() <= area.vpn_range.get_end()
            {
                return true;
            }
        }
        if self.mmap_manager.mmap_start <= start_va && end_va < self.mmap_manager.mmap_top {
            return true;
        }

        if self.heap_areas.vpn_range.get_start() <= start_va.floor()
            && end_va.ceil() <= self.heap_areas.vpn_range.get_end()
        {
            return true;
        }
        return false;
    }

    pub fn is_lazy_mapped(&self, addr_vpn: VirtPageNum) -> bool {
        self.page_table.find_pte(addr_vpn).is_some()
    }
    pub fn attach_shm(&mut self, key: usize, start_va: VirtAddr) {
        let (start_pa, size) = shm_get_address_and_size(key);
        let mut flags = PTEFlags::V | PTEFlags::U | PTEFlags::W | PTEFlags::R;
        let mut offset = 0;

        while offset < size {
            let va: VirtAddr = (start_va.0 + offset).into();
            let pa: PhysAddr = (start_pa.0 + offset).into();
            // println!("attach map va:{:x?} to pa{:x?}",va,pa);
            self.page_table.map(va.into(), pa.into(), flags);
            offset += PAGE_SIZE;
        }
        self.shm_top = self.shm_top.max(start_va.0 + size);
        let page_table = &self.page_table;
        let shm_tracker = SharedMemoryTracker::new(key);

        self.shm_trackers.insert(start_va, shm_tracker);
        let vma = VmArea::new(
            start_va,
            (start_va.0 + size).into(),
            MapType::Framed,
            MapPermission::R | MapPermission::W,
            None,
            0,
        );
        self.shm_areas.push(vma);
    }
    pub fn detach_shm(&mut self, start_va: VirtAddr) -> usize {
        // println!("detach start_va:{:?}",start_va);
        let key = self.shm_trackers.get(&start_va).unwrap().key;
        let (_, size) = shm_get_address_and_size(key);
        // println!("detach size:{:?}",size);
        let mut offset = 0;
        while offset < size {
            let va: VirtAddr = (start_va.0 + offset).into();
            // println!("detach va:{:?}",va);
            self.page_table.unmap(va.into());
            offset += PAGE_SIZE
        }
        self.shm_trackers.remove(&start_va);
        let vpn: VirtPageNum = start_va.into();
        self.shm_areas.retain(|x| x.start_vpn() != vpn);
        shm_get_nattch(key)
    }
    pub fn lazy_mmap(&mut self, vpn: VirtPageNum) {
        if let Some(frame) = alloc_frame() {
            let ppn = frame.ppn;
            self.mmap_manager.frame_trackers.insert(vpn, frame);
            let mmap_page = self.mmap_manager.mmap_map.get_mut(&vpn).unwrap();
            let pte_flags = PTEFlags::from_bits((mmap_page.prot.bits() << 1 & 0xf) as u16).unwrap();
            let pte_flags = pte_flags | PTEFlags::U;
            self.page_table.map(vpn, ppn, pte_flags);
            mmap_page.lazy_map_page(self.page_table.token());
        } else {
            panic!("No more memory!");
        }
    }
}

pub struct LoadedELF {
    pub memory_set: MemorySet,
    pub user_stack_top: usize,
    pub elf_entry: usize,
    pub auxs: Vec<AuxEntry>,
}
