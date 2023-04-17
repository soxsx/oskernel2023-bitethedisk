use alloc::vec::Vec;

use crate::{fs::File, mm::UserBuffer};

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
        // println!("buffer:{:?}",user_buf);
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }

    fn name(&self) -> &str {
        "Stdout"
    }

    fn set_cloexec(&self) {
        // 涉及刚开始的 open /dev/tty，然后 sys_fcntl:fd:2,cmd:1030,arg:Some(10)
        // 可能是 sh: ls: unknown operan 等问题的原因
        // panic!("Stdput not implement set_cloexec");
    }

    fn write_kernel_space(&self, data: Vec<u8>) -> usize {
        // println!("data:{:?}",data);
        let buffer = data.as_slice();
        // println!("str:{:?}",core::str::from_utf8(buffer).unwrap());
        print!("{}", core::str::from_utf8(buffer).unwrap());
        data.len()
    }
}
