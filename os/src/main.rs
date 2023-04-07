// os/src/main.rs

#![no_std]
// 告诉 Rust 编译器不使用 Rust 标准库 std 转而使用核心库 core（core库不需要操作系统的支持）
#![no_main] // 不使用main函数，而使用汇编代码指定的入口
#![feature(panic_info_message)] // 让panic函数能通过 PanicInfo::message 获取报错信息
#![feature(alloc_error_handler)] // 用于处理动态内存分配失败的情形

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

#[path = "boards/qemu.rs"]
mod board; // 与虚拟机相关的参数
mod config; // 参数库
mod console; // 控制台模块
mod drivers; // 设备驱动层
mod fs; // 内核文件系统接口
mod mm; // 内存空间模块
mod sbi; // 实现了 RustSBI 通信的相关功能
mod sync; // 允许在单核处理器上将引用做全局变量使用
mod syscall; // 系统调用模块
mod task; // 任务管理模块
mod timer; // 时间片模块
mod trap; // 提供 Trap 管理

use core::arch::global_asm;

global_asm!(include_str!("entry.S")); // 代码的第一条语句，执行指定的汇编文件，汇编程序再调用Rust实现的内核
global_asm!(include_str!("buildin_app.S")); // 将 c_usertests 程序放入内核区内存空间

#[no_mangle]
pub fn meow() -> ! {
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
    panic!("Unreachable in rust_main!");
}

/// 初始化内存.bss区域
fn clear_bss() {
    unsafe {
        core::slice::from_raw_parts_mut(sbss!() as *mut u8, ebss!() - sbss!()).fill(0);
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

    #[panic_handler] //通知编译器用panic函数来对接 panic! 宏
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

    use crate::config::KERNEL_HEAP_SIZE;

    /// 通过 `global_allocator` 注解将 HEAP_ALLOCATOR 标记为 Rust 的内存分配器
    /// Rust 的相关数据结构，如 Vec, BTreeMap 等，依赖于该分配器
    #[global_allocator]
    static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

    /// 用于处理动态内存分配失败的情形,直接panic
    #[alloc_error_handler]
    pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
        panic!("Heap allocation error, layout = {:?}", layout);
    }

    /// 给全局分配器用于分配的一块内存，位于内核的 .bss 段中
    static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

    /// 初始化内核堆内存，以便使用 Rust 提供的数据结构
    #[inline(always)]
    fn init_heap() {
        unsafe {
            HEAP_ALLOCATOR
                .lock()
                .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
        }
    }
}

fn check_kernel_segment() {
    println!("{:X} (stext)", stext!());
    println!("{:X} (strampoline)", strampoline!());
    println!("{:X} (etext)", etext!());
    println!("{:X} (srodata)", srodata!());
    println!("{:X} (erodata)", erodata!());
    println!("{:X} (sdata)", sdata!());
    println!("{:X} (edata)", edata!());
    println!("{:X} (sstack)", skstack!());
    println!("{:X} (estack)", ekstack!());
    println!("{:X} (sbss)", sbss!());
    println!("{:X} (ebss)", ebss!());
    println!("{:X} (ekernel)", ekernel!());
}
