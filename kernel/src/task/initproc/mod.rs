use core::arch::global_asm;

use alloc::{borrow::ToOwned, sync::Arc};

use crate::{
    fs::{self, open_flags::CreateMode, AbsolutePath, OpenFlags},
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
}
