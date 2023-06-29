//! trap 处理模块
//!
//! 根据 trap 发生的原因进行分发处理

use log::{debug, error};
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    stval,
};

use crate::{
    consts::TRAMPOLINE,
    mm::VirtAddr,
    syscall::dispatcher::syscall,
    task::{
        current_add_signal, current_task, current_trap_cx, suspend_current_and_run_next,
        SignalFlags,
    },
    timer::set_next_trigger,
};

use super::{set_kernel_trap_entry, trap_return};

/// 用户态 trap 发生时的处理函数
#[no_mangle]
pub fn user_trap_handler() -> ! {
    set_kernel_trap_entry();

    // 用于描述 Trap 的原因
    let scause = scause::read();
    // 给出 Trap 附加信息
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_cx();

            cx.sepc += 4;

            let result = syscall(
                cx.a7(),
                [cx.a0(), cx.a1(), cx.a2(), cx.a3(), cx.a4(), cx.a5()],
            );

            // cx is changed during sys_exec, so we have to call it again
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }

        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            debug!(
                "user_trap_handler: memory fault, task: {} at {:?}, {:?}",
                current_task().unwrap().pid(),
                VirtAddr::from(stval as usize),
                VirtAddr::from(stval as usize).floor(),
            );

            let is_load: bool;
            if scause.cause() == Trap::Exception(Exception::LoadFault)
                || scause.cause() == Trap::Exception(Exception::LoadPageFault)
            {
                is_load = true;
            } else {
                is_load = false;
            }

            let va: VirtAddr = (stval as usize).into();
            if va > TRAMPOLINE.into() {
                println!("[kernel trap] VirtAddr out of range!");
                current_add_signal(SignalFlags::SIGSEGV);
            }
            let task = current_task().unwrap();

            debug!("user_trap_handler: lazy mapping, task: {:?}", task.pid());

            let lazy = task.check_lazy(va, is_load);

            if lazy != 0 {
                current_add_signal(SignalFlags::SIGSEGV);
            }
        }

        Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault) => {
            let task = current_task().unwrap();
            debug!(
                "{:?} in application {}, bad addr = {:#x}, bad instruction = {:#x}.",
                scause.cause(),
                task.pid.0,
                stval,
                current_trap_cx().sepc,
            );
            drop(task);

            current_add_signal(SignalFlags::SIGSEGV);
        }

        Trap::Exception(Exception::IllegalInstruction) => {
            println!("stval:{}", stval);

            let sepc = riscv::register::sepc::read();
            println!("sepc:0x{:x}", sepc);

            current_add_signal(SignalFlags::SIGILL);
        }

        // 时间片到了
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            suspend_current_and_run_next();
            set_next_trigger();
        }

        _ => panic!(
            "trap {:?} is unsupported, stval = {:#x}!",
            scause.cause(),
            stval
        ),
    }

    trap_return();
}

/// 内核态 trap 发生时的处理函数
#[no_mangle]
pub fn kernel_trap_handler() -> ! {
    use riscv::register::sepc;

    error!(
        "kernel_trap_handler: stval = {:#x}, sepc = {:#x}",
        stval::read(),
        sepc::read()
    );

    panic!("a trap {:?} from kernel!", scause::read().cause());
}
