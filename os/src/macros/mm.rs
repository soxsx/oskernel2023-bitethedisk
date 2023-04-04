macro_rules! new_kernel_vm {
    () => {{
        $crate::debug!("start kernel virtual memory mapping");
        let mut memory_set = MemorySet::new();

        $crate::debug!("mapping trampoline for kernel...");
        memory_set.map_trampoline();

        $crate::debug!("mapping .text section for kernel...");
        memory_set.add_one_segment(
            $crate::mm::memory_set::VirtSegment::new(
                stext!().into(),
                etext!().into(),
                $crate::mm::memory_set::MappingType::Identical,
                $crate::mm::memory_set::SegmentAccess::R
                    | $crate::mm::memory_set::SegmentAccess::X,
            ),
            None,
        );
        $crate::debug!("mapping .rodata section for kernel...");
        memory_set.add_one_segment(
            $crate::mm::memory_set::VirtSegment::new(
                srodata!().into(),
                erodata!().into(),
                $crate::mm::memory_set::MappingType::Identical,
                $crate::mm::memory_set::SegmentAccess::R,
            ),
            None,
        );
        $crate::debug!("mapping .data section for kernel...");
        memory_set.add_one_segment(
            $crate::mm::memory_set::VirtSegment::new(
                sdata!().into(),
                edata!().into(),
                $crate::mm::memory_set::MappingType::Identical,
                $crate::mm::memory_set::SegmentAccess::R
                    | $crate::mm::memory_set::SegmentAccess::W,
            ),
            None,
        );
        $crate::debug!("mapping .bss section for kernel...");
        memory_set.add_one_segment(
            $crate::mm::memory_set::VirtSegment::new(
                sbss!().into(),
                ebss!().into(),
                $crate::mm::memory_set::MappingType::Identical,
                $crate::mm::memory_set::SegmentAccess::R
                    | $crate::mm::memory_set::SegmentAccess::W,
            ),
            None,
        );
        $crate::debug!("mapping rest availble memory for kernel...");
        memory_set.add_one_segment(
            $crate::mm::memory_set::VirtSegment::new(
                ekernel!().into(),
                $crate::config::MEMORY_END.into(),
                $crate::mm::memory_set::MappingType::Identical,
                $crate::mm::memory_set::SegmentAccess::R
                    | $crate::mm::memory_set::SegmentAccess::W,
            ),
            None,
        );

        memory_set
    }};
}

/// kernel 根页表的 token
macro_rules! kernel_token {
    () => {{
        $crate::mm::kernel_vmm::KERNEL_VMM.lock().token()
    }};
}
