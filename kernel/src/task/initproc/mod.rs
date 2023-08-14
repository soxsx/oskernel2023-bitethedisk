use core::arch::global_asm;

use alloc::{borrow::ToOwned, sync::Arc};
use nix::{CreateMode, OpenFlags};
use path::AbsolutePath;

use crate::{fs::open, task::TaskControlBlock};

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

        let  inode = open(path, OpenFlags::O_CREATE, CreateMode::empty()).expect("initproc create failed!");
        inode.write_all(&initproc.to_owned());

        let task = TaskControlBlock::new(inode.clone());
        inode.delete(); // 删除 initproc 文件
        task
    });

}

// This is the processing done in the first stage of the national competition.
// At that time, the file system was not optimized, and the page cache mechanism was not added.
// We could only do some simple optimization.
#[cfg(feature = "static_busybox")]
pub static mut STATIC_BUSYBOX_ENTRY: usize = 0;
#[cfg(feature = "static_busybox")]
pub static mut STATIC_BUSYBOX_AUX: Vec<AuxEntry> = Vec::new();
#[cfg(feature = "static_busybox")]
pub struct Busybox {
    inner: Arc<TaskControlBlock>,
}
#[cfg(feature = "static_busybox")]
impl Busybox {
    pub fn elf_entry_point(&self) -> usize {
        unsafe { STATIC_BUSYBOX_ENTRY }
    }
    pub fn aux(&self) -> Vec<AuxEntry> {
        use alloc::vec::Vec;
        unsafe { STATIC_BUSYBOX_AUX.clone() }
    }
    pub fn memory_set(&self) -> MemorySet {
        use crate::mm::MemorySet;
        let mut memory_set = self.inner.memory_set.write();
        MemorySet::from_copy_on_write(&mut memory_set)
    }
}
#[cfg(feature = "static_busybox")]
lazy_static! {
    pub static ref BUSYBOX: RwLock<Busybox> = RwLock::new({
        use super::AuxEntry;
        use spin::RwLock;
        extern "C" {
            fn busybox_entry();
            fn busybox_tail();
        }
        let entry = busybox_entry as usize;
        let tail = busybox_tail as usize;
        let siz = tail - entry;

        let busybox = unsafe { core::slice::from_raw_parts(entry as *const u8, siz) };
        let path = AbsolutePath::from_str("/static-busybox");

        let inode = open(path, OpenFlags::O_CREATE, CreateMode::empty())
            .expect("static-busybox create failed");
        inode.write_all(&busybox.to_owned());

        let task = Arc::new(TaskControlBlock::new(inode.clone()));
        inode.delete();
        Busybox { inner: task }
    });
}
