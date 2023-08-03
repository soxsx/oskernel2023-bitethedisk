use core::arch::global_asm;

use alloc::{borrow::ToOwned, sync::Arc, vec::Vec};
use spin::RwLock;

use crate::{
    fs::{self, AbsolutePath, CreateMode, OpenFlags},
    mm::MemorySet,
    task::task::TaskControlBlock,
};

use super::AuxEntry;

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

        let task = TaskControlBlock::new(inode.clone());
        inode.delete(); // 删除 initproc 文件
        task
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
        let path = AbsolutePath::from_str("/static-busybox");

        let inode = fs::open(path, OpenFlags::O_CREATE, CreateMode::empty()).expect("static-busybox create failed");
        inode.write_all(&busybox.to_owned());

        let task = Arc::new(TaskControlBlock::new(inode.clone()));
        inode.delete();
        Busybox {
            inner: task,
        }
    });
}

pub static mut STATIC_BUSYBOX_ENTRY: usize = 0;
pub static mut STATIC_BUSYBOX_AUX: Vec<AuxEntry> = Vec::new();

pub struct Busybox {
    inner: Arc<TaskControlBlock>,
}

impl Busybox {
    pub fn elf_entry_point(&self) -> usize {
        unsafe { STATIC_BUSYBOX_ENTRY }
    }
    pub fn aux(&self) -> Vec<AuxEntry> {
        unsafe { STATIC_BUSYBOX_AUX.clone() }
    }
    pub fn memory_set(&self) -> MemorySet {
        let mut memory_set = self.inner.memory_set.write();
        MemorySet::from_copy_on_write(&mut memory_set)
    }
}
