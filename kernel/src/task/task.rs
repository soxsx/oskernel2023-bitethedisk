use super::kstack::KernelStack;
use super::TaskContext;
use super::{pid_alloc, PidHandle, SigSet};
use crate::consts::*;
use crate::fs::{File, Stdin, Stdout};
use crate::mm::acquire_kvmm;
use crate::mm::copyout;
use crate::mm::LoadedELF;
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, VirtPageNum};
use crate::trap::user_trap_handler;
use crate::trap::TrapContext;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;
use nix::time::TimeVal;
use nix::{
    AuxEntry, CloneFlags, IntervalTimer, MmapFlags, MmapProts, RLimit, RobustList, SigAction,
    SigMask, MAX_SIGNUM,
};
use path::AbsolutePath;
use riscv::register::scause::Scause;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(feature = "static_busybox")]
use super::initproc::{STATIC_BUSYBOX_AUX, STATIC_BUSYBOX_ENTRY};

pub struct TaskControlBlock {
    pub pid: PidHandle,
    pub tgid: usize,
    pub kernel_stack: KernelStack,

    pub sigactions: Arc<RwLock<[SigAction; MAX_SIGNUM as usize]>>,
    pub memory_set: Arc<RwLock<MemorySet>>,
    pub fd_table: Arc<RwLock<FDTable>>,

    inner: RwLock<TaskControlBlockInner>,
}

impl TaskControlBlock {
    #[inline]
    pub fn is_child_thread(&self) -> bool {
        self.pid.0 != self.tgid
    }
}

impl Debug for TaskControlBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaskControlBlock")
            .field("pid", &self.pid.0)
            .finish()
    }
}

pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    pub trap_cause: Option<Scause>,

    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,

    pub pending_signals: SigSet,
    pub sigmask: SigMask,

    pub cwd: AbsolutePath,
    pub exit_code: i32,

    pub interval_timer: Option<IntervalTimer>,
    pub utime: TimeVal,
    pub stime: TimeVal,
    pub last_enter_umode_time: TimeVal,
    pub last_enter_smode_time: TimeVal,

    pub robust_list: RobustList,
    pub rlimit_nofile: RLimit,

    pub clear_child_tid: usize, /* CLONE_CHILD_CLEARTID */
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
    pub fn token(&self) -> usize {
        self.memory_set.read().token()
    }

    /// Find an empty slot in the file descriptor table
    ///
    /// From low to high, find an empty slot in the file descriptor table,
    /// return the vector subscript, and insert an empty slot at the end if there is no empty slot
    pub fn alloc_fd(fd_table: &mut FDTable, fd_limit: usize) -> usize {
        if let Some(fd) = (0..fd_table.len()).find(|fd| fd_table[*fd].is_none()) {
            fd
        } else {
            if fd_table.len() >= fd_limit {
                return fd_limit;
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
    pub fn new(elf: Arc<dyn File>) -> Self {
        // Translate ELF format data to construct the application address
        // space memory_set and obtain other information
        #[allow(unused_variables)]
        let LoadedELF {
            memory_set,
            elf_entry: entry_point,
            user_stack_top: user_sp,
            auxs,
        } = MemorySet::load_elf(elf.clone());

        #[cfg(feature = "static_busybox")]
        if elf.name() == "static-busybox" {
            save_busybox_related(entry_point, auxs.clone());
        }

        // Allocate PID and kernel stack for the process,
        // and record the position of the kernel stack in the kernel address space
        let pid_handle = pid_alloc();
        let tgid = pid_handle.0;
        // Find out which physical page frame the Trap context in the application address space is actually placed in
        // note: main tread no need to use fn trap_context_position
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.top();

        // Push the initialized task context on the kernel stack of the process,
        // so that when the task is switched to it for the first time,
        // it can jump to trap_return and enter the user mode to start execution
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

            inner: RwLock::new(TaskControlBlockInner {
                trap_cx_ppn,
                task_cx: TaskContext::readied_for_switching(kernel_stack_top),
                task_status: TaskStatus::Ready,
                parent: None,
                children: Vec::new(),
                robust_list: RobustList::default(),
                rlimit_nofile: RLimit::new(FD_LIMIT, FD_LIMIT),
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
                interval_timer: None,
            }),
            sigactions: Arc::new(RwLock::new([SigAction::new(); MAX_SIGNUM as usize])),
        };
        // Init the Trap context in the application address space so that when it first enters the user mode,
        // it can be correct to enter the kernel mode when Trap occurs
        // Jump to the application entry point and set the user stack,
        // and also ensure that the user state can enter the kernel state correctly when Trap occurs
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
        let mut user_sp = user_sp;

        #[cfg(feature = "u740")]
        {
            // Calculate the total length of args and envs
            let mut total_len = 0;
            for i in 0..envs.len() {
                total_len += envs[i].len() + 1; // add 1 for '\0'
            }
            for i in 0..args.len() {
                total_len += args[i].len() + 1;
            }
            let align = core::mem::size_of::<usize>() / core::mem::size_of::<u8>(); // 8
            let mut user_sp = user_sp - (align - total_len % align) * core::mem::size_of::<u8>();
            user_sp -= core::mem::size_of::<usize>();
            *translated_mut(token, user_sp as *mut usize) = 123;
            user_sp -= core::mem::size_of::<usize>();
            *translated_mut(token, user_sp as *mut usize) = 456;
        }

        // alloc envs space, and add the position of dynamic link libraryc
        let envs_ptrv: Vec<_> = (0..envs.len())
            .map(|idx| {
                user_sp -= envs[idx].len() + 1; // 1 是手动添加结束标记的空间('\0')
                let mut ptr = user_sp;
                for c in envs[idx].as_bytes() {
                    // 将参数写入到用户栈
                    copyout(token, unsafe { (ptr as *mut u8).as_mut().unwrap() }, c);
                    ptr += 1;
                } // 写入字符串结束标记
                copyout(token, unsafe { (ptr as *mut u8).as_mut().unwrap() }, &0);
                user_sp
            })
            .collect();

        // alloc args space, and write string data, save the string address in argv
        // Here the high address puts the previous parameter, that is, store argv[0] first
        let args_ptrv: Vec<_> = (0..args.len())
            .map(|idx| {
                user_sp -= args[idx].len() + 1; // add 1 for '\0'
                let mut ptr = user_sp;
                for c in args[idx].as_bytes() {
                    // copyout the parameter to the user stack
                    copyout(token, unsafe { (ptr as *mut u8).as_mut().unwrap() }, c);
                    ptr += 1;
                }
                // write the string end mark
                copyout(token, unsafe { (ptr as *mut u8).as_mut().unwrap() }, &0);
                user_sp
            })
            .collect();

        // padding 0 to indicate the end of AT_NULL aux entry
        user_sp -= core::mem::size_of::<usize>();
        copyout(
            token,
            unsafe { (user_sp as *mut usize).as_mut().unwrap() },
            &0,
        );

        // alloc auxs space, and write data
        for i in 0..auxv.len() {
            user_sp -= core::mem::size_of::<AuxEntry>();
            copyout(
                token,
                unsafe { (user_sp as *mut AuxEntry).as_mut().unwrap() },
                &auxv[i],
            );
        }
        // auxv.push(AuxEntry(AT_EXECFN,args_ptrv[0] ));

        // padding 0 to indicate the end of args
        user_sp -= core::mem::size_of::<usize>();
        copyout(
            token,
            unsafe { (user_sp as *mut usize).as_mut().unwrap() },
            &0,
        );

        // envs_ptr
        user_sp -= (envs.len()) * core::mem::size_of::<usize>();
        let envs_ptr_base = user_sp; // start address of parameter string pointer
        for i in 0..envs.len() {
            copyout(
                token,
                unsafe {
                    ((envs_ptr_base + i * core::mem::size_of::<usize>()) as *mut usize)
                        .as_mut()
                        .unwrap()
                },
                &envs_ptrv[i],
            );
        }

        // padding 0 to indicate the end of envs
        user_sp -= core::mem::size_of::<usize>();
        copyout(
            token,
            unsafe { (user_sp as *mut usize).as_mut().unwrap() },
            &0,
        );

        // args_ptr
        user_sp -= (args.len()) * core::mem::size_of::<usize>();
        let args_ptr_base = user_sp; // start address of parameter string pointer
        for i in 0..args.len() {
            copyout(
                token,
                unsafe {
                    ((args_ptr_base + i * core::mem::size_of::<usize>()) as *mut usize)
                        .as_mut()
                        .unwrap()
                },
                &args_ptrv[i],
            );
        }

        // argc
        user_sp -= core::mem::size_of::<usize>();
        let len = args.len();
        copyout(
            token,
            unsafe { (user_sp as *mut usize).as_mut().unwrap() },
            &len,
        );

        (user_sp, args_ptr_base as usize, envs_ptr_base as usize)
    }

    /// 用来实现 exec 系统调用, 即当前进程加载并执行另一个 ELF 格式可执行文件
    pub fn exec(&self, elf_file: Arc<dyn File>, args: Vec<String>, envs: Vec<String>) {
        // 从 ELF 文件生成一个全新的地址空间并直接替换
        let LoadedELF {
            memory_set,
            user_stack_top: user_sp,
            elf_entry: entry_point,
            mut auxs,
        } = MemorySet::load_elf(elf_file);
        assert!(self.pid.0 == self.tgid, "exec task must be thread");
        // let trap_addr = trap_context_position(self.pid() - self.tgid);
        // let trap_cx_ppn = memory_set
        //     .translate(VirtAddr::from(trap_addr).into())
        //     .unwrap()
        //     .ppn();
        // main tread: no need to  use fn: trap_context_position
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
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
        // 修改新的地址空间中的 Trap 上下文, 将解析得到的应用入口点, 用户栈位置以及一些内核的信息进行初始化
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            acquire_kvmm().token(),
            self.kernel_stack.top(),
            user_trap_handler as usize,
        );
    }

    /// 用来实现 fork 系统调用, 即当前进程 fork 出来一个与之几乎相同的子进程
    pub fn fork(self: &Arc<TaskControlBlock>, flags: CloneFlags) -> Arc<TaskControlBlock> {
        // 分配一个 PID
        let pid_handle = pid_alloc();
        let tgid = if flags.contains(CloneFlags::THREAD) {
            self.pid.0
        } else {
            pid_handle.0
        };
        let private_tid = pid_handle.0 - tgid;
        // 根据 PID 创建一个应用内核栈
        let kernel_stack = KernelStack::new(&pid_handle);

        let kernel_stack_top = kernel_stack.top();

        // 拷贝用户地址空间
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set.clone()
        } else {
            Arc::new(RwLock::new(MemorySet::from_copy_on_write(
                &mut self.memory_set.write(),
            ))) // use 4 pages
        };

        if flags.contains(CloneFlags::THREAD) {
            memory_set.write().map_thread_trap_context(private_tid);
        }

        let trap_cx_ppn = memory_set
            .read()
            .translate(trap_context_position(private_tid).into())
            .unwrap()
            .ppn();

        if flags.contains(CloneFlags::THREAD) {
            let trap_cx: &mut TrapContext = trap_cx_ppn.as_mut() as &mut TrapContext;
            *trap_cx = self.inner_ref().trap_context().clone();
        }
        // copy fd table
        let fd_table = if flags.contains(CloneFlags::FILES) {
            self.fd_table.clone()
        } else {
            let mut new_fd_table = Vec::new();
            // parent fd table
            let pfd_table_ref = self.fd_table.read();
            for fd in pfd_table_ref.iter() {
                if let Some(file) = fd {
                    new_fd_table.push(Some(file.clone()));
                } else {
                    new_fd_table.push(None);
                }
            }
            Arc::new(RwLock::new(new_fd_table))
        };

        let sigactions = if flags.contains(CloneFlags::SIGHAND) {
            self.sigactions.clone()
        } else {
            // parent sigactions
            let psa_ref = self.sigactions.read();
            let sa = Arc::new(RwLock::new([SigAction::new(); MAX_SIGNUM as usize]));
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
            memory_set,
            fd_table,
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

                rlimit_nofile: RLimit::new(FD_LIMIT, FD_LIMIT),
                robust_list: RobustList::default(),

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
                interval_timer: None,
            }),
        });

        // 把新生成的进程加入到子进程向量中
        parent_inner.children.push(task_control_block.clone());
        // 更新子进程 trap 上下文中的栈顶指针
        let trap_cx = task_control_block.inner_mut().trap_context();
        trap_cx.kernel_sp = kernel_stack_top;

        task_control_block
    }

    /// 尝试用时加载缺页, 目前只支持mmap缺页
    ///
    /// - 参数:
    ///     - `va`: 缺页中的虚拟地址
    ///     - `is_load`: 加载(1)/写入(0)
    /// - 返回值:
    ///     - `0`: 成功加载缺页
    ///     - `-1`: 加载缺页失败
    ///
    /// 分别用于:
    ///     - 用户态: handler page fault
    ///     - 内核态:  translate_bytes_buffer
    pub fn check_lazy(&self, va: VirtAddr) -> isize {
        let mut memory_set = self.memory_set.write();

        let mmap_start = memory_set.mmap_manager.mmap_start;
        let mmap_end = memory_set.mmap_manager.mmap_top;
        let heap_start = VirtAddr::from(memory_set.brk_start);
        let heap_end = VirtAddr::from(memory_set.brk_start + USER_HEAP_SIZE);
        let stack_start = VirtAddr::from(memory_set.user_stack_start);
        let stack_end = VirtAddr::from(memory_set.user_stack_end);
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

        // lazy map / lazy alloc heap / lazy alloc stack
        if va >= stack_start && va < stack_end {
            memory_set.lazy_alloc_stack(va.floor())
        } else if va >= heap_start && va <= heap_end {
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
        // 可能会有 mmap 后没有访问直接 munmap 的情况, 需要检查是否访问过 mmap 的区域(即
        // 是否引发了 lazy_mmap), 防止 unmap 页表中不存在的页表项引发 panic
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

#[cfg(feature = "static_busybox")]
pub fn save_busybox_related(elf_entry: usize, auxs: Vec<AuxEntry>) {
    unsafe {
        STATIC_BUSYBOX_ENTRY = elf_entry;
        STATIC_BUSYBOX_AUX = auxs;
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Ready,
    Running,
    Blocking,
    Hanging,
    Zombie,
}
pub fn trap_context_position(tid: usize) -> VirtAddr {
    VirtAddr::from(TRAP_CONTEXT_BASE - tid * PAGE_SIZE)
}
