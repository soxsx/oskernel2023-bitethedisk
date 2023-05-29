#![no_std]
#![no_main]
// features
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

#[macro_use]
mod macros;
#[macro_use]
mod console; // 控制台模块

#[path = "boards/qemu.rs"]
mod board; // 与虚拟机相关的参数

mod consts;
mod drivers; // 设备驱动层
mod fs;
mod logging;
mod mm;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

use core::{arch::global_asm, slice};
use riscv::register::sstatus::{set_fs, FS};

global_asm!(include_str!("entry.S"));

#[cfg(not(feature = "multi_harts"))]
#[no_mangle]
pub fn meow() -> ! {
    if hartid!() == 0 {
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

#[cfg(feature = "multi_harts")]
#[no_mangle]
pub fn meow() -> ! {
    if hartid!() == 0 {
        init_bss();
        unsafe { set_fs(FS::Dirty) }
        lang_items::setup();
        mm::init_frame_allocator();
        mm::enable_mmu();
        trap::init();
        trap::enable_stimer_interrupt();
        timer::set_next_trigger();
        fs::init();
        task::add_initproc();

        synchronize_hart!()
    } else {
        wait_for_booting!();

        unsafe { set_fs(FS::Dirty) }

        mm::enable_mmu();
        trap::init();
        trap::enable_stimer_interrupt();
        timer::set_next_trigger();
    }

    task::run_tasks();
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

    use crate::sbi::shutdown;
    use core::panic::PanicInfo;

    pub fn setup() {
        init_heap();
    }

    #[panic_handler]
    fn _panic(info: &PanicInfo) -> ! {
        if let Some(location) = info.location() {
            println!(
                "[kernel] Panicked at {}:{} {}",
                location.file(),
                location.line(),
                info.message().unwrap()
            );
        } else {
            println!("[kernel] Panicked: {}", info.message().unwrap());
        }
        shutdown()
    }

    // 通过 `global_allocator` 注解将 HEAP_ALLOCATOR 标记为 Rust 的内存分配器
    // Rust 的相关数据结构，如 Vec, BTreeMap 等，依赖于该分配器
    #[global_allocator]
    static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

    // 用于处理动态内存分配失败的情形，直接 panic
    #[alloc_error_handler]
    pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
        panic!("Heap allocation error, layout = {:#x?}", layout);
    }

    const KERNEL_HEAP_SIZE: usize = 4096 * 256; // 1M

    // 给全局分配器用于分配的一块内存，位于内核的 .bss 段中
    static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

    fn init_heap() {
        unsafe {
            HEAP_ALLOCATOR
                .lock()
                .init(KERNEL_HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
        }
    }
}
