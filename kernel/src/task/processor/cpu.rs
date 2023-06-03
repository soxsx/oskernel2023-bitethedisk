use core::cell::{RefCell, RefMut};

use alloc::vec::Vec;

use crate::task::{processor::Processor, PidHandle};

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
