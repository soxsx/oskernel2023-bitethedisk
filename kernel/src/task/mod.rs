pub mod context; // 任务上下文模块
mod initproc;
mod kernel_stack;
mod manager; // 进程管理器
mod pid; // 进程标识符模块
pub mod processor; // 处理器管理模块
mod signals;
mod switch; // 任务上下文切换模块
pub mod task;

use core::usize;

use alloc::sync::Arc;
use fat32::sync_all;
use manager::remove_from_pid2task;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use manager::{add_task, check_hanging, pid2task, unblock_task};
pub use pid::{pid_alloc, PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, schedule::*, take_current_task,
};
pub use signals::*;
pub use task::FD_LIMIT;

use self::{
    initproc::INITPROC,
    manager::block_task,
    processor::{acquire_processor, schedule},
};

/// 将当前任务置为就绪态，放回到进程管理器中的就绪队列中，重新选择一个进程运行
pub fn suspend_current_and_run_next() -> isize {
    // 取出当前正在执行的任务
    let task_cp = current_task().unwrap();
    let mut task_inner = task_cp.write();
    if task_inner.pending_signals.contains(SigMask::SIGKILL) {
        let exit_code = task_inner.exit_code;
        drop(task_inner);
        drop(task_cp);
        exit_current_and_run_next(exit_code);
        return 0;
    }
    let task = take_current_task().unwrap();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    // 修改其进程控制块内的状态为就绪状态
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);

    // 将进程加入进程管理器中的就绪队列
    add_task(task);

    // 开启一轮新的调度
    schedule(task_cx_ptr);

    0
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // println!("[KERNEL] pid:{} exited", current_task().unwrap().pid.0);

    // 获取访问权限，修改进程状态
    let task = take_current_task().unwrap();
    remove_from_pid2task(task.pid());
    let mut inner = task.write();
    inner.task_status = TaskStatus::Zombie; // 后续才能被父进程在 waitpid 系统调用的时候回收
                                            // 记录退出码，后续父进程在 waitpid 的时候可以收集
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    if task.pid() == 0 {
        sync_all();
        panic!("initproc return!");
    }

    // 将这个进程的子进程转移到 initproc 进程的子进程中
    let mut initproc_inner = INITPROC.write();
    for child in inner.children.iter() {
        child.write().parent = Some(Arc::downgrade(&INITPROC));
        initproc_inner.children.push(child.clone()); // 引用计数 -1
    }
    drop(initproc_inner);

    // 引用计数 +1
    // 对于当前进程占用的资源进行早期回收
    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    drop(inner);
    drop(task);

    // 使用全0的上下文填充换出上下文，开启新一轮进程调度
    let mut _unused = TaskContext::empty();
    schedule(&mut _unused as *mut _);
}

pub fn hanging_current_and_run_next(sleep_time: usize, duration: usize) {
    let task = current_task().unwrap();
    let mut inner = task.write();
    let current_cx_ptr = &mut inner.task_cx as *mut TaskContext;
    inner.task_status = TaskStatus::Hanging;
    drop(inner);
    drop(task);
    acquire_processor().hang_current(sleep_time, duration);
    schedule(current_cx_ptr);
}

pub fn block_current_and_run_next() {
    let task = current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.write();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Blocking;
    block_task(task.clone());

    drop(task_inner);
    drop(task);

    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// 将初始进程 `initproc` 加入任务管理器
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn exec_signal_handlers() {
    let task = current_task().unwrap();
    let pid = task.pid();
    let mut task_inner = task.write();

    if task_inner.pending_signals == SigSet::empty() {
        return;
    }

    loop {
        // 取出 pending 的第一个 signal
        let signum = match task_inner.pending_signals.fetch() {
            Some(s) => s,
            None => return,
        };
        let sigaction = task_inner.sigactions[signum as usize];

        // 如果信号对应的处理函数存在，则做好跳转到 handler 的准备
        let handler = sigaction.sa_handler;
        match handler {
            SIG_IGN => {
                // return;
                continue; // loop
            }
            SIG_DFL => {
                if signum == Signal::SIGKILL as u32 || signum == Signal::SIGSEGV as u32 {
                    println!(
                        "[Kernel] task/mod(exec_signal_handlers) pid:{} signal_num:{}, SIG_DFL kill process",
                        pid, signum
                    );
                    drop(task_inner);
                    drop(task);
                    exit_current_and_run_next(-(signum as i32));
                }
                return;
            }
            _ => {
                // 阻塞当前信号以及 sigaction.sa_mask 中的信号
                let mut sigmask = sigaction.sa_mask.clone();
                if !sigaction.sa_flags.contains(SAFlags::SA_NODEFER) {
                    sigmask.add(signum);
                }

                // 保存旧的信号掩码
                let old_sigmask = task_inner.sigmask.clone();
                sigmask.add_other(old_sigmask);
                // 将信号掩码设置为 sigmask
                task_inner.sigmask = sigmask;

                // 将 SignalContext 数据放入栈中
                let trap_cx = task_inner.trap_context();
                // 保存 Trap 上下文与 old_sigmask 到 sig_context 中
                let sig_context = SignalContext::from_another(trap_cx, old_sigmask);
                trap_cx.x[2] -= core::mem::size_of::<SignalContext>(); // sp -= sizeof(sigcontext)
                trap_cx.x[10] = signum as usize; // a0 (args0 = signum)
                let token = current_user_token();
                let sig_context_ptr = trap_cx.x[2] as *mut SignalContext;
                *translated_mut(token, sig_context_ptr) = sig_context;

                if sigaction.sa_flags.contains(SAFlags::SA_SIGINFO) {
                    todo!("SA_SIGINFO")
                }

                // 将 sigreturn 的地址放入 ra 中
                extern "C" {
                    fn user_sigreturn();
                }
                trap_cx.x[1] = user_sigreturn as usize; // ra = user_sigreturn

                // debug!("prepare to jump to `handler`, original sepc = {:#x?}", trap_cx.sepc);
                trap_cx.sepc = handler; // sepc = handler
                return;
            }
        }
    }
}
