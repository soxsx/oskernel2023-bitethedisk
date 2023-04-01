mod address; // 地址数据类型
mod frame_allocator; // 物理页帧管理器
mod memory_set; // 地址空间模块
mod page_table; // 页表
mod vma; // 虚拟内存地址映射空间

pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_dealloc, frame_usage, FrameTracker};
pub use memory_set::{kernel_token, MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};
pub use vma::*;

/// 内存管理子系统的初始化
pub fn init() {
    frame_allocator::init_frame_allocator();
    // 从这一刻开始 SV39 分页模式就被启用了
    KERNEL_SPACE.lock().activate();
}
