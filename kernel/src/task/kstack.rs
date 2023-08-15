//! Kernel stack for a process.

use crate::{
    consts::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE},
    mm::{acquire_kvmm, MapPermission, VirtAddr, VmAreaType},
};

use super::PidHandle;

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;

    (bottom, top)
}
pub struct KernelStack {
    pid: usize,
}
impl KernelStack {
    /// Generate a kernel stack for a process with a given pid.
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        acquire_kvmm().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
            VmAreaType::KernelStack,
        );

        KernelStack { pid: pid_handle.0 }
    }
    /// Get the top address of the kernel stack in the kernel address space.(This address is only related to app_id)
    pub fn top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}
impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        acquire_kvmm().remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}
