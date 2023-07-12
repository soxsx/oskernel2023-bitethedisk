pub const USER_STACK_SIZE: usize = 4096 * 2048;
pub const KERNEL_STACK_SIZE: usize = 4096 * 8; // 应用进程在内核的栈大小

pub const USER_HEAP_SIZE: usize = 4096 * 48;

pub const PHYS_END: usize = 0x88000000; // 128 MiB

/// 页面大小：4KiB
pub const PAGE_SIZE: usize = 0x1000;

/// 跳板虚拟内存中的起始地址，虚拟内存最高页
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// Trap 上下文在应用地址空间中的位置
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub use crate::board::{CLOCK_FREQ, MMIO};

