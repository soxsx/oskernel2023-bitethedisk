use core::cell::{RefCell, RefMut};

use alloc::vec::Vec;

use crate::task::{processor::Processor, PidHandle};

/// 对于每一个 CPU 的抽象，在 Rust 语言层面，[`Cpu`] 提供了并发访问的 trait 实现
/// 与内部可变性的功能
pub struct Cpu {
    processor: RefCell<Processor>,
    pub pid_pool: Vec<PidHandle>,
}

unsafe impl Sync for Cpu {}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            processor: RefCell::new(Processor::new()),
            pid_pool: Vec::new(),
        }
    }

    pub fn get_mut(&self) -> RefMut<'_, Processor> {
        self.processor.borrow_mut()
    }
}
