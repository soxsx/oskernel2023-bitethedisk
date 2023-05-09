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

/// `trap` 处理函数
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
                cx.x[17],
                [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]],
            );
            // cx is changed during sys_exec, so we have to call it again
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }

        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            // println!("[Kernel trap] pid:{}, Mem Fault trapped, {:?}, {:?}", current_task().unwrap().getpid(), VirtAddr::from(stval as usize), VirtAddr::from(stval as usize).floor());
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
            let lazy = task.check_lazy(va, is_load);

            if lazy != 0 {
                current_add_signal(SignalFlags::SIGSEGV);
            }
        }

        Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault) => {
            let task = current_task().unwrap();
            println!(
                "[kernel] {:?} in application {}, bad addr = {:#x}, bad instruction = {:#x}.",
                scause.cause(),
                task.pid.0,
                stval,
                current_trap_cx().sepc,
            );
            drop(task);

            current_add_signal(SignalFlags::SIGSEGV);
        }

        Trap::Exception(Exception::IllegalInstruction) => {
            // println!("[kernel] IllegalInstruction in application, kernel killed it.");
            // // illegal instruction exit code
            // exit_current_and_run_next(-3);
            println!("stval:{}", stval);
            let sepc = riscv::register::sepc::read();
            println!("sepc:0x{:x}", sepc);
            // current_task().unwrap().inner_exclusive_access().memory_set.debug_show_data(sepc.into());
            current_add_signal(SignalFlags::SIGILL);
        }

        // 时间片到了
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }

        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }

    trap_return();
}

/// 在内核触发Trap后会转到这里引发Panic
#[no_mangle]
pub fn kernel_trap_handler() -> ! {
    use riscv::register::sepc;
    println!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}
