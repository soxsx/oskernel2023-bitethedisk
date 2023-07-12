//! 参考 `xv6-riscv` 启动阶段的设计
//!
//! [`BOOTED`] 全局变量负责记录内核资源是否已经完全初始化，该初始化过程是 0 号 CPU
//! 来负责的
//!
//! [`synchronize_hart`] 在资源初始化后调用，原子性的将 [`BOOTED`] 字段设置为 `true`，
//! 并加入内存屏障保证对于 [`BOOTED`] 的读都发生在 [`synchronize_hart`] 的写之后
//!
//! [`wait_for_booting`] 会持续读 [`BOOTED`]，直到其变成 `true`

use core::sync::atomic::AtomicBool;

pub static mut BOOTED: AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// 原子性的将内核资源初始化完成标记变量 [`BOOTED`] 设置为 `true`
///
/// 该宏同时会插入内存屏障:
///
/// ```rust
/// { core::arch::asm!("fence") };
/// ```
///
/// 保证对 [`BOOTED`] 的 `读` 发生在对其的 `写` 之后
macro_rules! synchronize_hart {
    () => {{
        unsafe {
            $crate::macros::on_boot::BOOTED.store(true, core::sync::atomic::Ordering::Relaxed);
            core::arch::asm!("fence");
        }
    }};
}

/// 循环 `读` 内核资源初始化完成标记变量 [`BOOTED`]，直到其变为 `true`
macro_rules! wait_for_booting {
    () => {{
        unsafe {
            while !$crate::macros::on_boot::BOOTED.load(core::sync::atomic::Ordering::Acquire) {}
        }
    }};
}
