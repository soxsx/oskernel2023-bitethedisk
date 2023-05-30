//! 内核 fs

mod dirent;
mod fat32;
// mod file;
mod mount;
pub mod open_flags;
mod path;
mod pipe;
mod stat;
mod stdio;

use crate::{
    fs::fat32::{list_apps, ROOT_INODE},
    mm::UserBuffer,
    timer::Timespec,
};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};

pub use crate::fs::fat32::{chdir, open, Fat32File};
pub use dirent::Dirent;
pub use mount::MNT_TABLE;
pub use open_flags::OpenFlags;
pub use pipe::{make_pipe, Pipe};
pub use stat::*;
pub use stdio::{Stdin, Stdout};

pub fn init() {
    println!("===+ Files Loaded +===");
    list_apps(ROOT_INODE.clone());
    println!("===+==============+===");
}

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn available(&self) -> bool;
    /// read 指的是从文件中读取数据放到缓冲区中，最多将缓冲区填满，并返回实际读取的字节数
    fn read(&self, buf: UserBuffer) -> usize;
    /// 将缓冲区中的数据写入文件，最多将缓冲区中的数据全部写入，并返回直接写入的字节数
    fn write(&self, buf: UserBuffer) -> usize;

    // (lzm)
    fn read_to_vec(&self, offset: isize, len: usize) -> Vec<u8> {
        panic!("{} not implement read_to_vec", self.name());
    }

    // TODO seek mode
    fn seek(&self, _pos: usize) {
        panic!("{} not implement seek", self.name());
    }

    fn name(&self) -> &str;

    fn fstat(&self, _kstat: &mut Kstat) {
        panic!("{} not implement get_fstat", self.name());
    }

    fn set_time(&self, _timespec: &Timespec) {
        panic!("{} not implement set_time", self.name());
    }

    fn dirent(&self, _dirent: &mut Dirent) -> isize {
        panic!("{} not implement get_dirent", self.name());
    }

    fn get_path(&self) -> &str {
        panic!("{} not implement get_path", self.name());
    }

    fn offset(&self) -> usize {
        panic!("{} not implement get_offset", self.name());
    }

    fn set_offset(&self, _offset: usize) {
        panic!("{} not implement set_offset", self.name());
    }

    fn set_flags(&self, _flag: OpenFlags) {
        panic!("{} not implement set_flags", self.name());
    }

    fn set_cloexec(&self) {
        panic!("{} not implement set_cloexec", self.name());
    }

    fn read_kernel_space(&self) -> Vec<u8> {
        panic!("{} not implement read_kernel_space", self.name());
    }

    fn write_kernel_space(&self, _data: Vec<u8>) -> usize {
        panic!("{} not implement write_kernel_space", self.name());
    }

    fn file_size(&self) -> usize {
        panic!("{} not implement file_size", self.name());
    }

    fn r_ready(&self) -> bool {
        true
    }
    fn w_ready(&self) -> bool {
        true
    }
}

impl Debug for dyn File + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "name:{}", self.name())
    }
}
