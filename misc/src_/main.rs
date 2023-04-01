#![no_std]
// 这里使失能了所有的 Rust 语言入口点，即执行 main 函数前的初始化函数 `start`
// 如果没写，则会报错：
// error: requires `start` lang_item
//
// ps: 其实可以在这里初始化串口之类的，但使用了 SBI 后就不用管这些了。
#![no_main]
#![allow(dead_code)]
#![allow(unused)]
// features
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

use core::{arch::global_asm, panic::PanicInfo};

use config::KERNEL_HEAP_SIZE;
use linked_list_allocator::LockedHeap;
use log::{debug, error, info, trace};
use sbi::legacy::shutdown;

// `alloc` with `#![no_std]` support
// see [`link`](https://doc.rust-lang.org/edition-guide/rust-2018/path-changes.html#an-exception-for-extern-crate)
extern crate alloc;

#[macro_use]
mod macros;

mod config;
mod console;
mod logging;
mod mm;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;
mod batch;

global_asm!(include_str!("entry.S"));

#[cfg(feature = "board_k210")]
#[path = "boards/k210.rs"]
mod board; // 与硬件板相关的参数
#[cfg(not(any(feature = "board_k210")))]
#[path = "boards/qemu.rs"]
mod board; // 与虚拟机相关的参数

#[macro_use]
mod drivers; // 设备驱动层
mod fs; // 内核文件系统接口

use riscv::register::sstatus::{set_fs, FS};

core::arch::global_asm!(include_str!("entry.S")); // 代码的第一条语句，执行指定的汇编文件，汇编程序再调用Rust实现的内核
#[cfg(feature = "nofs")]
core::arch::global_asm!(include_str!("buildin_app.S")); // 将 syscall 测试程序放入内核区内存空间

#[no_mangle]
fn meow() -> ! {
    #[cfg(not(feature = "single_hart"))]
    boot_multi_harts();

    #[cfg(feature = "single_hart")]
    boot_single_hart();

    unreachable!("you are missing...");
}

#[inline(always)]
fn boot_single_hart() {
    clear_bss();

    unsafe { set_fs(FS::Dirty) }

    logging::init();

    // 初始化 Rust 全局内存分配器，为了使用 core 中的数据结构
    init_global_allocator();
    info!("gloabl allocator initialized successfully!");

    mm::init();
    info!("memory initialized successfully!");

    trap::init();
    info!("trap initialized successfully!");
    trap::enable_timer_interrupt();
    info!("enabled timer interrupt.");

    timer::set_next_trigger();
    info!("");

    fs::init();
    task::add_initproc();

    #[cfg(feature = "dev")]
    check_kernel_segment();
    info!("initialization succeeded");

    task::run_tasks();
}

#[inline(always)]
fn boot_multi_harts() {
    todo!("not finished yet!");
    if get_hartid!() == 0 {
        synchronize_hart!();
    } else {
        wait_for_booting!();
    }
}

fn check_kernel_segment() {
    debug!("{:X} (stext)", stext!());
    debug!("{:X} (strampoline)", strampoline!());
    debug!("{:X} (etext)", etext!());
    debug!("{:X} (srodata)", srodata!());
    debug!("{:X} (erodata)", erodata!());
    debug!("{:X} (sdata)", sdata!());
    debug!("{:X} (edata)", edata!());
    debug!("{:X} (sbss)", sbss!());
    debug!("{:X} (ebss)", ebss!());
    debug!("{:X} (ekernel)", ekernel!());
}

fn clear_bss() {
    unsafe {
        core::slice::from_raw_parts_mut(sbss!() as *mut u8, ebss!() - sbss!()).fill(0);
    }
}

#[panic_handler]
fn panic_(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        #[cfg(feature = "colorful")]
        error!(
            "[kernel] panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
        #[cfg(not(feature = "colorful"))]
        println!(
            "[kernel] panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        #[cfg(feature = "colorful")]
        error!("[kernel] panicked: {}", info.message().unwrap());
        #[cfg(not(feature = "colorful"))]
        println!("[kernel] panicked: {}", info.message().unwrap());
    }
    shutdown()
}

static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

fn init_global_allocator() {
    unsafe {
        ALLOCATOR
            .lock()
            .init(KERNEL_HEAP.as_ptr() as *mut u8, KERNEL_HEAP_SIZE);
    }
}

#[alloc_error_handler]
fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    heap_usage();
    // TODO: 检查需要分配的内存大小并在可能合适的情况下多分配些内存，避免 panic
    panic!(
        "[kernel] heap allocation error, required layout: {:?}",
        layout
    );
}

fn heap_usage() {
    let used = ALLOCATOR.lock().used();
    let total_size = ALLOCATOR.lock().size();
    let usage = used as f64 / total_size as f64 * 100.0;
    #[cfg(feature = "colorful")]
    error!(
        "[kernel] heap usage: {:.2}% ({}/{} bytes)",
        usage, used, total_size
    );
    #[cfg(not(feature = "colorful"))]
    println!(
        "[kernel] heap usage: {:.2}% ({}/{} bytes)",
        usage, used, total_size
    );
}
