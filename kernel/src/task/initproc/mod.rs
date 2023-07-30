use core::{
    arch::global_asm,
    sync::atomic::{AtomicU8, AtomicUsize},
};

use alloc::{borrow::ToOwned, collections::BTreeMap, sync::Arc, vec::Vec};
use nix::CloneFlags;
use spin::RwLock;

use crate::{
    fs::{self, open_flags::CreateMode, AbsolutePath, OpenFlags},
    mm::{
        alloc_frame,
        memory_set::{AuxEntry, MapPermission, MapType, MemorySet, VmArea, VmAreaType},
        shared_memory::SharedMemoryTracker,
        FrameTracker, MmapManager, PTEFlags, PageTable, PageTableEntry, VirtAddr,
    },
    task::task::TaskControlBlock,
};

global_asm!(include_str!("initproc.S"));

lazy_static! {
    /// 引导 pcb
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        extern "C" {
            fn initproc_entry();
            fn initproc_tail();
        }
        let entry = initproc_entry as usize;
        let tail = initproc_tail as usize;
        let siz = tail - entry;

        let initproc = unsafe { core::slice::from_raw_parts(entry as *const u8, siz) };
        let path = AbsolutePath::from_str("/initproc");

        let  inode = fs::open(path, OpenFlags::O_CREATE, CreateMode::empty()).expect("initproc create failed!");
        inode.write_all(&initproc.to_owned());

        let tcb = TaskControlBlock::new(inode.clone());
        inode.delete(); // 删除 initproc 文件
        tcb
    });

    pub static ref BUSYBOX: RwLock<Busybox> = RwLock::new({
        extern "C" {
            fn busybox_entry();
            fn busybox_tail();
        }
        let entry = busybox_entry as usize;
        let tail = busybox_tail as usize;
        let siz = tail - entry;

        let busybox = unsafe { core::slice::from_raw_parts(entry as *const u8, siz) };
        let path = AbsolutePath::from_str("/busybox0");

        let inode = fs::open(path, OpenFlags::O_CREATE, CreateMode::empty()).expect("busybox0 create failed");
        inode.write_all(&busybox.to_owned());

        let bb = Arc::new(TaskControlBlock::new(inode.clone()));
        inode.delete();
        Busybox {
            inner: bb,
        }
    });
}

pub static mut ONCE_BB_ENTRY: usize = 0;
pub static mut ONCE_BB_AUX: Vec<AuxEntry> = Vec::new();

pub struct Busybox {
    inner: Arc<TaskControlBlock>,
}

impl Busybox {
    pub fn elf_entry_point(&self) -> usize {
        unsafe { ONCE_BB_ENTRY }
    }
    pub fn aux(&self) -> Vec<AuxEntry> {
        unsafe { ONCE_BB_AUX.clone() }
    }
    pub fn memory_set(&self) -> MemorySet {
        let mut write = self.inner.memory_set.write();
        MemorySet::from_copy_on_write(&mut write)
    }
    fn reflect_vm_areas(&self, other: &mut Vec<VmArea>, page_table: &mut PageTable) {
        let mm = self.inner.memory_set.read();
        let vm_areas = &mm.vm_areas;
        let copy_vm_area_shallow = |vm_area: &VmArea| -> VmArea {
            VmArea {
                area_type: vm_area.area_type,
                vpn_range: vm_area.vpn_range,
                map_type: vm_area.map_type,
                permission: vm_area.permission,
                file: None,
                file_offset: 0,
                frame_map: BTreeMap::new(),
            }
        };
        vm_areas.iter().for_each(|vm_area| {
            let new_vm_area = copy_vm_area_shallow(vm_area);
            let bb_pte_flags = PTEFlags::from_bits(vm_area.permission.bits()).unwrap();
            new_vm_area.vpn_range.into_iter().for_each(|vpn| {
                if bb_pte_flags.contains(PTEFlags::W) {
                    let frame = alloc_frame().unwrap();
                    let ppn = frame.ppn;
                    let pte = page_table.find_pte_create(vpn).unwrap();
                    assert!(!pte.is_valid(), "{:#x?} is mapped before mapping", vpn);
                    *pte = PageTableEntry::new(ppn, bb_pte_flags | PTEFlags::V);
                    page_table.frames.push(frame);
                } else {
                    let ppn = vm_area.frame_map.get(&vpn).unwrap().ppn;
                    page_table.map(vpn, ppn, bb_pte_flags);
                }
            });
            other.push(new_vm_area);
        });
    }
    fn reflect_heap_areas(&self, other: &mut VmArea) {
        let mm = self.inner.memory_set.read();
        *other = VmArea::new(
            mm.brk_start.into(),
            mm.brk.into(),
            MapType::Framed,
            VmAreaType::UserHeap,
            MapPermission::R | MapPermission::W | MapPermission::U,
            None,
            VirtAddr(mm.brk_start).page_offset(),
        );
    }
    fn reflect_stack_areas(&self, other: &mut VmArea) {
        let mm = self.inner.memory_set.read();
        *other = VmArea::new(
            mm.user_stack_start.into(),
            mm.user_stack_end.into(),
            MapType::Framed,
            VmAreaType::UserStack,
            MapPermission::R | MapPermission::W | MapPermission::U,
            None,
            0,
        );
    }
}
