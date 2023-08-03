mod context;
mod handler;
pub use context::*;
pub use handler::*;

use crate::consts::TRAMPOLINE;
use crate::task::trap_context_position;
use crate::task::{current_task, current_user_token};
use crate::timer::get_timeval;
use core::arch::{asm, global_asm};
use riscv::register::{mtvec::TrapMode, sie, stvec};

global_asm!(include_str!("trampoline.S"));

/// trap 初始化
///
/// 在内核初始化阶段发生的 trap 为 S 特权器的 trap, 设置成相应的内核态 trap 处理函数 [`kernel_trap_handler`]
pub fn init() {
    set_kernel_trap_entry();
}

/// 设置内核态下的 trap 入口
///
/// 在内核态发生 trap 后, CPU 会跳转执行 [`kernel_trap_handler`] 处的代码
fn set_kernel_trap_entry() {
    unsafe { stvec::write(kernel_trap_handler as usize, TrapMode::Direct) }
}

/// 设置用户态下的 trap 入口
///
/// 在用户态发生 trap 后, CPU 会跳转执行 [`TRAMPOLINE`] 处的代码
fn set_user_trap_entry() {
    unsafe { stvec::write(TRAMPOLINE as usize, TrapMode::Direct) }
}

/// 使能 S 特权级时钟中断
pub fn enable_stimer_interrupt() {
    unsafe { sie::set_stimer() }
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();

    let user_satp = current_user_token();

    extern "C" {
        fn user_trapvec();
        fn user_trapret();
    }

    let task = current_task();
    let mut inner = task.inner_mut();
    let diff = get_timeval() - inner.last_enter_smode_time;
    inner.add_stime(diff);
    inner.set_last_enter_umode(get_timeval());

    // 参考 [`trap_handler`] 中的注释
    //
    //      if let Some(scause) = inner.trap_cause {
    //          if scauseinner_mute() == Trap::Interrupt(Interrupt::SupervisorTimer) {
    //              set_next_trigger();
    //          }
    //          inner.trap_cause = None;
    //      }
    //
    // 之后考虑使能上面的代码, 并删除处理时钟中断的 trap 分支中的时间片设置操作
    let trap_addr = trap_context_position(task.pid() - task.tgid).0;
    drop(inner);
    drop(task);

    let trapret_addr = user_trapret as usize - user_trapvec as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",              // 指令清空指令缓存 i-cache
            "jr {user_trapret}",
            user_trapret = in(reg) trapret_addr,
            in("a0") trap_addr,  // trap 上下文在应用地址空间中的位置
            in("a1") user_satp,     // 即将回到的应用的地址空间的 token
            options(noreturn)
        );
    }
}
