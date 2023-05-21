use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    board::MMIO,
    consts::PHYS_END,
    mm::{
        memory_set::{MapArea, MapType},
        MapPermission, MemorySet,
    },
};

lazy_static! {
    /// Kernel virtual memory [`MemorySet`]
    pub static ref KERNEL_VMM: Arc<Mutex<MemorySet>> = Arc::new(Mutex::new({
        extern "C" {
            fn stext();
            fn etext();
            fn srodata();
            fn erodata();
            fn sdata();
            fn edata();
            fn sbss();
            fn ebss();
            fn ekernel();
        }

        let mut memory_set = MemorySet::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (sbss as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                PHYS_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
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
    }));
}

#[macro_export]
macro_rules! kernel_token {
    () => {{
        $crate::mm::kernel_vmm::KERNEL_VMM.lock().token()
    }};
}
