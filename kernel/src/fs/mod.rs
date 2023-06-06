//! 内核 fs

mod dirent;
mod fat32;
pub mod file;
mod mount;
pub mod open_flags;
mod path;
mod pipe;
mod stat;
mod stdio;

use crate::{
    fs::fat32::{list_apps, ROOT_INODE},
    timer::Timespec,
};

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
