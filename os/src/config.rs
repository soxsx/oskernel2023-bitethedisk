#![allow(unused)]

pub const USER_STACK_SIZE: usize = 0x2000; // 8 KiB
pub const KERNEL_STACK_SIZE: usize = 0x2000;
pub const KERNEL_HEAP_SIZE: usize = 0x20_0000; // 128 KiB

/// 指定内存终止物理地址，内存大小为 128 MiB（左闭右开）
pub const PHYS_END: usize = 0x88000000;

/// 页面大小：4KiB
pub const PAGE_SIZE: usize = 0x1000;

/// 页内偏移：12bit
pub const IN_PAGE_OFFSET: usize = 0xc;

/// 跳板虚拟内存中的起始地址，虚拟内存最高页
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// Trap 上下文在应用地址空间中的位置
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub const KMMAP_BASE: usize = 0x90000000;
pub const MMAP_BASE: usize = 0x60000000;

pub use board_qemu::*;
pub mod board_qemu {
    pub const CLOCK_FREQ: usize = 12500000;

    /// 硬编码 Qemu 上的 VirtIO 总线的 MMIO 地址区间（起始地址，长度）
    pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];

    // pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
}
