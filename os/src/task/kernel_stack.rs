//!
//! 进程内核栈
//!

use crate::{
    config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE},
    mm::{kernel_vmm::KERNEL_VMM, MapPermission, VirtAddr},
};

use super::PidHandle;

pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    /// 从一个已分配的进程标识符中对应生成一个内核栈
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);

        KERNEL_VMM.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        KernelStack { pid }
    }

    pub fn top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }

    /// 将一个类型为 T 的变量压入内核栈顶并返回其裸指针
    #[inline]
    pub fn push<T>(&self, value: T) -> *mut T {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);

        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_VMM
            .lock()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;

    (bottom, top)
}
