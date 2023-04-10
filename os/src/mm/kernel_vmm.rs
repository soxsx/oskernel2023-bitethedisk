//!
//! Kernel virtual memory [`MemorySet`]
//!

use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    config::{MMIO, PHYS_END},
    mm::{
        memory_set::{MapArea, MapType},
        MapPermission, MemorySet,
    },
};

lazy_static! {
    pub static ref KERNEL_VMM: Arc<Mutex<MemorySet>> = Arc::new(Mutex::new(build_kernel_vmm()));
}

#[macro_export]
macro_rules! kernel_token {
    () => {{
        $crate::mm::kernel_vmm::KERNEL_VMM.lock().token()
    }};
}

fn build_kernel_vmm() -> MemorySet {
    // 下面的已按照 linker.ld 中的排序
    extern "C" {
        fn stext();
        fn strampoline();
        fn etext();

        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn skstack();
        fn ekstack();
        fn ebss();

        fn ekernel();
    }

    let mut memory_set = MemorySet::new();

    memory_set.map_trampoline();

    // .text section
    memory_set.push(
        MapArea::new(
            (stext as usize).into(),
            (etext as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::X,
        ),
        None,
    );

    // .rodata section
    memory_set.push(
        MapArea::new(
            (srodata as usize).into(),
            (erodata as usize).into(),
            MapType::Identical,
            MapPermission::R,
        ),
        None,
    );

    // .data section
    memory_set.push(
        MapArea::new(
            (sdata as usize).into(),
            (edata as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ),
        None,
    );

    // .bss section
    memory_set.push(
        MapArea::new(
            (sbss as usize).into(),
            (ebss as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ),
        None,
    );

    // 其他可用物理内存的映射
    memory_set.push(
        MapArea::new(
            (ekernel as usize).into(),
            PHYS_END.into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ),
        None,
    );

    // MMIO
    for pair in MMIO {
        // 恒等映射 内存映射 I/O (MMIO, Memory-Mapped I/O) 地址到内核地址空间
        memory_set.push(
            MapArea::new(
                (*pair).0.into(),
                ((*pair).0 + (*pair).1).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
    }

    memory_set
}
