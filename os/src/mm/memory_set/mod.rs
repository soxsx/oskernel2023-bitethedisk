use alloc::vec::Vec;

use crate::fs::{open, OpenFlags};

pub mod chunk_area;
pub mod map_area;
pub mod map_flags;
pub mod memory_set;

pub use chunk_area::*;
pub use map_area::*;
pub use map_flags::*;
pub use memory_set::*;

lazy_static! {
    // 不知道往哪里放
    pub static ref BUSYBOX: Vec<u8> = {
        if let Some(app_inode) = open("/", "busybox", OpenFlags::O_RDONLY) {
            app_inode.read_vec(0, 96 * 4096)
        } else {
            panic!("can't find busybox");
        }
    };
}
