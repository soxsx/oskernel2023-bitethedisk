use alloc::sync::Arc;
use spin::{Mutex, MutexGuard};

use crate::{
    board::MMIO,
    consts::PHYS_END,
    mm::{
        memory_set::{vm_area::VmArea, MapType, VmAreaType},
        MapPermission, MemorySet,
    },
};

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

lazy_static! {
    /// 内核虚拟地址空间抽象
    static ref KERNEL_VMM: Arc<Mutex<MemorySet>> = Arc::new(Mutex::new({
        let mut memory_set = MemorySet::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map signal trampoline
        memory_set.map_signal_trampoline();
        macro_rules! insert_kernel_vm_areas {
            ($kvmm:ident,$($start:expr, $end:expr, $permission:expr, $file:expr, $page_offset:expr)*) => {
                $(
                    $kvmm.insert(
                        VmArea::new(
                            ($start as usize).into(),
                            ($end as usize).into(),
                            MapType::Identical,
                            VmAreaType::KernelSpace,
                            $permission,
                            $file,
                            $page_offset,
                        ),
                        None
                    );
                )*
            };
        }
        insert_kernel_vm_areas! { memory_set,
            stext,   etext,    MapPermission::R | MapPermission::X, None, 0
            srodata, erodata,  MapPermission::R, None, 0
            sdata,   edata,    MapPermission::R | MapPermission::W, None, 0
            sbss,    ebss,     MapPermission::R | MapPermission::W, None, 0
            ekernel, PHYS_END, MapPermission::R | MapPermission::W, None, 0
        }

        // 恒等映射 内存映射 I/O (MMIO, Memory-Mapped I/O) 地址到内核地址空间
        for &pair in MMIO {
            insert_kernel_vm_areas!(memory_set,
                pair.0, pair.0+pair.1, MapPermission::R | MapPermission::W, None, 0);
        }

        memory_set
    }));
}

pub fn acquire_kvmm<'a>() -> MutexGuard<'a, MemorySet> {
    KERNEL_VMM.lock()
}
