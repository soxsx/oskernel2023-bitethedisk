//! 内核 Trap 管理
//!
//! 流程：
//! 首先通过 __alltraps 将 Trap 上下文（不是那个结构体）保存在进程内核栈上，
//! 然后跳转到使用 Rust 编写的 trap_handler 函数完成 Trap 分发及处理。
//! 当 trap_handler 返回之后，使用 __restore 从保存在内核栈上的 Trap 上下文恢复寄存器。
//! 最后通过一条 sret 指令回到应用程序执行。

mod context;
pub mod handler;

use self::handler::kernel_trap_handler;
use crate::consts::{TRAMPOLINE, TRAP_CONTEXT};
use crate::task::{check_signals_of_current, current_user_token, exit_current_and_run_next};
use core::arch::{asm, global_asm};
use riscv::register::{mtvec::TrapMode, sie, stvec};

// 跳板页上的汇编代码
global_asm!(include_str!("trampoline.S"));

pub fn init() {
    set_kernel_trap_entry();
}

/// ### 设置内核态下的 Trap 入口
/// 一旦进入内核后再次触发到 S态 Trap，则硬件在设置一些 CSR 寄存器之后，会跳过对通用寄存器的保存过程，
/// 直接跳转到 trap_from_kernel 函数，在这里直接 panic 退出
fn set_kernel_trap_entry() {
    unsafe { stvec::write(kernel_trap_handler as usize, TrapMode::Direct) }
}

/// ### 设置用户态下的 Trap 入口
/// 我们把 stvec 设置为内核和应用地址空间共享的跳板页面的起始地址 TRAMPOLINE
/// 而不是编译器在链接时看到的 __alltraps 的地址。这是因为启用分页模式之后，
/// 内核只能通过跳板页面上的虚拟地址来实际取得 __alltraps 和 __restore 的汇编代码
fn set_user_trap_entry() {
    unsafe { stvec::write(TRAMPOLINE as usize, TrapMode::Direct) }
}

/// 启用 S 特权级时钟中断
pub fn enable_stimer_interrupt() {
    unsafe { sie::set_stimer() }
}

/// 通过在Rust语言中加入宏命令调用 `__restore` 汇编函数
#[no_mangle]
pub fn trap_return() -> ! {
    // check signals
    if let Some((errno, _msg)) = check_signals_of_current() {
        // println!("[kernel] {}", _msg);
        exit_current_and_run_next(errno);
    }

    set_user_trap_entry();

    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    // __restore 在虚拟地址空间的地址
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",              // 指令清空指令缓存 i-cache
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") TRAP_CONTEXT,   // Trap 上下文在应用地址空间中的位置
            in("a1") user_satp,     // 即将回到的应用的地址空间的 token
            options(noreturn)
        );
    }
}

pub use context::TrapContext;
