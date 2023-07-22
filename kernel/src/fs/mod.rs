//! 内核 fs

mod dirent;
mod fat32;
pub mod fdset;
pub mod file;
mod mount;
pub mod open_flags;
mod path;
mod pipe;
mod stat;
mod stdio;

use alloc::string::ToString;
pub use path::*;

use crate::{
    fs::{fat32::list_apps, open_flags::CreateMode},
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
    // 预创建文件/文件夹
    open(
        "/proc".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/tmp".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/dev".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/var".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/dev/misc".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/var/tmp".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open("/dev/null".into(), OpenFlags::O_CREATE, CreateMode::empty());
    open("/dev/zero".into(), OpenFlags::O_CREATE, CreateMode::empty());
    open(
        "/proc/mounts".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/proc/meminfo".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/dev/misc/rtc".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/var/tmp/lmbench".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    );

    println!("===+ Files Loaded +===");
    list_apps(AbsolutePath::from_string("/".to_string()));
    println!("===+==============+===");
}
