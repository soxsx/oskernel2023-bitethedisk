pub const USER_STACK_SIZE: usize = 4096 * 2048;
pub const KERNEL_STACK_SIZE: usize = 4096 * 32; // 应用进程在内核的栈大小

pub const USER_HEAP_SIZE: usize = 4096 * 30000;
pub const KERNEL_HEAP_SIZE: usize = 4096 * 8192; // 32M

pub const PHYS_END: usize = 0x8800_0000; // 128 MiB
                                         // pub const PHYS_END: usize = 0xa000_0000; // 512 MiB

/// 页面大小: 4KiB
pub const PAGE_SIZE: usize = 0x1000;

/// 跳板虚拟内存中的起始地址, 虚拟内存最高页
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// 用于存放信号处理函数的栈
pub const SIGNAL_TRAMPOLINE: usize = TRAMPOLINE - PAGE_SIZE;

/// Trap 上下文在应用地址空间中的位置
pub const TRAP_CONTEXT_BASE: usize = SIGNAL_TRAMPOLINE - PAGE_SIZE;

pub const USER_STACK_BASE: usize = 0xf000_0000;

pub const THREAD_LIMIT: usize = 4096 * 2;

pub use crate::board::{CLOCK_FREQ, MMIO};

pub const MMAP_BASE: usize = 0x6000_0000;

// pub const MMAP_END: usize = 0x68000000; // mmap 区大小为 128 MiB

pub const SHM_BASE: usize = 0x7000_0000;

pub const LINK_BASE: usize = 0x2000_0000;

pub const FD_LIMIT: usize = 1024;
