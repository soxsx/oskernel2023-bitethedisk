pub const USER_STACK_SIZE: usize = 4096 * 4;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 应用进程在内核的栈大小

pub const KERNEL_HEAP_SIZE: usize = 4096 * 256; // 1M
pub const USER_HEAP_SIZE: usize = 4096 * 48;

pub const PHYS_END: usize = 0x88000000; // 128 MiB

/// 页面大小：4KiB
pub const PAGE_SIZE: usize = 0x1000;
/// 页内偏移：12bit
pub const IN_PAGE_OFFSET: usize = 0xc;

/// 跳板虚拟内存中的起始地址，虚拟内存最高页
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
/// Trap 上下文在应用地址空间中的位置
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub use crate::board::{CLOCK_FREQ, MMIO};

pub const MMAP_BASE: usize = 0x60000000;
