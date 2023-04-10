// 告诉 Rust 编译器不使用 Rust 标准库 std 转而使用核心库 core（core库不需要操作系统的支持）
#![no_std]
// 不使用main函数，而使用汇编代码指定的入口
#![no_main]
// features
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

// Suppress warning.
// #![allow(unused)]
// #![allow(dead_code)]

#[macro_use]
extern crate alloc;

// 如果 extern crate 出现在 crate 的根模块中，那么此 crate名称也会被添加到外部预导入包中，
// 以便使其自动出现在所有模块的作用域中。
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;

mod config; // 参数库
mod console;
#[macro_use]
mod mm;
mod drivers;
mod fs;
mod sbi; // 实现了 RustSBI 通信的相关功能
mod sync; // 允许在单核处理器上将引用做全局变量使用
mod syscall; // 系统调用模块
mod task; // 任务管理模块
mod timer; // 时间片模块
mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.S")); // 代码的第一条语句，执行指定的汇编文件，汇编程序再调用Rust实现的内核
global_asm!(include_str!("buildin_app.S")); // 将 c_usertests 程序放入内核区内存空间

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

#[no_mangle]
pub fn meow() -> ! {
    if hartid!() != 0 {
        loop {}
    }

    clear_bss();
    lang_items::setup();
    println!("[kernel] Hello, world!");
    check_kernel_segment();
    mm::init();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    fs::list_apps();
    task::add_initproc();
    println!("[kernel] add initproc!");
    task::run_tasks();

    unreachable!("you should not be here");
}

fn clear_bss() {
    unsafe {
        core::slice::from_raw_parts_mut(ekstack as usize as *mut u8, ebss as usize - ekstack as usize)
            .fill(0);
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
                "[kernel] panicked at {}:{} {}",
                location.file(),
                location.line(),
                info.message().unwrap()
            );
        } else {
            println!("[kernel] panicked: {}", info.message().unwrap());
        }
        shutdown()
    }

    use crate::config::KERNEL_HEAP_SIZE;

    // 通过 `global_allocator` 注解将 HEAP_ALLOCATOR 标记为 Rust 的内存分配器
    // Rust 的相关数据结构，如 Vec, BTreeMap 等，依赖于该分配器
    #[global_allocator]
    static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

    // 用于处理动态内存分配失败的情形，直接 panic
    #[alloc_error_handler]
    pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
        panic!("Heap allocation error, layout = {:?}", layout);
    }

    // 给全局分配器用于分配的一块内存，位于内核的 .bss 段中
    static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

    #[inline(always)]
    fn init_heap() {
        unsafe {
            HEAP_ALLOCATOR
                .lock()
                .init(KERNEL_HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
        }
    }
}

fn check_kernel_segment() {
    println!("{:X} (stext)", stext as usize);
    println!("{:X} (strampoline)", strampoline as usize);
    println!("{:X} (etext)", etext as usize);
    println!("{:X} (srodata)", srodata as usize);
    println!("{:X} (erodata)", erodata as usize);
    println!("{:X} (sdata)", sdata as usize);
    println!("{:X} (edata)", edata as usize);
    println!("{:X} (sstack)", skstack as usize);
    println!("{:X} (estack)", ekstack as usize);
    println!("{:X} (sbss)", sbss as usize);
    println!("{:X} (ebss)", ebss as usize);
    println!("{:X} (ekernel)", ekernel as usize);
}
