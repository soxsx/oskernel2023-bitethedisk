/// # 进程标识符和应用内核栈模块
/// `os/src/task/pid.rs`
/// ## 实现功能
/// ```
/// struct PidAllocator
/// static ref PID_ALLOCATOR: Mutex<PidAllocator>
/// pub struct PidHandle(pub usize)
/// pub struct KernelStack
///
/// pub fn pid_alloc() -> PidHandle
/// pub fn kernel_stack_position(app_id: usize) -> (usize, usize)
/// ```
//
use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::mm::{MapPermission, VirtAddr, KERNEL_SPACE};
use alloc::vec::Vec;
use lazy_static::*;
use spin::Mutex;

/// ### 栈式进程标识符分配器
/// |成员变量|描述|
/// |--|--|
/// |`current`|当前可用的最小PID|
/// |`recycled`|以栈的形式存放着已经回收的PID|
/// ```
/// PidAllocator::new() -> Self
/// PidAllocator::alloc(&mut self) -> PidHandle
/// PidAllocator::dealloc(&mut self, pid: usize)
/// ```
struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub const fn new() -> Self {
        PidAllocator {
            current: 0,
            recycled: vec![],
        }
    }

    /// 分配一个进程标识符
    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    /// 释放一个进程标识符
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        // HINT: 可以用 HashSet
        assert!(
            !self.recycled.iter().any(|ppid| *ppid == pid),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

/// 进程标识符
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PidHandle(pub usize);

impl From<usize> for PidHandle {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<PidHandle> for usize {
    fn from(value: PidHandle) -> Self {
        value.0
    }
}

// 为 PidHandle 实现 Drop Trait 来允许编译器进行自动的资源回收
impl Drop for PidHandle {
    fn drop(&mut self) {
        // 这里可能导致在从 pid map 里面清除对应 pcb 的时候多次释放，被 assert panic 了
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}

/// 从全局栈式进程标识符分配器 `PID_ALLOCATOR` 分配一个进程标识符
#[inline]
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}

/// Return (bottom, top) of a kernel stack in kernel space.
#[inline]
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;

    (bottom, top)
}

/// ### 应用内核栈
/// - 成员变量：pid
/// ```
/// KernelStack::new(pid_handle: &PidHandle) -> Self
/// KernelStack::push_on_top<T>(&self, value: T) -> *mut T
/// KernelStack::get_top(&self) -> usize
/// ```
pub struct KernelStack {
    pid: PidHandle,

    /// 应用内核栈顶在内核地址空间中的地址
    top: usize,
}

impl KernelStack {
    /// 从一个已分配的进程标识符中对应生成一个内核栈
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.clone();
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid.0);

        KERNEL_SPACE.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        KernelStack {
            pid,
            top: kernel_stack_top,
        }
    }

    /// 将一个类型为 T 的变量压入内核栈顶并返回其裸指针
    #[inline]
    pub fn push<T>(&self, value: T) -> *mut T {
        let ptr_mut = (self.top - core::mem::size_of::<T>()) as *mut T;
        unsafe { *ptr_mut = value }

        ptr_mut
    }

    /// 获取当前应用内核栈顶在内核地址空间中的地址(这地址仅与app_id有关)
    #[inline(always)]
    pub fn top(&self) -> usize {
        self.top
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid.0);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}
