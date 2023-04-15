mod address; // 地址数据类型
mod frame_allocator; // 物理页帧管理器
pub mod kernel_vmm;
mod memory_set; // 地址空间模块
mod page_table; // 页表
mod vma; // 虚拟内存地址映射空间

pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_dealloc, frame_usage, FrameTracker};
pub use memory_set::{MapPermission, MemorySet};
pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};
use riscv::register::satp;
pub use vma::*;

use crate::{kernel_token, lang_items};

/// 内存管理子系统的初始化
pub fn init() {
    init_frame_allocator();
    enable_mmu();
}

pub fn init_frame_allocator() {
    frame_allocator::init();
}

pub fn enable_mmu() {
    satp::write(kernel_token!());
    unsafe { core::arch::asm!("sfence.vma") } // 刷新 MMU 的 TLB
}

pub fn memory_usage() {
    println!("---------------------Memory usage---------------------");
    frame_allocator::frame_usage();
    lang_items::heap_usage();
    println!("------------------------------------------------------");
}
