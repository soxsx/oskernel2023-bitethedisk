use alloc::collections::BTreeMap;
use alloc::{sync::Arc, vec::Vec};
use path::AbsolutePath;

use super::{MapPermission, MapType, VmArea, VmAreaType};
use crate::board::CLOCK_FREQ;
use crate::consts::{
    LINK_BASE, MMAP_BASE, PAGE_SIZE, SHM_BASE, SIGNAL_TRAMPOLINE, THREAD_LIMIT, TRAMPOLINE,
    TRAP_CONTEXT_BASE, USER_HEAP_SIZE, USER_STACK_BASE, USER_STACK_SIZE,
};
use crate::fs::{open, File};
use crate::mm::{
    alloc_frame, enquire_refcount, shm_get_address_and_size, shm_get_nattch, FrameTracker,
    MmapManager, PTEFlags, PageTable, PageTableEntry, PhysAddr, PhysPageNum, SharedMemoryTracker,
    VirtAddr, VirtPageNum,
};
use crate::task::trap_context_position;

use nix::{
    AuxEntry, CreateMode, OpenFlags, AT_BASE, AT_CLKTCK, AT_EGID, AT_ENTRY, AT_EUID, AT_FLAGS,
    AT_GID, AT_HWCAP, AT_PAGESZ, AT_PHDR, AT_PHENT, AT_PHNUM, AT_RANDOM, AT_SECURE, AT_UID,
};

pub struct MemorySet {
    pub page_table: PageTable,

    pub vm_areas: Vec<VmArea>,

    pub mmap_manager: MmapManager,
    pub heap_areas: VmArea,

    pub shm_areas: Vec<VmArea>,

    pub shm_trackers: BTreeMap<VirtAddr, SharedMemoryTracker>,
    pub shm_top: usize,

    pub brk_start: usize,
    pub brk: usize,
    pub user_stack_areas: VmArea,
    pub user_stack_start: usize,
    pub user_stack_end: usize,
}

impl MemorySet {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            vm_areas: Vec::new(),
            heap_areas: VmArea::new(
                0.into(),
                0.into(),
                MapType::Framed,
                VmAreaType::UserHeap,
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
            user_stack_areas: VmArea::new(
                0.into(),
                0.into(),
                MapType::Framed,
                VmAreaType::UserStack,
                MapPermission::R | MapPermission::W | MapPermission::U,
                None,
                0,
            ),
            user_stack_start: 0,
            user_stack_end: 0,
        }
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// 在当前地址空间插入一个 `Framed` 方式映射到物理内存的逻辑段
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
        area_tpye: VmAreaType,
    ) {
        self.insert(
            VmArea::new(
                start_va,
                end_va,
                MapType::Framed,
                area_tpye,
                permission,
                None,
                0,
            ),
            None,
        );
    }

    /// 通过起始虚拟页号删除对应的逻辑段(包括连续逻辑段和离散逻辑段)
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
            // 写入初始化数据, 如果数据存在
            map_area.copy_data(&mut self.page_table, data.0, data.1, data.2);
        }
        self.vm_areas.push(map_area); // 将生成的数据段压入 areas 使其生命周期由areas控制
    }

    /// 在当前地址空间插入一段已被分配空间的连续逻辑段
    ///
    /// 主要用于 COW 创建时子进程空间连续逻辑段的插入, 其要求指定物理页号
    pub fn push_mapped_area(&mut self, map_area: VmArea) {
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

    pub fn map_signal_trampoline(&mut self) {
        extern "C" {
            fn user_sigreturn();
        }
        self.page_table.map(
            VirtAddr::from(SIGNAL_TRAMPOLINE).into(),
            PhysAddr::from(user_sigreturn as usize).into(),
            PTEFlags::R | PTEFlags::X | PTEFlags::U,
        );
    }

    pub fn map_trap_context(&mut self) {
        self.insert(
            VmArea::new(
                TRAP_CONTEXT_BASE.into(),
                SIGNAL_TRAMPOLINE.into(),
                MapType::Framed,
                VmAreaType::TrapContext,
                MapPermission::R | MapPermission::W,
                None,
                0,
            ),
            None,
        );
    }
    pub fn map_thread_trap_context(&mut self, tid: usize) {
        assert!(tid > 0 && tid < THREAD_LIMIT);
        let start_va = trap_context_position(tid);
        let end_va = VirtAddr::from(start_va.0 + PAGE_SIZE);
        self.insert(
            VmArea::new(
                start_va,
                end_va,
                MapType::Framed,
                VmAreaType::TrapContext,
                MapPermission::R | MapPermission::W,
                None,
                0,
            ),
            None,
        );
    }

    pub fn load_elf(elf_file: Arc<dyn File>) -> LoadedELF {
        #[cfg(feature = "static-busybox")]
        {
            const BB: &str = "BUSYBOX";
            if elf_file.name() == BB {
                return hijack_busybox_load_elf();
            }
        }
        let mut memory_set = Self::new_bare();
        let mut auxs = Vec::new();

        memory_set.map_trampoline();
        memory_set.map_signal_trampoline();
        memory_set.map_trap_context();

        // 第一次读取前64字节确定程序表的位置与大小
        let elf_head_data = elf_file.kernel_read_with_offset(0, 64);
        let elf_head_data_slice = elf_head_data.as_slice();
        let elf = xmas_elf::ElfFile::new(elf_head_data_slice).unwrap();

        let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
        let ph_offset = elf.header.pt2.ph_offset() as usize;
        let ph_count = elf.header.pt2.ph_count() as usize;

        // 进行第二次读取, 这样的elf对象才能正确解析程序段头的信息
        let elf_head_data =
            elf_file.kernel_read_with_offset(0, ph_offset + ph_count * ph_entry_size);
        let elf = xmas_elf::ElfFile::new(elf_head_data.as_slice()).unwrap();

        let mut head_va = None; // top va of ELF which points to ELF header

        // 记录目前涉及到的最大的虚拟地址
        let mut brk_start_va = VirtAddr(0);
        let mut dynamic_link = false;
        let mut entry_point = elf.header.pt2.entry_point() as usize;
        // 遍历程序段进行加载
        for i in 0..ph_count as u16 {
            let ph = elf.program_header(i).unwrap();
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
                        VmAreaType::Elf,
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
                _ => {}
            }
        }
        if dynamic_link {
            let path = AbsolutePath::from_str("/libc.so");
            let interpreter_file = open(path, OpenFlags::O_RDONLY, CreateMode::empty())
                .expect("can't find interpreter file");
            // 第一次读取前64字节确定程序表的位置与大小
            let interpreter_head_data = interpreter_file.kernel_read_with_offset(0, 64);
            let interp_elf = xmas_elf::ElfFile::new(interpreter_head_data.as_slice()).unwrap();

            let ph_entry_size = interp_elf.header.pt2.ph_entry_size() as usize;
            let ph_offset = interp_elf.header.pt2.ph_offset() as usize;
            let ph_count = interp_elf.header.pt2.ph_count() as usize;

            // 进行第二次读取, 这样的elf对象才能正确解析程序段头的信息
            let interpreter_head_data =
                interpreter_file.kernel_read_with_offset(0, ph_offset + ph_count * ph_entry_size);
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
                        VmAreaType::Elf,
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

        // let user_stack_top = TRAP_CONTEXT_BASE - THREAD_LIMIT * PAGE_SIZE;
        // let user_stack_bottom = user_stack_top - USER_STACK_SIZE;

        let user_stack_top = USER_STACK_BASE;
        let user_stack_bottom = user_stack_top - USER_STACK_SIZE;

        let ph_head_addr = head_va.unwrap() + elf.header.pt2.ph_offset() as usize;

        // get auxv vector
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
        auxs.push(AuxEntry(AT_PHDR, ph_head_addr as usize));
        auxs.push(AuxEntry(AT_PHENT, elf.header.pt2.ph_entry_size() as usize)); // ELF64 header 64bytes
        auxs.push(AuxEntry(AT_PHNUM, ph_count as usize));
        // Interp
        // auxs.push(AuxEntry(AT_BASE, 0));
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

        // 分配用户栈, 懒加载
        memory_set.user_stack_start = user_stack_bottom;
        memory_set.user_stack_end = user_stack_top;
        memory_set.user_stack_areas = VmArea::new(
            user_stack_bottom.into(),
            user_stack_top.into(),
            MapType::Framed,
            VmAreaType::UserStack,
            MapPermission::R | MapPermission::W | MapPermission::U,
            None,
            0,
        );

        // 分配用户堆, 懒加载
        let user_heap_bottom: usize = usize::from(brk_start_va) + PAGE_SIZE;
        let user_heap_top: usize = user_heap_bottom + USER_HEAP_SIZE;
        // println!("[DEBUG] user heap:0x{:x?},0x{:x?}",user_heap_bottom,user_heap_top);
        memory_set.heap_areas = VmArea::new(
            user_heap_bottom.into(),
            user_heap_top.into(),
            MapType::Framed,
            VmAreaType::UserHeap,
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
        new_memory_set.map_trampoline();
        new_memory_set.map_signal_trampoline();

        // This part is for copy on write
        let parent_areas = &user_space.vm_areas;
        let parent_page_table = &mut user_space.page_table;
        for area in parent_areas.iter() {
            match area.area_type {
                VmAreaType::TrapContext => {
                    let new_area = VmArea::from_another(area);
                    new_memory_set.insert(new_area, None);
                    for vpn in area.vpn_range {
                        let src_ppn = parent_page_table.translate(vpn).unwrap().ppn();
                        let dst_ppn = new_memory_set.translate(vpn).unwrap().ppn();
                        // println!{"[COW TRAP_CONTEXT] mapping {:?} --- {:?}, src: {:?}", vpn, dst_ppn, src_ppn};
                        dst_ppn
                            .as_bytes_array()
                            .copy_from_slice(src_ppn.as_bytes_array());
                    }
                }
                VmAreaType::UserStack | VmAreaType::Elf => {
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
                _ => {
                    unreachable!()
                }
            }
        }
        new_memory_set.mmap_manager = user_space.mmap_manager.clone();
        for (vpn, mmap_page) in user_space.mmap_manager.mmap_map.iter() {
            if mmap_page.valid {
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
        new_memory_set.user_stack_areas = VmArea::from_another(&user_space.user_stack_areas);
        for vpn in user_space.user_stack_areas.vpn_range.into_iter() {
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
                    .user_stack_areas
                    .frame_map
                    .insert(vpn, FrameTracker::from_ppn(src_ppn));
            }
        }
        new_memory_set.user_stack_start = user_space.user_stack_start;
        new_memory_set.user_stack_end = user_space.user_stack_end;

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
            let new_shm_area = VmArea::from_another(shm_area);

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
        // 如果只有一个引用, 那么改回 writable, 而不是重新分配 ppn
        if enquire_refcount(former_ppn) == 1 {
            self.page_table.reset_cow(vpn);
            // change the flags of the src_pte
            self.page_table.set_flags(
                vpn,
                self.page_table.translate(vpn).unwrap().flags() | PTEFlags::W,
            );
            return 0;
        }
        // 如果有多个引用, 那么分配一个新的物理页, 将内容复制过去
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
        if vpn >= self.user_stack_areas.vpn_range.get_start()
            && vpn < self.user_stack_areas.vpn_range.get_end()
        {
            self.user_stack_areas.frame_map.insert(vpn, frame);
            return 0;
        }

        if vpn >= self.heap_areas.vpn_range.get_start() && vpn < self.heap_areas.vpn_range.get_end()
        {
            self.heap_areas.frame_map.insert(vpn, frame);
            return 0;
        }
        panic!("cow of of range");
    }

    fn remap_cow(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, former_ppn: PhysPageNum) {
        self.page_table.remap_cow(vpn, ppn, former_ppn);
    }

    pub fn lazy_alloc_heap(&mut self, vpn: VirtPageNum) -> isize {
        self.heap_areas.lazy_map_vpn(vpn, &mut self.page_table);

        0
    }
    pub fn lazy_alloc_stack(&mut self, vpn: VirtPageNum) -> isize {
        self.user_stack_areas
            .lazy_map_vpn(vpn, &mut self.page_table);

        0
    }

    /// 根据多级页表和 vpn 查找页表项
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// 回收应用地址空间
    ///
    /// 将地址空间中的逻辑段列表 areas 清空(即执行 Vec 向量清空),
    /// 这将导致应用地址空间被回收(即进程的数据和代码对应的物理页帧都被回收),
    /// 但用来存放页表的那些物理页帧此时还不会被回收(会由父进程最后回收子进程剩余的占用资源)
    #[allow(unused)]
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.vm_areas.clear();
    }

    /// 在地址空间中插入一个空的离散逻辑段
    ///
    /// - 已确定:
    ///     - 起止虚拟地址
    ///     - 映射方式: Framed
    ///     - map_perm
    /// - 留空:
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

        // TODO boundary check
        if self.heap_areas.vpn_range.get_start() <= start_va.floor()
            && end_va.ceil() <= self.heap_areas.vpn_range.get_end()
        {
            return true;
        }
        if self.user_stack_areas.vpn_range.get_start() <= start_va.floor()
            && end_va.ceil() <= self.user_stack_areas.vpn_range.get_end()
        {
            return true;
        }

        return false;
    }

    #[allow(unused)]
    pub fn is_lazy_mapped(&self, addr_vpn: VirtPageNum) -> bool {
        self.page_table.find_pte(addr_vpn).is_some()
    }
    pub fn attach_shm(&mut self, key: usize, start_va: VirtAddr) {
        let (start_pa, size) = shm_get_address_and_size(key);
        let flags = PTEFlags::V | PTEFlags::U | PTEFlags::W | PTEFlags::R;
        let mut offset = 0;

        while offset < size {
            let va: VirtAddr = (start_va.0 + offset).into();
            let pa: PhysAddr = (start_pa.0 + offset).into();
            // println!("attach map va:{:x?} to pa{:x?}",va,pa);
            self.page_table.map(va.into(), pa.into(), flags);
            offset += PAGE_SIZE;
        }
        self.shm_top = self.shm_top.max(start_va.0 + size);
        let shm_tracker = SharedMemoryTracker::new(key);

        self.shm_trackers.insert(start_va, shm_tracker);
        let vma = VmArea::new(
            start_va,
            (start_va.0 + size).into(),
            MapType::Framed,
            VmAreaType::Shared,
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

#[cfg(feature = "static-busybox")]
use crate::task::BUSYBOX;
#[cfg(feature = "static-busybox")]
fn hijack_busybox_load_elf() -> LoadedELF {
    let bb = BUSYBOX.read();
    let memory_set = bb.memory_set();
    let user_stack_top = memory_set.user_stack_end;
    let elf_entry = bb.elf_entry_point();
    let auxs = bb.aux();
    LoadedELF {
        memory_set,
        user_stack_top,
        elf_entry,
        auxs,
    }
}

pub struct LoadedELF {
    pub memory_set: MemorySet,
    pub user_stack_top: usize,
    pub elf_entry: usize,
    pub auxs: Vec<AuxEntry>,
}
