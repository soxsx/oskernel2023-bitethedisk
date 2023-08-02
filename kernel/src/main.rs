#![no_std]
#![no_main]
// Features, need nightly toolchain.
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
#![feature(error_in_core)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

#[cfg(feature = "time_tracer")]
#[macro_use]
extern crate time_tracer;

#[macro_use]
mod macros;
#[macro_use]
mod console;

#[path = "boards/qemu.rs"]
mod board;

mod consts;
mod drivers;
mod fs;
mod logging;
mod mm;
mod panic;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

use core::{
    arch::global_asm,
    slice,
    sync::atomic::{AtomicBool, Ordering},
};
use riscv::register::sstatus::{set_fs, FS};

global_asm!(include_str!("entry.S"));

lazy_static! {
    static ref MEOWED: AtomicBool = AtomicBool::new(false);
}

#[no_mangle]
pub fn meow() -> ! {
    if MEOWED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        println!("boot hart id: {}", hartid!());
        init_bss();
        unsafe { set_fs(FS::Dirty) }
        lang_items::setup();
        logging::init();
        mm::init();
        trap::init();
        trap::enable_stimer_interrupt();
        timer::set_next_trigger();
        fs::init();
        task::add_initproc();
        task::run_tasks();
    } else {
        loop {}
    }

    unreachable!("main.rs/meow: you should not be here!");
}

fn init_bss() {
    extern "C" {
        fn ekstack0();
        fn ebss();
    }
    unsafe {
        let sbss = ekstack0 as usize as *mut u8;
        let ebss = ebss as usize as *mut u8;
        slice::from_mut_ptr_range(sbss..ebss)
            .into_iter()
            .for_each(|byte| (byte as *mut u8).write_volatile(0));
    }
}

pub use lang_items::*;

pub mod lang_items {

    use buddy_system_allocator::LockedHeap;

    use crate::consts::KERNEL_HEAP_SIZE;

    pub fn setup() {
        init_heap();
    }

    #[global_allocator]
    static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();
    #[alloc_error_handler]
    pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
        panic!("Heap allocation error, layout = {:#x?}", layout);
    }

    static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

    fn init_heap() {
        unsafe {
            HEAP_ALLOCATOR
                .lock()
                .init(KERNEL_HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
        }
    }
    pub fn heap_usage() {
        let usage_actual = HEAP_ALLOCATOR.lock().stats_alloc_actual();
        let usage_all = HEAP_ALLOCATOR.lock().stats_total_bytes();
        println!("[kernel] HEAP USAGE:{:?} {:?}", usage_actual, usage_all);
    }
}
