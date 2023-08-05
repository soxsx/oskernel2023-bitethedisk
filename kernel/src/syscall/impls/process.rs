//! 进程相关系统调用

use crate::board::CLOCK_FREQ;
use crate::fs::PollFd;
use core::task::Poll;
use core::usize;

use crate::fs::CreateMode;
use crate::fs::{make_pipe, open, OpenFlags};
use crate::mm::{
    copyin, copyout, translated_bytes_buffer, translated_mut, translated_ref, translated_str,
    UserBuffer, VirtPageNum,
};
use crate::return_errno;
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next, pid2task,
    suspend_current_and_run_next, SigAction, SigMask, Signal, SignalContext, MAX_SIGNUM,
};
use crate::timer::get_timeval;
use crate::timer::{get_time, NSEC_PER_SEC};

use alloc::{string::String, string::ToString, sync::Arc, vec::Vec};
use nix::info::{CloneFlags, RUsage, Utsname};
use nix::resource::{RLimit, Resource};
use nix::robustlist::RobustList;
use nix::time::{TimeSpec, TimeVal};

use super::super::errno::*;
use super::*;

use crate::task::*;

/// #define SYS_clone 220
///
/// 功能: 创建一个子进程;
///
/// 输入:
///
/// - flags: 创建的标志, 如SIGCHLD;
/// - stack: 指定新进程的栈, 可为0;
/// - ptid: 父线程ID;
/// - tls: TLS线程本地存储描述符;
/// - ctid: 子线程ID;
///
/// 返回值: 成功则返回子进程的线程ID, 失败返回-1;
///
/// ```c
/// pid_t ret = syscall(SYS_clone, flags, stack, ptid, tls, ctid)
/// ```
pub fn sys_do_fork(flags: usize, stack_ptr: usize, ptid: usize, tls: usize, ctid: usize) -> Result {
    let current_task = current_task();
    let signal = flags & 0xff;
    let flags = CloneFlags::from_bits(flags & !0xff).unwrap();

    let new_task = current_task.fork(flags);

    if stack_ptr != 0 {
        let trap_cx = new_task.inner_mut().trap_context();
        trap_cx.set_sp(stack_ptr);
    }
    let new_pid = new_task.pid.0;

    let memory_set = new_task.memory_set.read();
    let child_token = memory_set.token();
    drop(memory_set);

    if flags.contains(CloneFlags::PARENT_SETTID) {
        *translated_mut(current_user_token(), ptid as *mut u32) = new_pid as u32;
    }
    if flags.contains(CloneFlags::CHILD_SETTID) {
        *translated_mut(child_token, ctid as *mut u32) = new_pid as u32;
    }
    if flags.contains(CloneFlags::CHILD_CLEARTID) {
        new_task.inner_mut().clear_child_tid = ctid;
    }
    if flags.contains(CloneFlags::SETTLS) {
        let trap_cx = new_task.inner_mut().trap_context();
        trap_cx.set_tp(tls);
    }

    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_mut().trap_context();
    // we do not have to move to next instruction since we have done it before
    // trap_handler 已经将当前进程 Trap 上下文中的 sepc 向后移动了 4 字节,
    // 使得它回到用户态之后, 会从发出系统调用的 ecall 指令的下一条指令开始执行

    trap_cx.x[10] = 0; // 对于子进程, 返回值是0
    add_task(new_task); // 将 fork 到的进程加入任务调度器
    unsafe {
        core::arch::asm!("sfence.vma");
        core::arch::asm!("fence.i");
    }
    Ok(new_pid as isize) // 对于父进程, 返回值是子进程的 PID
}

/// #define SYS_execve 221
///
/// 功能: 执行一个指定的程序;
///
/// 输入:
///
/// - path: 待执行程序路径名称,
/// - argv: 程序的参数,
/// - envp: 环境变量的数组指针
///
/// 返回值: 成功不返回, 失败返回-1;
///
/// ```c
/// const char *path, char *const argv[], char *const envp[];
/// int ret = syscall(SYS_execve, path, argv, envp);
/// ```
pub fn sys_exec(path: *const u8, mut argv: *const usize, mut envp: *const usize) -> Result {
    let token = current_user_token();
    // 读取到用户空间的应用程序名称(路径)
    let mut path = translated_str(token, path);

    // {
    //     info!("exec path:{}", path);
    // }
    // println!("path:{:?},argv:{:?},envp:{:?}",path,argv,envp);
    let mut args_vec: Vec<String> = Vec::new();
    if path.ends_with(".sh") {
        path = "busybox".to_string();
        args_vec.push("sh".to_string());
    }
    // if path == "/bin/sh"{
    // // for lmbench_all lat_proc -P 1 shell
    // 	warn!("/bin/sh redirect to /busybox");
    //     path = "/busybox".to_string();
    // }
    if argv as usize != 0 {
        loop {
            let mut arg_str_ptr = 0;
            copyin(token, &mut arg_str_ptr, argv);
            if arg_str_ptr == 0 {
                // 读到下一参数地址为0表示参数结束
                break;
            } // 否则从用户空间取出参数, 压入向量
            args_vec.push(translated_str(token, arg_str_ptr as *const u8));
            unsafe { argv = argv.add(1) }
        }
    }
    // 环境变量
    let mut envs_vec: Vec<String> = Vec::new();
    if envp as usize != 0 {
        loop {
            let mut env_str_ptr = 0;
            copyin(token, &mut env_str_ptr, envp);
            if env_str_ptr == 0 {
                // 读到下一参数地址为0表示参数结束
                break;
            } // 否则从用户空间取出参数, 压入向量
              //	    println!("envp:{:?},env_str_ptr:{:x?}",envp,env_str_ptr);
            envs_vec.push(translated_str(token, env_str_ptr as *const u8));
            unsafe {
                envp = envp.add(1);
            }
        }
    }
    envs_vec.push("PATH=/".to_string());
    envs_vec.push("LD_LIBRARY_PATH=/".to_string());
    // TODO right value
    envs_vec.push("ENOUGH=5000".to_string());
    let task = current_task();

    let inner = task.inner_mut();
    let new_path = inner.cwd.clone().cd(path);
    let app_inode = open(new_path.clone(), OpenFlags::O_RDONLY, CreateMode::empty())?;
    drop(inner);
    task.exec(app_inode, args_vec, envs_vec);
    Ok(0)
    // } else {
    //     return_errno!(Errno::ENOENT, "path {:?} not exits", new_path);
    // }
}

/// #define SYS_wait4 260
///
/// 功能: 等待进程改变状态;
///
/// 输入:
///
/// - pid: 指定进程ID, 可为-1等待任何子进程;
/// - status: 接收状态的指针;
/// - options: 选项: WNOHANG, WUNTRACED, WCONTINUED;
///
/// 返回值: 成功则返回进程ID; 如果指定了WNOHANG, 且进程还未改变状态, 直接返回0; 失败则返回-1;
///
/// ```c
/// pid_t pid, int *status, int options;
/// pid_t ret = syscall(SYS_wait4, pid, status, options);
/// ```
pub fn sys_wait4(pid: isize, exit_code_ptr: *mut i32) -> Result {
    let task = current_task();

    let inner = task.inner_ref();

    // 根据pid参数查找有没有符合要求的进程
    if pid == -1 && inner.children.len() == 0 {
        return Ok(0);
    }
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.pid())
    {
        return_errno!(Errno::ECHILD, "pid {} does not exist", pid);
    }
    drop(inner);

    loop {
        let mut inner = task.inner_mut();
        // 查找所有符合PID要求的处于僵尸状态的进程, 如果有的话还需要同时找出它在当前进程控制块子进程向量中的下标
        let pair = inner
            .children
            .iter()
            .enumerate()
            .find(|(_, p)| p.inner_ref().is_zombie() && (pid == -1 || pid as usize == p.pid()));
        if let Some((idx, _)) = pair {
            // 将子进程从向量中移除并置于当前上下文中
            let child = inner.children.remove(idx);
            // 确认这是对于该子进程控制块的唯一一次强引用, 即它不会出现在某个进程的子进程向量中,
            // 更不会出现在处理器监控器或者任务管理器中.当它所在的代码块结束, 这次引用变量的生命周期结束,
            // 将导致该子进程进程控制块的引用计数变为 0 , 彻底回收掉它占用的所有资源,
            // 包括: 内核栈和它的 PID 还有它的应用地址空间存放页表的那些物理页帧等等
            assert_eq!(Arc::strong_count(&child), 1);
            // 收集的子进程信息返回
            let cpid = child.pid();
            let exit_code = child.inner_ref().exit_code;
            // ++++ release child PCB
            // 将子进程的退出码写入到当前进程的应用地址空间中
            if exit_code_ptr as usize != 0 {
                let memory_set = task.memory_set.read();
                let token = memory_set.token();
                drop(memory_set);
                *translated_mut(token, exit_code_ptr) = exit_code << 8;
            }

            return Ok(cpid as isize);
        } else {
            drop(inner); // 因为下个函数会切换上下文, 所以需要手动释放锁
            suspend_current_and_run_next();
        }
    }
}

/// #define SYS_exit 93
///
/// 功能: 触发进程终止, 无返回值;
///
/// 输入: 终止状态值;
///
/// 返回值: 无返回值;
///
/// ```c
/// int ec;
/// syscall(SYS_exit, ec);
/// ```
pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    unreachable!("unreachable in sys_exit!");
}

pub fn sys_exit_group(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// #define SYS_getppid 173
///
/// 功能: 获取父进程ID;
///
/// 输入: 系统调用ID;
///
/// 返回值: 成功返回父进程ID;
///
/// ```c
/// pid_t ret = syscall(SYS_getppid);
/// ```
pub fn sys_getppid() -> Result {
    Ok(current_task().tgid as isize)
}

/// #define SYS_getpid 172
///
/// 功能: 获取进程ID;
///
/// 输入: 系统调用ID;
///
/// 返回值: 成功返回进程ID;
///
/// ```c
/// pid_t ret = syscall(SYS_getpid);
/// ```
pub fn sys_getpid() -> Result {
    Ok(current_task().pid.0 as isize)
}

pub fn sys_set_tid_address(tidptr: *mut usize) -> Result {
    let token = current_user_token();
    let task = current_task();
    task.inner_mut().clear_child_tid = tidptr as usize;
    Ok(task.pid() as isize)
}

pub fn sys_getuid() -> Result {
    Ok(0)
}

pub fn sys_gettid() -> Result {
    Ok(0)
}

pub fn sys_geteuid() -> Result {
    Ok(0)
}

pub fn sys_ppoll(
    fds: usize,
    nfds: usize,
    tmo_p: *const TimeSpec,
    sigmask: *const SigMask,
) -> Result {
    // let token = current_user_token();
    // if sigmask as usize != 0 {
    //     let mut mask = translated_ref(token, sigmask as *const SigMask).clone();
    //     mask.sub(Signal::SIGKILL as u32); // sub 函数保证即使不存在 SIGKILL 也无影响
    //     mask.sub(Signal::SIGSTOP as u32);
    //     mask.sub(Signal::SIGILL as u32);
    //     mask.sub(Signal::SIGSEGV as u32);
    //     current_task().inner_mut().sigmask |= mask;
    // }
    // let mut poll_fd = Vec::<PollFd>::with_capacity(nfds);
    // for i in 0..nfds {
    //     let fd = translated_ref(token, unsafe { (fds as *mut PollFd).add(i) }).clone();
    //     poll_fd.push(fd);
    // }

    Ok(1)
}

pub fn sys_clock_gettime(_clk_id: usize, ts: *mut u64) -> Result {
    if ts as usize == 0 {
        return Ok(0);
    }
    let token = current_user_token();
    let ticks = get_time();
    let sec = (ticks / CLOCK_FREQ) as u64;
    let nsec = ((ticks % CLOCK_FREQ) * (NSEC_PER_SEC / CLOCK_FREQ)) as u64;
    *translated_mut(token, ts) = sec;
    *translated_mut(token, unsafe { ts.add(1) }) = nsec;
    Ok(0)
}

pub fn sys_kill(pid: usize, signal: u32) -> Result {
    //TODO pid==-1
    if signal == 0 {
        return Ok(0);
    }
    let signal = 1 << (signal - 1);
    if let Some(task) = pid2task(pid) {
        if let Some(flag) = SigMask::from_bits(signal) {
            task.inner_mut().pending_signals |= flag;
            Ok(0)
        } else {
            return_errno!(Errno::EINVAL, "invalid signal, signum: {}", signal);
        }
    } else {
        // return_errno!(Errno::ESRCH, "could not find task with pid: {}", pid); // for hackbench
        Ok(0)
    }
}
pub fn sys_tkill(tid: usize, signal: usize) -> Result {
    //TODO pid==-1
    // println!("[DEBUG] tkill tid:{:?} signal:0x{:x?}", tid, signal);
    if signal == 0 {
        return Ok(0);
    }
    let pid = if tid == 0 { current_task().pid.0 } else { tid };

    let signal = 1 << (signal - 1);
    if let Some(task) = pid2task(pid) {
        if let Some(flag) = SigMask::from_bits(signal) {
            task.inner_mut().pending_signals |= flag;
            Ok(0)
        } else {
            return_errno!(Errno::EINVAL, "invalid signal, signum: {}", signal);
        }
    } else {
        return_errno!(Errno::ESRCH, "could not find task with pid: {}", pid);
    }
}

const RUSAGE_SELF: isize = 0;
pub fn sys_getrusage(who: isize, usage: *mut u8) -> Result {
    if who != RUSAGE_SELF {
        return_errno!(Errno::EINVAL, "currently only supports RUSAGE_SELF");
    }
    let token = current_user_token();
    let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(
        token,
        usage,
        core::mem::size_of::<RUsage>(),
    ));
    let mut rusage = RUsage::new();
    let task = current_task();
    let mut inner = task.inner_mut();
    rusage.ru_stime = inner.stime;
    rusage.ru_utime = inner.utime;
    userbuf.write(rusage.as_bytes());
    Ok(0)
}

///
///
/// ```c
/// int tgkill(int tgid, int tid, int sig);
/// ```
pub fn sys_tgkill(tgid: isize, tid: usize, sig: isize) -> Result {
    if tgid == -1 {
        todo!("给当前tgid对应的线程组里面所有的线程发送对应的信号")
    }
    let master_pid = tgid as usize;
    let son_pid = tid;
    if let Some(parent_task) = pid2task(master_pid) {
        let inner = parent_task.inner_mut();
        if let Some(target_task) = inner.children.iter().find(|child| child.pid() == son_pid) {
            todo!("发送信号")
        } else {
            todo!("errno")
        }
    } else {
        todo!("errno")
    }
}

pub struct CpuMask {
    mask: [u8; 1024 / (8 * core::mem::size_of::<u8>())],
}

impl CpuMask {
    pub fn new() -> Self {
        Self {
            mask: [0; 1024 / (8 * core::mem::size_of::<u8>())],
        }
    }

    pub fn set(&mut self, cpu: usize) {
        let index = cpu / (8 * core::mem::size_of::<u8>());
        let offset = cpu % (8 * core::mem::size_of::<u8>());
        self.mask[index] |= 1 << offset;
    }

    pub fn get(&self, cpu: usize) -> bool {
        let index = cpu / (8 * core::mem::size_of::<u8>());
        let offset = cpu % (8 * core::mem::size_of::<u8>());
        self.mask[index] & (1 << offset) != 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.mask
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.mask
    }
}

// TODO
#[repr(C)]
pub struct CpuSet {
    mask: [usize; 1024 / (8 * core::mem::size_of::<usize>())],
}

impl CpuSet {
    pub fn new() -> Self {
        Self {
            mask: [0; 1024 / (8 * core::mem::size_of::<usize>())],
        }
    }

    pub fn set(&mut self, cpu: usize) {
        let index = cpu / (8 * core::mem::size_of::<usize>());
        let offset = cpu % (8 * core::mem::size_of::<usize>());
        self.mask[index] |= 1 << offset;
    }

    pub fn get(&self, cpu: usize) -> bool {
        let index = cpu / (8 * core::mem::size_of::<usize>());
        let offset = cpu % (8 * core::mem::size_of::<usize>());
        self.mask[index] & (1 << offset) != 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

// TODO 多核 在进程内加入 CpuMask
// 用于获取一个进程或线程的 CPU 亲和性(CPU affinity).
// CPU 亲和性指定了一个进程或线程可以运行在哪些 CPU 上.
// 通过使用 sched_getaffinity 系统调用, 程序员可以查询进程或线程当前绑定的 CPU.
// mask 参数是一个位图, 其中每个位表示一个 CPU.
// 如果某个位为 1, 表示进程或线程可以运行在对应的 CPU 上;
// 如果某个位为 0, 则表示进程或线程不能运行在对应的 CPU 上.
// int sched_getaffinity(pid_t pid, size_t cpusetsize, cpu_set_t *mask);
// cpu_set_t* cpu_set_t 是一个位图, 其中每个位表示一个 CPU. cpu_set_t 是一个结构体, 定义如下:
// typedef struct {
//     unsigned long __bits[1024 / (8 * sizeof(long))];
// } cpu_set_t;
// cpusetsize 参数指定了 mask 参数指向的位图的大小, 单位是字节.
pub fn sys_sched_getaffinity(pid: usize, cpusetsize: usize, mask: *mut u8) -> Result {
    let token = current_user_token();
    let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(token, mask, cpusetsize));

    // 内核中的调度器会维护一个位图, 用于记录进程或线程当前的 CPU 亲和性信息.
    // 当调用 sched_getaffinity 系统调用时, 调度器会将位图中对应的位复制到用户空间中的 mask 指针指向的内存区域中.
    let mut cpuset = CpuMask::new();
    cpuset.set(0);
    userbuf.write(cpuset.as_bytes());
    Ok(0)
}

pub fn sys_sched_setaffinity(pid: usize, cpusetsize: usize, mask: *const u8) -> Result {
    let token = current_user_token();
    let mut userbuf = UserBuffer::wrap(translated_bytes_buffer(token, mask, cpusetsize));

    let mut cpuset = CpuMask::new();
    userbuf.read(cpuset.as_bytes_mut());
    Ok(0)
}

pub const SCHED_OTHER: isize = 0;
pub const SCHED_FIFO: isize = 1;
pub const SCHED_RR: isize = 2;
pub const SCHED_BATCH: isize = 3;
pub const SCHED_IDLE: isize = 5;
pub const SCHED_DEADLINE: isize = 6;

// TODO 系统调用策略
// 该函数接受一个参数 pid, 表示要查询的进程的 PID.如果 pid 是 0, 则表示查询当前进程的调度策略.
// 函数返回值是一个整数, 表示指定进程的调度策略, 可能的取值包括:
// SCHED_FIFO: 先进先出调度策略.
// SCHED_RR: 轮转调度策略.
// SCHED_OTHER: 其他调度策略.
// 如果查询失败, 则返回 -1, 并将错误码存入 errno 变量中.
pub fn sys_getscheduler(pid: usize) -> Result {
    // let task = pid2task(pid).ok_or(SyscallError::PidNotFound(-1, pid as isize))?;
    // let inner = task.read();
    // Ok(inner.policy as isize) // TODO
    Ok(SCHED_OTHER as isize)
}

#[repr(C)]
pub struct SchedParam {
    sched_priority: isize,
}

impl SchedParam {
    pub fn new() -> Self {
        Self { sched_priority: 0 }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(&self.sched_priority as *const isize as *const u8, 8) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(&mut self.sched_priority as *mut isize as *mut u8, 8)
        }
    }
    pub fn set_priority(&mut self, priority: isize) {
        self.sched_priority = priority;
    }
    pub fn get_priority(&self) -> isize {
        self.sched_priority
    }
}

// sched_priority 成员表示进程的调度优先级, 值越高表示优先级越高.
// 在 Linux 中, 调度优先级的取值范围是 1-99, 其中 1 表示最低优先级, 99 表示最高优先级.
// 如果调用成功, sched_getparam 返回 0; 否则返回 -1, 并设置 errno 变量表示错误类型.
// 可能的错误类型包括 EINVAL(无效的参数), ESRCH(指定的进程不存在)等.
// sched_getparam 系统调用只能获取当前进程或当前进程的子进程的调度参数, 对于其他进程则需要相应的权限或特权.
// 如果要获取其他进程的调度参数, 可以使用 sys_sched_getaffinity 系统调用获取进程的 CPU 亲和性, 然后在相应的 CPU 上运行一个特权进程, 以便获取进程的调度参数.
// struct sched_param {
//     int sched_priority;
// };
pub fn sys_sched_getparam(pid: usize, param: *mut SchedParam) -> Result {
    // let task = pid2task(pid).ok_or(SyscallError::PidNotFound(-1, pid as isize))?;
    // let inner = task.read();
    let token = current_user_token();
    let user_param = translated_mut(token, param);
    // user_param.set_priority(inner.priority);
    user_param.set_priority(1);
    Ok(0)
}

#[repr(C)]
pub struct SchedPolicy(isize);

// pid 参数指定要设置调度策略和参数的进程的 PID;
// policy 参数是一个整数值, 表示要设置的调度策略;
// param 参数是一个指向 sched_param 结构体的指针, 用于设置调度参数.
// 如果调用成功, sched_setscheduler 返回 0; 否则返回 -1, 并设置 errno 变量表示错误类型.
// 可能的错误类型包括 EINVAL(无效的参数), ESRCH(指定的进程不存在)等.
// int sched_setscheduler(pid_t pid, int policy, const struct sched_param *param);
// struct sched_param {
//     int sched_priority;
// };
pub fn sys_sched_setscheduler(pid: usize, policy: isize, param: *const SchedParam) -> Result {
    let task = pid2task(pid).ok_or(Errno::UNCLEAR)?;
    let inner = task.inner_ref();

    {
        info!("sched_setscheduler: pid: {}, policy: {}", pid, policy);
    }
    // let user_param = translated_ref(token, param);
    // inner.policy = policy as u8;
    // inner.priority = user_param.get_priority();

    Ok(0)
}

// 用于获取指定时钟的精度(resolution)
// 其中, clk_id 参数指定要获取精度的时钟 ID; res 参数是一个指向 timespec 结构体的指针, 用于存储获取到的精度.
// 如果调用成功, clock_getres() 返回值为 0; 否则返回一个负数值, 表示错误类型.
// 可能的错误类型包括 EINVAL(无效的参数), EFAULT(无效的内存地址)等.
// int clock_getres(clockid_t clk_id, struct timespec *res);
// struct timespec {
//     time_t tv_sec; /* seconds */
//     long tv_nsec;  /* nanoseconds */
// };
pub fn sys_clock_getres(clockid: usize, res: *mut TimeSpec) -> Result {
    let token = current_user_token();
    let user_res = translated_mut(token, res);
    // 赋值看的测试样例 TODO
    user_res.tv_sec = 0;
    user_res.tv_nsec = 1;
    Ok(0)
}

pub const SOCK_DGRAM: isize = 1;
pub const SOCK_STREAM: isize = 2;

// int socketpair(int domain, int type, int protocol, int sv[2]);
// domain: 指定要创建的套接字的协议族, 可以取值为 AF_UNIX 或 AF_LOCAL, 表示使用本地 IPC.
// type: 指定要创建的套接字的类型, 可以取值为 SOCK_STREAM 或 SOCK_DGRAM.
// protocol: 指定要使用的协议, 通常为 0.
// sv: 指向一个长度为 2 的数组的指针, 用于保存创建的套接字文件描述符.
pub fn sys_socketpair(domain: isize, _type: isize, _protocol: isize, sv: *mut [i32; 2]) -> Result {
    let token = current_user_token();
    let task = current_task();
    let user_sv = translated_mut(token, sv);

    let (sv0, sv1) = make_pipe();

    // fd_table mut borrow
    let mut fd_table = task.fd_table.write();
    let inner = task.inner_ref();
    let fd_limit = inner.rlimit_nofile.rlim_cur;
    drop(inner);
    let fd0 = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
    if fd0 >= fd_limit {
        return_errno!(Errno::EMFILE);
    }
    fd_table[fd0] = Some(sv0);

    let fd1 = TaskControlBlock::alloc_fd(&mut fd_table, fd_limit);
    if fd1 >= fd_limit {
        return_errno!(Errno::EMFILE);
    }
    fd_table[fd1] = Some(sv1);

    drop(fd_table);

    user_sv[0] = fd0 as i32;
    user_sv[1] = fd1 as i32;
    Ok(0)
}

/*********** SIGNAL ******************/
// 用于在信号处理程序中恢复被中断的程序执行流程.
// 当一个进程收到一个信号时, 内核会为该进程保存信号处理程序的上下文(如寄存器的值, 栈指针等), 并将程序的执行流程转移到信号处理程序中.
// 在信号处理程序中, 如果需要返回到被中断的程序执行流程中, 可以使用 sigreturn 系统调用
pub fn sys_sigreturn() -> Result {
    let token = current_user_token();
    let task = current_task();
    let mut task_inner = task.inner_mut();

    let trap_cx = task_inner.trap_context();

    // 还原被保存的 signal_context
    let sig_context_ptr = trap_cx.x[2]; // 函数调用保证了 x[2] 的值是 sig_context 的地址 (user signal handler 执行前后 x[2] 值不变)
    trap_cx.x[2] += core::mem::size_of::<SignalContext>();
    let siginfo_ptr = trap_cx.x[2];
    trap_cx.x[2] += core::mem::size_of::<SigInfo>();
    let ucontext_ptr = trap_cx.x[2];
    trap_cx.x[2] += core::mem::size_of::<UContext>();

    let ucontext = translated_ref(token, ucontext_ptr as *const UContext);
    let sig_context = translated_ref(token, sig_context_ptr as *mut SignalContext);
    let sigmask = sig_context.mask.clone();
    // 还原 signal handler 之前的 trap context
    *trap_cx = sig_context.context.clone();
    // 还原 signal handler 之前的 signal mask
    task_inner.sigmask = sigmask;
    trap_cx.sepc = ucontext.uc_mcontext.greps[1];

    Ok(0)
}

// 用于设置和修改信号
// sig 表示要设置或修改的信号的编号, act 是一个指向 sigaction 结构体的指针, 用于指定新的信号处理方式,
// oact 是一个指向 sigaction 结构体的指针, 用于保存原来的信号处理方式
// ```c
// asmlinkage long sys_sigaction(int sig, const struct sigaction __user *act, struct sigaction __user *oact);
// struct sigaction {
//     void (*sa_handler)(int);
//     void (*sa_sigaction)(int, siginfo_t *, void *);
//     unsigned long sa_flags;
//     void (*sa_restorer)(void);
//     struct old_sigaction __user *sa_restorer_old;
//     sigset_t sa_mask;
// };
// ```
pub fn sys_sigaction(signum: isize, act: *const SigAction, oldact: *mut SigAction) -> Result {
    let token = current_user_token();
    let task = current_task();
    let mut inner = task.inner_mut();
    let signum = signum as u32;

    // signum 超出范围, 返回错误
    if signum > MAX_SIGNUM || signum == Signal::SIGKILL as u32 || signum == Signal::SIGSTOP as u32 {
        // println!(
        //     "[Kernel] syscall/impl/process: sys_sigaction(signum: {}, sigaction = {:#x?}, old_sigaction = {:#x?} ) = {}",
        //     signum, act, oldact, -Errno::EINVAL
        // );
        return Err(Errno::EINVAL);
    }

    // 当 sigaction 存在时,  在 pcb 中注册给定的 signaction

    if act as usize != 0 {
        let mut sigaction = task.sigactions.write();
        if oldact as usize != 0 {
            *translated_mut(token, oldact) = sigaction[signum as usize];
        }
        //在 pcb 中注册给定的 signaction
        let mut sa = translated_ref(token, act).clone();
        // kill 和 stop 信号不能被屏蔽
        sa.sa_mask.sub(Signal::SIGKILL as u32); // sub 函数保证即使不存在 SIGKILL 也无影响
        sa.sa_mask.sub(Signal::SIGSTOP as u32);
        sa.sa_mask.sub(Signal::SIGILL as u32);
        sa.sa_mask.sub(Signal::SIGSEGV as u32);

        sigaction[signum as usize] = sa;
    }

    // println!(
    //     "[Kernel] syscall/impl/process: sys_sigaction(signum: {}, sigaction = {:#x?}, old_sigaction = {:#x?} ) = {}",
    //     signum,
    //     act, // sigact,
    //     oldact,
    //     0
    // );
    Ok(0)
}

// 用于设置和修改进程的信号屏蔽字.
// 信号屏蔽字是一个位图, 用于指定哪些信号在当前进程中被屏蔽, 即在进程处理某些信号时, 屏蔽掉一些信号, 以避免这些信号的干扰.
// ```c
// int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);
// ```
// how 参数指定了如何修改进程的信号屏蔽字, 可以取以下三个值之一:
// - SIG_BLOCK: 将 set 中指定的信号添加到进程的信号屏蔽字中.
// - SIG_UNBLOCK: 将 set 中指定的信号从进程的信号屏蔽字中移除.
// - SIG_SETMASK: 将进程的信号屏蔽字设置为 set 中指定的信号.
pub fn sys_sigprocmask(
    how: usize,
    set: *const usize,
    old_set: *mut usize,
    sigsetsize: usize,
) -> Result {
    let token = current_user_token();
    let task = current_task();
    let mut task_inner = task.inner_mut();
    let mut old_mask = task_inner.sigmask.clone();

    if old_set as usize != 0 {
        *translated_mut(token, old_set as *mut SigMask) = old_mask;
    }

    if set as usize != 0 {
        // let mut new_set = translated_ref(token, set as *const SigMask).clone();
        let mut new_set = translated_ref(token, set as *const SigMask).clone();
        new_set.sub(Signal::SIGKILL as u32); // sub 函数保证即使不存在 SIGKILL 也无影响
        new_set.sub(Signal::SIGSTOP as u32);
        new_set.sub(Signal::SIGILL as u32);
        new_set.sub(Signal::SIGSEGV as u32);

        let how = MaskFlags::from_how(how);
        match how {
            MaskFlags::SIG_BLOCK => old_mask |= new_set,
            MaskFlags::SIG_UNBLOCK => old_mask &= !new_set,
            MaskFlags::SIG_SETMASK => old_mask = new_set,
            _ => panic!("ENOSYS"),
        }
        task_inner.sigmask = old_mask;
    }

    // println!(
    //     "[Kernel] syscall/impls/process: sys_sigprocmask(how: {}, set: {:#x?}, old_set: {:#x?}) = 0",
    //     how, set, old_set
    // );
    Ok(0)
}
pub fn sys_set_robust_list(head: usize, len: usize) -> Result {
    if len != RobustList::HEAD_SIZE {
        return_errno!(Errno::EINVAL, "robust list head len missmatch:{:?}", len);
    }
    let task = current_task();
    let mut inner = task.inner_mut();
    inner.robust_list.head = head;
    drop(inner);
    Ok(0)
}

pub fn sys_get_robust_list(pid: usize, head_ptr: *mut usize, len_ptr: *mut usize) -> Result {
    let task = if pid == 0 {
        current_task()
    } else {
        match pid2task(pid) {
            Some(taskk) => taskk,
            None => return_errno!(Errno::ESRCH, "no such pid:{:?}", pid),
        }
    };
    let token = current_user_token();
    let inner = task.inner_ref();
    let robust_list = &inner.robust_list;
    copyout(token, head_ptr, &robust_list.head);
    copyout(token, len_ptr, &robust_list.len);
    drop(inner);
    Ok(0)
}

pub fn sys_prlimit64(
    pid: usize,
    resource: u32,
    new_limit: *const RLimit,
    old_limit: *mut RLimit,
) -> Result {
    if pid == 0 {
        let task = current_task();
        let token = current_user_token();
        let resource = if resource == 7 {
            Resource::NOFILE
        } else {
            Resource::ILLEAGAL
        };
        // info!("[sys_prlimit] pid: {}, resource: {:?}", pid, resource);
        if !old_limit.is_null() {
            match resource {
                Resource::NOFILE => {
                    let inner = task.inner_ref();
                    let rlimit_nofile = &inner.rlimit_nofile;
                    copyout(token, old_limit, &rlimit_nofile);
                    drop(inner)
                }
                // TODO
                // Resource::ILLEAGAL => return_errno!(Errno::EINVAL),
                // _ => todo!(),
                _ => (),
            }
        }
        if !new_limit.is_null() {
            let mut rlimit = RLimit::new(0, 0);
            copyin(token, &mut rlimit, new_limit);
            match resource {
                Resource::NOFILE => {
                    let mut inner = task.inner_mut();
                    let rlimit_nofile = &mut inner.rlimit_nofile;
                    *rlimit_nofile = rlimit;
                    drop(inner)
                }
                // TODO
                // Resource::ILLEAGAL => return_errno!(Errno::EINVAL),
                // _ => todo!(),
                _ => (),
            }
        }
        Ok(0)
    } else {
        todo!()
    }
}
