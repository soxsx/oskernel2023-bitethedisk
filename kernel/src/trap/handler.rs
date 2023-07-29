//! trap 处理模块
//!
//! 根据 trap 发生的原因进行分发处理

use log::{debug, error};
use riscv::register::{
    mcause,
    mstatus::{self},
    mtval,
    scause::{self, Exception, Interrupt, Trap},
    stval,
};

use crate::{
    consts::TRAMPOLINE,
    syscall::dispatcher::{syscall, SYS_SIGRETURN},
    task::{
        current_add_signal, current_task, current_trap_cx, exec_signal_handlers,
        suspend_current_and_run_next, SigMask,
    },
    timer::{get_timeval, set_next_trigger, check_interval_timer},
};
use crate::{
    mm::{translated_mut, VirtAddr},
    task::current_user_token,
};

use super::{set_kernel_trap_entry, trap_return};

/// 用户态 trap 发生时的处理函数
#[no_mangle]
pub fn user_trap_handler() -> ! {
    // let pid = current_task().unwrap().pid();
    // println!("pid:{:?}",pid);
    set_kernel_trap_entry();
    // 用于描述 Trap 的原因
    let scause = scause::read();
    // 给出 Trap 附加信息
    let stval = stval::read();

    let task = current_task().unwrap();
    let mut inner = task.inner_mut();

    // 考虑以下情况，当一个进程因为耗尽时间片而让出执行流，切换回一个因为在内核态阻塞而让出执行流的
    // 另外一个进程的时候(内核态让出 `suspend` 可能是因为读取了一个空的管道等原因)，由于我们没有对
    // scause 等寄存器进行保存，所以当前这个由于时间片耗尽让出而恢复执行的内核态进程所关联的寄存器
    // 其实是那个因为时间片耗尽而让出的进程的相关寄存器的值。
    //
    // 类似的，当一个进程通过 `suspend` 让出执行流，若让出给了一个之前因为时间片中断而让出执行流
    // 的进程，则当前这个因为时间片耗尽而让出的进程的 scause 相关寄存器值也已经不是自己的值
    //
    // 所以通过在 `trap_return` 通过当前的 scause 寄存器值来判断当前进程是否应该再次赋予新的
    // 时间片的做法是错误的
    //
    // 另一种情况，在由于时间片耗尽而让出的 `suspend` 之后直接设置新的时间片，若在内核态执行了一
    // 系列耗时操作导致时间片提前用尽，则会使用户态程序一直处于时间片耗尽的状态而触发中断，最终会
    // 导致该用户态进程永远无法退出(在此用户态程序被 wait 的情况下等)，造成系统死锁
    //
    // 如果通过在 TaskControlBlock 中新加入一个字段 trap_cause 来保存和恢复 scause 则可以解决
    // 这个问题，但是如果内核在 trap_return 时执行的操作异常耗时(通常是错误的逻辑等)，那么会造成
    // 系统性能问题，而由于系统的“正确的”运行而造成的性能损耗相对于前边由于时间片问题导致内核锁死
    // 来说更难排查，所以目前只是在 TaskControlBlock 添加了字段并更新了逻辑，但是并未使能
    //
    //      inner.trap_cause = Some(scause);
    //
    // 之后再考虑处理这个问题

    let diff = get_timeval() - inner.last_enter_umode_time;
    inner.add_utime(diff);
    inner.set_last_enter_smode(get_timeval());
    drop(inner);
    drop(task);

    let mut is_sigreturn = false;

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            let syscall_id = cx.x[17];
            // get system call return value
            if syscall_id == SYS_SIGRETURN {
                is_sigreturn = true;
            }

            let result = syscall(
                cx.a7(),
                [cx.a0(), cx.a1(), cx.a2(), cx.a3(), cx.a4(), cx.a5()],
            );

            // cx is changed during sys_exec, so we have to call it again
            if !is_sigreturn {
                cx = current_trap_cx();
                cx.x[10] = result as usize;
            }
        }

        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            // println!(
            //     "user_trap_handler: memory fault, task: {} at {:x?}, {:x?}",
            //     current_task().unwrap().pid(),
            //     VirtAddr::from(stval as usize),
            //     current_trap_cx().sepc,
            // );

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
                current_add_signal(SigMask::SIGSEGV);
            }
            let task = current_task().unwrap();

            // println!("######### check_lazy ##############");

            let lazy = task.check_lazy(va, is_load);
            if lazy != 0 {
                println!("LAZY FAIL {:?}", lazy);
                current_add_signal(SigMask::SIGSEGV);
            }
            // println!("TRAP END");
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

            current_add_signal(SigMask::SIGSEGV);
        }

        Trap::Exception(Exception::IllegalInstruction) => {
            println!("stval:{}", stval);

            let sepc = riscv::register::sepc::read();
            println!("sepc:0x{:x}", sepc);

            current_add_signal(SigMask::SIGILL);
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

    check_interval_timer();

    if !is_sigreturn {
        exec_signal_handlers();
    }

    trap_return();
}

/// 内核态 trap 发生时的处理函数
#[no_mangle]
pub fn kernel_trap_handler() -> ! {
    let mstatus = mstatus::read();
    let mcause = mcause::read();
    error!(
        "mstatus: {:?}, mtval: {}, mcause: {:?}",
        mstatus,
        mtval::read(),
        mcause
    );

    panic!("a trap {:?} from kernel!", scause::read().cause());
}
