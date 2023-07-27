use core::fmt::Debug;
use core::mem;

use super::kernel_stack::KernelStack;
use super::{pid_alloc, PidHandle, SigMask, SigSet};
use super::{SigAction, TaskContext, MAX_SIGNUM};
use crate::consts::*;
use crate::fs::{file::File, Fat32File, Stdin, Stdout};
use crate::mm::kernel_vmm::acquire_kvmm;
use crate::mm::memory_set::{AuxEntry, LoadedELF};
use crate::mm::MmapFlags;
use crate::mm::{
    translated_mut, MemorySet, MmapProts, PageTableEntry, PhysPageNum, VirtAddr, VirtPageNum,
};
use crate::trap::handler::user_trap_handler;
use crate::trap::TrapContext;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use nix::time::TimeVal;
use nix::CloneFlags;
use riscv::register::scause::Scause;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::fs::AbsolutePath;

pub const FD_LIMIT: usize = 1024;

// TODO futex, chan, sigmask, sigpending, rubustlist, rlimt,
pub struct TaskControlBlock {
    /// 进程标识符
    pub pid: PidHandle,
    /// thread group id
    pub tgid: usize,
    // pub set_child_tid: usize,   /* CLONE_CHILD_SETTID */
    // pub clear_child_tid: usize, /* CLONE_CHILD_CLEARTID */
    /// 应用内核栈
    pub kernel_stack: KernelStack,

    pub sigactions: Arc<RwLock<[SigAction; MAX_SIGNUM as usize]>>,
    pub memory_set: Arc<RwLock<MemorySet>>,
    pub fd_table: Arc<RwLock<FDTable>>,

    inner: RwLock<TaskControlBlockInner>,
}

impl Debug for TaskControlBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaskControlBlock")
            .field("pid", &self.pid.0)
            .finish()
    }
}

pub struct TaskControlBlockInner {
    /// 应用地址空间中的 Trap 上下文所在的物理页帧的物理页号
    pub trap_cx_ppn: PhysPageNum,

    /// 任务上下文
    pub task_cx: TaskContext,

    pub task_status: TaskStatus,

    /// 指向当前进程的父进程（如果存在的话）
    pub parent: Option<Weak<TaskControlBlock>>,

    /// 当前进程的所有子进程的任务控制块向量
    pub children: Vec<Arc<TaskControlBlock>>,

    /// 退出码
    pub exit_code: i32,

    pub pending_signals: SigSet,

    pub sigmask: SigMask,

    pub cwd: AbsolutePath,

    pub utime: TimeVal,

    pub stime: TimeVal,

    pub last_enter_umode_time: TimeVal,

    pub last_enter_smode_time: TimeVal,
    pub clear_child_tid: usize, /* CLONE_CHILD_CLEARTID */

    pub trap_cause: Option<Scause>,
}

pub type FDTable = Vec<Option<Arc<dyn File>>>;

impl TaskControlBlockInner {
    pub fn trap_context(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.as_mut()
    }

    fn status(&self) -> TaskStatus {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.status() == TaskStatus::Zombie
    }

    pub fn get_work_path(&self) -> AbsolutePath {
        self.cwd.clone()
    }

    pub fn add_utime(&mut self, new_time: TimeVal) {
        self.utime = self.utime + new_time;
    }

    pub fn add_stime(&mut self, new_time: TimeVal) {
        self.stime = self.stime + new_time;
    }

    pub fn set_last_enter_umode(&mut self, new_time: TimeVal) {
        self.last_enter_umode_time = new_time;
    }

    pub fn set_last_enter_smode(&mut self, new_time: TimeVal) {
        self.last_enter_smode_time = new_time;
    }
}

impl TaskControlBlock {
    /// 查找空闲文件描述符下标
    ///
    /// 从文件描述符表中 **由低到高** 查找空位，返回向量下标，没有空位则在最后插入一个空位
    pub fn alloc_fd(fd_table: &mut FDTable) -> usize {
        if let Some(fd) = (0..fd_table.len()).find(|fd| fd_table[*fd].is_none()) {
            fd
        } else {
            if fd_table.len() == FD_LIMIT {
                return FD_LIMIT;
            }
            fd_table.push(None);
            fd_table.len() - 1
        }
    }

    pub fn inner_mut(&self) -> RwLockWriteGuard<'_, TaskControlBlockInner> {
        self.inner.write()
    }

    pub fn inner_ref(&self) -> RwLockReadGuard<'_, TaskControlBlockInner> {
        self.inner.read()
    }

    /// 通过 elf 数据新建一个任务控制块，目前仅用于内核中手动创建唯一一个初始进程 initproc
    pub fn new(elf: Arc<dyn File>) -> Self {
        // 解析传入的 ELF 格式数据构造应用的地址空间 memory_set 并获得其他信息
        let LoadedELF {
            memory_set,
            elf_entry: entry_point,
            user_stack_top: user_sp,
            auxs,
        } = MemorySet::load_elf(elf.clone());
        // 从地址空间 memory_set 中查多级页表找到应用地址空间中的 Trap 上下文实际被放在哪个物理页帧
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 为进程分配 PID 以及内核栈，并记录下内核栈在内核地址空间的位置
        let pid_handle = pid_alloc();
        let tgid = pid_handle.0;
        let pgid = pid_handle.0;
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.top();

        // 在该进程的内核栈上压入初始化的任务上下文，使得第一次任务切换到它的时候可以跳转到 trap_return 并进入用户态开始执行
        let task_control_block = Self {
            pid: pid_handle,
            tgid,
            kernel_stack,
            memory_set: Arc::new(RwLock::new(memory_set)),
            fd_table: Arc::new(RwLock::new(vec![
                // 0 -> stdin
                Some(Arc::new(Stdin)),
                // 1 -> stdout
                Some(Arc::new(Stdout)),
                // 2 -> stderr
                Some(Arc::new(Stdout)),
            ])),

            // set_child_tid: 0,
            // clear_child_tid: 0,
            inner: RwLock::new(TaskControlBlockInner {
                trap_cx_ppn,
                task_cx: TaskContext::readied_for_switching(kernel_stack_top),
                task_status: TaskStatus::Ready,
                parent: None,
                children: Vec::new(),
                exit_code: 0,
                sigmask: SigMask::empty(),
                pending_signals: SigSet::empty(),
                cwd: AbsolutePath::from_str("/"),
                utime: TimeVal { sec: 0, usec: 0 },
                stime: TimeVal { sec: 0, usec: 0 },
                last_enter_umode_time: TimeVal { sec: 0, usec: 0 },
                last_enter_smode_time: TimeVal { sec: 0, usec: 0 },
                clear_child_tid: 0,
                trap_cause: None,
            }),
            sigactions: Arc::new(RwLock::new([SigAction::new(); MAX_SIGNUM as usize])),
        };
        // 初始化位于该进程应用地址空间中的 Trap 上下文，使得第一次进入用户态的时候时候能正
        // 确跳转到应用入口点并设置好用户栈，同时也保证在 Trap 的时候用户态能正确进入内核态
        let trap_cx = task_control_block.inner_mut().trap_context();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            acquire_kvmm().token(),
            kernel_stack_top,
            user_trap_handler as usize,
        );
        task_control_block
    }

    pub fn init_ustack(
        &self,
        user_sp: usize,
        args: Vec<String>,
        envs: Vec<String>,
        auxv: &mut Vec<AuxEntry>,
    ) -> (usize, usize, usize) {
        let memory_set = self.memory_set.read();
        let token = memory_set.token();
        drop(memory_set);

        // 计算共需要多少字节的空间
        let mut total_len = 0;
        for i in 0..envs.len() {
            total_len += envs[i].len() + 1; // String 不包含 '\0'
        }
        for i in 0..args.len() {
            total_len += args[i].len() + 1;
        }

        let mut user_sp = user_sp;
        // 先进行进行对齐
        // let align = core::mem::size_of::<usize>() / core::mem::size_of::<u8>(); // 8
        // let mut user_sp = user_sp - (align - total_len % align) * core::mem::size_of::<u8>();
        // user_sp -= core::mem::size_of::<usize>();
        // *translated_mut(token, user_sp as *mut usize) = 123;
        // user_sp -= core::mem::size_of::<usize>();
        // *translated_mut(token, user_sp as *mut usize) = 456;

        // 分配 envs 的空间, 加入动态链接库位置
        let envs_ptrv: Vec<_> = (0..envs.len())
            .map(|idx| {
                user_sp -= envs[idx].len() + 1; // 1 是手动添加结束标记的空间('\0')
                let mut ptr = user_sp;
                for c in envs[idx].as_bytes() {
                    // 将参数写入到用户栈
                    *translated_mut(token, ptr as *mut u8) = *c;
                    ptr += 1;
                } // 写入字符串结束标记
                *translated_mut(token, ptr as *mut u8) = 0;
                user_sp
            })
            .collect();

        // 分配 args 的空间, 并写入字符串数据, 把字符串首地址保存在 argv 中
        // 这里高地址放前面的参数, 即先存放 argv[0]
        let args_ptrv: Vec<_> = (0..args.len())
            .map(|idx| {
                user_sp -= args[idx].len() + 1; // 1 是手动添加结束标记的空间('\0')
                let mut ptr = user_sp;
                for c in args[idx].as_bytes() {
                    // 将参数写入到用户栈
                    *translated_mut(token, ptr as *mut u8) = *c;
                    ptr += 1;
                } // 写入字符串结束标记
                *translated_mut(token, ptr as *mut u8) = 0;
                user_sp
            })
            .collect();

        // padding 0 表示结束 AT_NULL aux entry
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = 0;

        // 分配 auxs 空间，并写入数据
        for i in 0..auxv.len() {
            user_sp -= core::mem::size_of::<AuxEntry>();
            *translated_mut(token, user_sp as *mut AuxEntry) = auxv[i];
        }
        // auxv.push(AuxEntry(AT_EXECFN,args_ptrv[0] ));

        // padding 0 表示结束
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = 0;

        // envs_ptr
        user_sp -= (envs.len()) * core::mem::size_of::<usize>();
        let envs_ptr_base = user_sp; // 参数字符串指针起始地址
        for i in 0..envs.len() {
            *translated_mut(
                token,
                (envs_ptr_base + i * core::mem::size_of::<usize>()) as *mut usize,
            ) = envs_ptrv[i];
        }

        // padding 0 表示结束
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = 0;

        // args_ptr
        user_sp -= (args.len()) * core::mem::size_of::<usize>();
        let args_ptr_base = user_sp; // 参数字符串指针起始地址
        for i in 0..args.len() {
            *translated_mut(
                token,
                (args_ptr_base + i * core::mem::size_of::<usize>()) as *mut usize,
            ) = args_ptrv[i];
        }

        // argc
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = args.len();

        (user_sp, args_ptr_base as usize, envs_ptr_base as usize)
    }

    /// 用来实现 exec 系统调用，即当前进程加载并执行另一个 ELF 格式可执行文件
    pub fn exec(&self, elf_file: Arc<dyn File>, args: Vec<String>, envs: Vec<String>) {
        // 从 ELF 文件生成一个全新的地址空间并直接替换
        let LoadedELF {
            memory_set,
            user_stack_top: user_sp,
            elf_entry: entry_point,
            mut auxs,
        } = MemorySet::load_elf(elf_file);

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // memory_set
        // 这将导致原有的地址空间生命周期结束, 里面包含的全部物理页帧都会被回收,
        // 结果表现为: 原有的地址空间中的所有页表项的 ppn 引用计数减 1
        let mut ms = self.memory_set.write();
        *ms = memory_set;
        drop(ms); // 避免接下来的操作导致死锁

        // fd_table
        let mut fd_table = self.fd_table.write();
        fd_table
            .iter_mut()
            .find(|fd| fd.is_some() && !fd.as_ref().unwrap().available())
            .take(); // TODO

        let mut inner = self.inner_mut();
        inner.trap_cx_ppn = trap_cx_ppn;
        let trap_cx = inner.trap_context();
        drop(inner); // 避免接下来的操作导致死锁

        let (user_sp, _args_ptr, _envs_ptr) = self.init_ustack(user_sp, args, envs, &mut auxs);
        // 修改新的地址空间中的 Trap 上下文，将解析得到的应用入口点、用户栈位置以及一些内核的信息进行初始化
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            acquire_kvmm().token(),
            self.kernel_stack.top(),
            user_trap_handler as usize,
        );
    }

    /// 用来实现 fork 系统调用，即当前进程 fork 出来一个与之几乎相同的子进程
    pub fn fork(self: &Arc<TaskControlBlock>, flags: CloneFlags) -> Arc<TaskControlBlock> {
        let memory_set = MemorySet::from_copy_on_write(&mut self.memory_set.write()); // use 4 pages
                                                                                      // 拷贝用户地址空间
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // 分配一个 PID
        let pid_handle = pid_alloc();
        let tgid = if flags.contains(CloneFlags::THREAD) {
            self.pid.0
        } else {
            pid_handle.0
        };

        // 根据 PID 创建一个应用内核栈
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.top();

        // copy fd table
        let mut fd_table = Vec::new();
        // parent fd table
        let pfd_table_ref = self.fd_table.read();
        for fd in pfd_table_ref.iter() {
            if let Some(file) = fd {
                fd_table.push(Some(file.clone()));
            } else {
                fd_table.push(None);
            }
        }

        let sigactions = if flags.contains(CloneFlags::SIGHAND) {
            self.sigactions.clone()
        } else {
            // parent sigactions
            let psa_ref = self.sigactions.read();
            let mut sa = Arc::new(RwLock::new([SigAction::new(); MAX_SIGNUM as usize]));
            let mut sa_mut = sa.write();
            for i in 1..MAX_SIGNUM as usize {
                sa_mut[i] = psa_ref[i].clone();
            }
            drop(sa_mut);
            sa
        };

        let mut parent_inner = self.inner_mut();

        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            tgid,
            memory_set: Arc::new(RwLock::new(memory_set)),
            fd_table: Arc::new(RwLock::new(fd_table)),
            sigactions,

            // set_child_tid: 0,
            // clear_child_tid: 0,
            kernel_stack,
            inner: RwLock::new(TaskControlBlockInner {
                trap_cx_ppn,
                task_cx: TaskContext::readied_for_switching(kernel_stack_top),
                task_status: TaskStatus::Ready,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,

                // [signal: msg about fork](https://man7.org/linux/man-pages/man7/signal.7.html)
                sigmask: parent_inner.sigmask.clone(),
                pending_signals: SigSet::empty(),

                cwd: parent_inner.cwd.clone(),
                utime: TimeVal { sec: 0, usec: 0 },
                stime: TimeVal { sec: 0, usec: 0 },
                last_enter_umode_time: TimeVal { sec: 0, usec: 0 },
                last_enter_smode_time: TimeVal { sec: 0, usec: 0 },
                clear_child_tid: 0,
                trap_cause: None,
            }),
        });

        // 把新生成的进程加入到子进程向量中
        parent_inner.children.push(task_control_block.clone());
        // 更新子进程 trap 上下文中的栈顶指针
        let trap_cx = task_control_block.inner_mut().trap_context();
        trap_cx.kernel_sp = kernel_stack_top;

        task_control_block
    }

    /// 尝试用时加载缺页，目前只支持mmap缺页
    ///
    /// - 参数：
    ///     - `va`：缺页中的虚拟地址
    ///     - `is_load`：加载(1)/写入(0)
    /// - 返回值：
    ///     - `0`：成功加载缺页
    ///     - `-1`：加载缺页失败
    ///
    /// 分别用于：
    ///     - 用户态：handler page fault
    ///     - 内核态： translate_bytes_buffer
    pub fn check_lazy(&self, va: VirtAddr, is_load: bool) -> isize {
        let mut memory_set = self.memory_set.write();

        let mmap_start = memory_set.mmap_manager.mmap_start;
        let mmap_end = memory_set.mmap_manager.mmap_top;
        let heap_start = VirtAddr::from(memory_set.brk_start);
        let heap_end = VirtAddr::from(memory_set.brk_start + USER_HEAP_SIZE);

        // fork
        let vpn: VirtPageNum = va.floor();
        let pte = memory_set.translate(vpn);
        if pte.is_some() && pte.unwrap().is_cow() {
            let former_ppn = pte.unwrap().ppn();
            return memory_set.cow_alloc(vpn, former_ppn);
        } else {
            if let Some(pte1) = pte {
                if pte1.is_valid() {
                    return -4;
                }
            }
        }

        // println!("check_lazy: va: {:#x}", va.0);

        // lazy map / lazy alloc heap
        if va >= heap_start && va <= heap_end {
            memory_set.lazy_alloc_heap(va.floor())
        } else if va >= mmap_start && va < mmap_end {
            memory_set.lazy_mmap(vpn);
            0
        } else {
            warn!("[check_lazy] {:x?}", va);
            warn!("[check_lazy] mmap_start: 0x{:x}", mmap_start.0);
            warn!("[check_lazy] mmap_end: 0x{:x}", mmap_end.0);
            warn!("[check_lazy] heap_start: 0x{:x}", heap_start.0);
            warn!("[check_lazy] heap_end: 0x{:x}", heap_end.0);
            -2
        }
    }

    // 在进程虚拟地址空间中分配创建一片虚拟内存地址映射
    pub fn mmap(
        &self,
        addr: usize,
        length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        fd: isize,
        offset: usize,
    ) -> usize {
        if addr % PAGE_SIZE != 0 {
            panic!("mmap: addr not aligned");
        }

        let fd_table = self.fd_table.read().clone();
        // memory_set mut borrow
        let mut ms_mut = self.memory_set.write();
        let mut start_va = VirtAddr::from(0);
        // "prot<<1" 右移一位以符合 MapPermission 的权限定义
        // "1<<4" 增加 MapPermission::U 权限
        if addr == 0 {
            start_va = ms_mut.mmap_manager.get_mmap_top();
        }

        if flags.contains(MmapFlags::MAP_FIXED) {
            start_va = VirtAddr::from(addr);
            ms_mut.mmap_manager.remove(start_va, length);
        }
        let file = if flags.contains(MmapFlags::MAP_ANONYMOUS) {
            None
        } else {
            fd_table[fd as usize].clone()
        };
        ms_mut
            .mmap_manager
            .push(start_va, length, prot, flags, offset, file);
        drop(ms_mut);
        start_va.0
    }

    pub fn munmap(&self, addr: usize, length: usize) -> isize {
        let start_va = VirtAddr(addr);
        // 可能会有 mmap 后没有访问直接 munmap 的情况，需要检查是否访问过 mmap 的区域(即
        // 是否引发了 lazy_mmap)，防止 unmap 页表中不存在的页表项引发 panic
        self.memory_set
            .write()
            .mmap_manager
            .remove(start_va, length);
        0
    }

    pub fn pid(&self) -> usize {
        self.pid.0
    }

    pub fn grow_proc(&self, grow_size: isize) -> usize {
        // memory_set mut borrow
        let mut ms_mut = self.memory_set.write();
        let brk = ms_mut.brk;
        let brk_start = ms_mut.brk_start;
        if grow_size > 0 {
            let growed_addr: usize = brk + grow_size as usize;
            let limit = brk_start + USER_HEAP_SIZE;
            if growed_addr > limit {
                panic!(
                    "process doesn't have enough memsize to grow! limit:0x{:x}, heap_pt:0x{:x}, growed_addr:0x{:x}, pid:{}",
                    limit,
                    brk,
                    growed_addr,
                    self.pid.0
                );
            }
            ms_mut.brk = growed_addr;
        } else {
            let shrinked_addr: usize = brk + grow_size as usize;
            if shrinked_addr < brk_start {
                panic!("Memory shrinked to the lowest boundary!")
            }
            ms_mut.brk = shrinked_addr;
        }
        return ms_mut.brk;
    }
}

/// 任务状态枚举
///
/// |状态|描述|
/// |--|--|
/// |`Ready`|准备运行|
/// |`Running`|正在运行|
/// |`Zombie`|僵尸态|
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Ready,    // 准备运行
    Running,  // 正在运行
    Blocking, // 阻塞态
    Hanging,  // 挂起态
    Zombie,   // 僵尸态
}
