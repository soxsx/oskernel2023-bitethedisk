use buddy_system_allocator::LockedHeap;

use crate::consts::KERNEL_HEAP_SIZE;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:#x?}", layout);
}

static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(KERNEL_HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

#[allow(unused)]
pub fn heap_usage() {
    let usage_actual = HEAP_ALLOCATOR.lock().stats_alloc_actual();
    let usage_all = HEAP_ALLOCATOR.lock().stats_total_bytes();
    println!("[kernel] HEAP USAGE:{:?} {:?}", usage_actual, usage_all);
}
