use alloc::sync::Arc;
use spin::{Mutex, MutexGuard};

use crate::{
    board::{MMIO, PHYSICAL_MEM_END},
    mm::{
        vm_area::{VmArea, VmAreaType},
        MapPermission, MapType, MemorySet,
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
    /// Kernel's virtual memory memory set.
    static ref KERNEL_VMM: Arc<Mutex<MemorySet>> = Arc::new(Mutex::new({
        let mut memory_set = MemorySet::new_bare();
        memory_set.map_trampoline();
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
            ekernel, PHYSICAL_MEM_END,
                MapPermission::R | MapPermission::W, None, 0
        }

        // For MMIO(Memory mapped IO).
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
