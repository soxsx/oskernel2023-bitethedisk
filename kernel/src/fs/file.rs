use super::{Dirent, Kstat, OpenFlags, TimeInfo};
use crate::mm::UserBuffer;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::fmt::{self, Formatter};
use path::AbsolutePath;

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn available(&self) -> bool;
    /// 从文件中读取数据放到缓冲区中, 最多将缓冲区填满, 并返回实际读取的字节数
    fn read_to_ubuf(&self, buf: UserBuffer) -> usize;
    /// 将缓冲区中的数据写入文件, 最多将缓冲区中的数据全部写入, 并返回直接写入的字节数
    fn write_from_ubuf(&self, buf: UserBuffer) -> usize;
    fn pread(&self, _buf: UserBuffer, _offset: usize) -> usize {
        panic!("{} not implement pread", self.name());
    }
    fn pwrite(&self, _buf: UserBuffer, _offset: usize) -> usize {
        panic!("{} not implement pwrite", self.name());
    }
    fn read_at_direct(&self, _offset: usize, _len: usize) -> Vec<u8> {
        panic!("{} not implement read_at_direct", self.name());
    }
    fn write_from_direct(&self, _offset: usize, _data: &Vec<u8>) -> usize {
        panic!("{} not implement write_from_direct", self.name());
    }
    fn read_to_kspace_with_offset(&self, _offset: usize, _len: usize) -> Vec<u8> {
        panic!("{} not implement read_to_kspace_with_offset", self.name());
    }
    fn seek(&self, _pos: usize) {
        panic!("{} not implement seek", self.name());
    }
    fn name(&self) -> &str;
    fn fstat(&self, _kstat: &mut Kstat) {
        panic!("{} not implement fstat", self.name());
    }
    fn set_time(&self, _xtime_info: TimeInfo) {
        panic!("{} not implement set_time", self.name());
    }
    fn time(&self) -> TimeInfo {
        panic!("{} not implement get_time", self.name());
    }
    fn dirent(&self, _dirent: &mut Dirent) -> isize {
        panic!("{} not implement get_dirent", self.name());
    }
    fn offset(&self) -> usize {
        panic!("{} not implement get_offset", self.name());
    }
    fn set_flags(&self, _flag: OpenFlags) {
        panic!("{} not implement set_flags", self.name());
    }
    fn flags(&self) -> OpenFlags {
        panic!("{} not implement get_flags", self.name());
    }
    fn set_cloexec(&self) {
        panic!("{} not implement set_cloexec", self.name());
    }
    fn read_to_kspace(&self) -> Vec<u8> {
        panic!("{} not implement read_kernel_space", self.name());
    }
    fn write_from_kspace(&self, _data: &Vec<u8>) -> usize {
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
    fn path(&self) -> AbsolutePath {
        unimplemented!("not implemente yet");
    }
    fn truncate(&self, _new_length: usize) {
        unimplemented!("not implemente yet");
    }
}

impl Debug for dyn File + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "name:{}", self.name())
    }
}
