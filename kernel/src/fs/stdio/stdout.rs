use alloc::vec::Vec;

use crate::{fs::file::File, mm::UserBuffer};

pub struct Stdout;

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn available(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }

    fn name(&self) -> &str {
        "Stdout"
    }

    fn write_kernel_space(&self, data: Vec<u8>) -> usize {
        let buffer = data.as_slice();
        print!("{}", core::str::from_utf8(buffer).unwrap());

        data.len()
    }
    fn set_cloexec(&self) {}
}
