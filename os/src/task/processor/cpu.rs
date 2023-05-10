use core::cell::{RefCell, RefMut};

use crate::task::processor::Processor;

pub struct Cpu {
    processor: RefCell<Processor>,
}

unsafe impl Sync for Cpu {}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            processor: RefCell::new(Processor::new()),
        }
    }

    pub fn get_mut(&self) -> RefMut<'_, Processor> {
        self.processor.borrow_mut()
    }
}
