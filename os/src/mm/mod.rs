/// # 内存管理模块
/// `os/src/mm/mod.rs`
/// ## 实现功能
/// ```
/// pub fn init()
/// ```
//
mod address; // 地址数据类型
mod frame_allocator; // 物理页帧管理器
pub mod kernel_vmm;
mod memory_set; // 地址空间模块
mod page_table; // 页表
mod vma; // 虚拟内存地址映射空间

use core::arch::asm;

use address::VPNRange;
pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
pub use memory_set::{MapPermission, MemorySet};
use page_table::PTEFlags;
pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};
use riscv::register::satp;
pub use vma::*;

use crate::kernel_token;

use self::kernel_vmm::KERNEL_VMM;

/// 初始化内存管理系统
pub fn init() {
    init_frame_allocator();
    enable_mmu();
}

pub fn init_frame_allocator() {
    frame_allocator::init();
}

pub fn enable_mmu() {
    satp::write(kernel_token!());
    unsafe { asm!("sfence.vma") } // 刷新 TLB
}
