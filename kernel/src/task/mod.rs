mod aux_entry;
mod context;
mod id;
mod initproc;
mod kstack;
mod manager;
mod processor;
mod signals;
mod switch;
mod task;
pub use aux_entry::*;
pub use context::*;
pub use id::*;
pub use initproc::*;
pub use kstack::*;
pub use manager::*;
pub use processor::*;
pub use signals::*;
pub use switch::*;
pub use task::*;
use time_tracer::TimeTracer;

use crate::{
    consts::SIGNAL_TRAMPOLINE,
    mm::{copyout, translated_mut},
    syscall::impls::futex::futex_wake,
};
use alloc::sync::Arc;
use fat32::sync_all;

/// 将当前任务置为就绪态, 放回到进程管理器中的就绪队列中, 重新选择一个进程运行
pub fn suspend_current_and_run_next() -> isize {
    exec_signal_handlers();

    // 取出当前正在执行的任务
    let task = take_current_task().unwrap();
    let mut inner = task.inner_mut();
    let task_cx_ptr = &mut inner.task_cx as *mut TaskContext;

    // 修改其进程控制块内的状态为就绪状态
    inner.task_status = TaskStatus::Ready;
    drop(inner);

    // 将进程加入进程管理器中的就绪队列
    add_task(task);

    // 开启一轮新的调度
    schedule(task_cx_ptr);

    0
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();

    let pid = task.pid();
    let token = task.token();
    let is_child_thread = task.is_child_thread();

    remove_from_pid2task(pid);

    let mut inner = task.inner_mut();
    let clear_child_tid = inner.clear_child_tid;
    if clear_child_tid != 0 {
        *translated_mut(token, clear_child_tid as *mut usize) = 0;
        futex_wake(clear_child_tid, 1).unwrap();
    }
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;

    if pid == 0 {
        sync_all();
        panic!("initproc return!");
    }

    assert!(if is_child_thread {
        inner.children.is_empty()
    } else {
        true
    });

    // 将这个进程的子进程转移到 initproc 进程的子进程中
    // 若当前进程为子线程则不会执行下面的 for
    for child in inner.children.iter() {
        if child.is_child_thread() {
            child.inner_mut().parent = None; // weak reference 可以注释掉?
            continue;
        }
        let mut initproc_inner = INITPROC.inner_mut();
        child.inner_mut().parent = Some(Arc::downgrade(&INITPROC));
        initproc_inner.children.push(child.clone()); // 引用计数 -1
    }

    if is_child_thread {
        let parent = inner.parent.as_ref().unwrap().upgrade().unwrap();
        let mut parent_inner = parent.inner_mut();
        let children_iter = parent_inner.children.iter();
        let (idx, _) = children_iter
            .enumerate()
            .find(|(_, t)| t.pid() == pid)
            .unwrap();
        parent_inner.children.remove(idx);
        drop(parent_inner);
        drop(parent);
        drop(inner);
        assert!(Arc::strong_count(&task) == 1);
        collect_cancelled_chiled_thread(task);
        schedule(&mut TaskContext::empty() as *mut _);
        unreachable!()
    }

    drop(inner);
    drop(task);
    schedule(&mut TaskContext::empty() as *mut _);
}

pub fn hanging_current_and_run_next(sleep_time: usize, duration: usize) {
    let task = current_task();
    let mut inner = task.inner_mut();
    let current_cx_ptr = &mut inner.task_cx as *mut TaskContext;
    inner.task_status = TaskStatus::Hanging;
    drop(inner);
    drop(task);
    acquire_processor().hang_current(sleep_time, duration);
    schedule(current_cx_ptr);
}

pub fn block_current_and_run_next() {
    let task = current_task();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_mut();
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
    let task = current_task();
    let mut task_inner = task.inner_mut();

    if task_inner.pending_signals == SigSet::empty() {
        return;
    }

    loop {
        // 取出 pending 的第一个 signal
        let signum = match task_inner
            .pending_signals
            .difference(task_inner.sigmask)
            .fetch()
        {
            Some(s) => s,
            None => return,
        };
        task_inner.pending_signals.sub(signum);
        let sigaction = task.sigactions.read()[signum as usize];

        // 如果信号对应的处理函数存在, 则做好跳转到 handler 的准备
        let handler = sigaction.sa_handler;
        match handler {
            SIG_IGN => {
                // return;
                continue; // loop
            }
            SIG_DFL => {
                if signum == Signal::SIGKILL as u32 || signum == Signal::SIGSEGV as u32 {
                    // info!("[Kernel] task/mod(exec_signal_handlers) pid:{} signal_num:{}, SIG_DFL kill process", pid, signum);
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
                trap_cx.x[10] = signum as usize; // a0 (args0 = signum)
                                                 // 如果 sa_flags 中包含 SA_SIGINFO, 则将 siginfo 和 ucontext 放入栈中

                let memory_set = task.memory_set.read();
                let token = memory_set.token();
                drop(memory_set);

                trap_cx.x[2] -= core::mem::size_of::<UContext>(); // sp -= sizeof(ucontext)
                let ucontext_ptr = trap_cx.x[2];
                trap_cx.x[2] -= core::mem::size_of::<SigInfo>(); // sp -= sizeof(siginfo)
                let siginfo_ptr = trap_cx.x[2];

                trap_cx.x[11] = siginfo_ptr; // a1 (args1 = siginfo)
                trap_cx.x[12] = ucontext_ptr; // a2 (args2 = ucontext)
                let mut ucontext = UContext::empty();
                ucontext.uc_mcontext.greps[1] = trap_cx.sepc; //pc
                copyout(token, ucontext_ptr as *mut UContext, &ucontext);

                trap_cx.x[2] -= core::mem::size_of::<SignalContext>(); // sp -= sizeof(sigcontext)
                let sig_context_ptr = trap_cx.x[2] as *mut SignalContext;
                copyout(token, sig_context_ptr, &sig_context);

                trap_cx.x[1] = SIGNAL_TRAMPOLINE; // ra = user_sigreturn

                // println!("prepare to jump to `handler`:{:x?}, original sepc = {:#x?},current sp:{:x?}",handler, trap_cx.sepc, trap_cx.x[2]);

                trap_cx.sepc = handler; // sepc = handler
                return;
            }
        }
    }
}
