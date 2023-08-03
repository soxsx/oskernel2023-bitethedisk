mod address;
mod frame_allocator;
mod kernel_vmm;
mod memory_set;
mod page_table;
mod shared_memory;
mod user_buffer;
mod vma;
pub use address::*;
pub use frame_allocator::*;
pub use kernel_vmm::*;
pub use memory_set::*;
pub use page_table::*;
pub use shared_memory::*;
pub use user_buffer::*;
pub use vma::*;

use crate::{consts::PAGE_SIZE, task::current_task};
use alloc::{string::String, vec::Vec};
use core::{cmp::min, mem::size_of};
use riscv::register::satp;

/// 内存管理子系统的初始化
pub fn init() {
    init_frame_allocator();
    enable_mmu();
}

pub fn init_frame_allocator() {
    frame_allocator::init();
}

pub fn enable_mmu() {
    satp::write(acquire_kvmm().token());
    unsafe { core::arch::asm!("sfence.vma") } // 刷新 MMU 的 TLB
}

/// 以向量的形式返回一组可以在内存空间中直接访问的字节数组切片
///
/// - `token`: 某个应用地址空间的 token
/// - `ptr`: 应用地址空间中的一段缓冲区的起始地址
/// - `len`: 应用地址空间中的一段缓冲区的长度
pub fn translated_bytes_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = VirtAddr::from(ptr as usize);
    let end = VirtAddr::from(start.0 + len);
    let mut v = Vec::new();
    while start < end {
        let mut vpn = start.floor();
        let ppn = match page_table.translate(vpn) {
            Some(pte) => pte.ppn(),
            None => {
                if current_task().check_lazy(start, true) != 0 {
                    panic!("check lazy error");
                }
                page_table.translate(vpn).unwrap().ppn()
            }
        };
        vpn.step();
        // 避免跨页
        let in_page_end_va: VirtAddr = min(vpn.into(), end);
        if in_page_end_va.page_offset() == 0 {
            v.push(&mut ppn.as_bytes_array()[start.page_offset()..]);
        } else {
            v.push(&mut ppn.as_bytes_array()[start.page_offset()..in_page_end_va.page_offset()]);
        }
        start = in_page_end_va.into();
    }
    v
}

/// 从内核地址空间之外的某个应用的用户态地址空间中拿到一个字符串
///
/// 针对应用的字符串中字符的用户态虚拟地址，查页表，找到对应的内核虚拟地址，逐字节地构造字符串，直到发现一个 \0 为止
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .as_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

/// 根据 多级页表token (satp) 和 虚拟地址 获取大小为 T 的空间的不可变切片
pub fn translated_ref<T>(token: usize, ptr: *const T) -> &'static T {
    let offset = ptr as usize % PAGE_SIZE;
    assert!(
        PAGE_SIZE - offset >= size_of::<T>(),
        "cross-page access from translated_ref"
    );
    let page_table = PageTable::from_token(token);
    page_table
        .translate_va(VirtAddr::from(ptr as usize))
        .unwrap()
        .as_ref()
}

/// 根据 多级页表token (satp) 和 虚拟地址 获取大小为 T 的空间的切片
pub fn translated_mut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let offset = ptr as usize % PAGE_SIZE;
    assert!(
        PAGE_SIZE - offset >= size_of::<T>(),
        "cross-page access from translated_mut"
    );
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .as_mut()
}

pub fn copyin<T>(token: usize, dst: &mut T, src: *const T) {
    let src_buffer = translated_bytes_buffer(token, src as *const u8, core::mem::size_of::<T>());

    let dst_slice = unsafe {
        core::slice::from_raw_parts_mut(dst as *mut T as *mut u8, core::mem::size_of::<T>())
    };

    let mut start_byte = 0;
    let mut index = 0;
    loop {
        let src_slice = &src_buffer[index];
        index += 1;
        let src_slice_len = src_slice.len();
        dst_slice[start_byte..start_byte + src_slice_len].copy_from_slice(src_slice);
        start_byte += src_slice_len;
        if src_buffer.len() == index {
            break;
        }
    }
}

pub fn copyout<T>(token: usize, dst: *mut T, src: &T) {
    let mut dst_buffer =
        translated_bytes_buffer(token, dst as *const u8, core::mem::size_of::<T>());

    let src_slice = unsafe {
        core::slice::from_raw_parts(src as *const T as *const u8, core::mem::size_of::<T>())
    };
    let mut index = 0;

    let mut start_byte = 0;
    loop {
        let dst_slice = &mut dst_buffer[index];
        index += 1;
        let dst_slice_len = dst_slice.len();
        dst_slice.copy_from_slice(&src_slice[start_byte..start_byte + dst_slice_len]);
        start_byte += dst_slice_len;
        if dst_buffer.len() == index {
            break;
        }
    }
}
