pub const USER_STACK_SIZE: usize = 4096 * 4;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 应用进程在内核的栈大小

pub const KERNEL_HEAP_SIZE: usize = 4096 * 256; // 1M
pub const USER_HEAP_SIZE: usize = 4096 * 48;

/// 指定内存终止物理地址，内存大小为6MiB（左闭右开）(8M有大坑，会随机卡死)
#[cfg(feature = "board_k210")]
pub const MEMORY_END: usize = 0x80600000;
#[cfg(not(any(feature = "board_k210")))]
pub const MEMORY_END: usize = 0x807E0000;

// pub const MEMORY_END:           usize = 0x88000000;

/// 页内偏移：12bit
pub const PAGE_SIZE_BITS: usize = 0xc;

/// 跳板虚拟内存中的起始地址，虚拟内存最高页
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
/// Trap 上下文在应用地址空间中的位置
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub const MMAP_BASE: usize = 0x60000000;

/// 4KiB
pub const PAGE_SIZE: usize = 0x1000;

/// CLOCK_FREQ 是一个预先获取到的各平台不同的时钟频率
// pub const CLOCK_FREQ: usize = 12500000;

// pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];

pub const NCPU: usize = 5;

pub mod platform {
    pub mod qemu {
        pub const CLOCK_FREQ: usize = 12500000;

        /// 硬编码 Qemu 上的 VirtIO 总线的 MMIO 地址区间（起始地址，长度）
        pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];

        pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
    }

    // pub mod todo!()
}
