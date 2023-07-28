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

use crate::fs::{fat32::list_apps, open_flags::CreateMode};
use nix::time::TimeSpec;

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
        "/dev/shm".into(),
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

    // sys_clock_getres
    // 应用程序可以通过打开 /dev/cpu_dma_latency 设备文件，并向其写入一个非负整数，来请求将 CPU 切换到低延迟模式。
    // 写入的整数值表示请求的最大延迟时间，单位为微秒
    open(
        "/dev/cpu_dma_latency".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    );

    open(
        "/etc/passwd".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    );

    open("/dev/tty".into(), OpenFlags::O_CREATE, CreateMode::empty());

    println!("===+ Files Loaded +===");
    list_apps(AbsolutePath::from_string("/".to_string()));
    println!("===+==============+===");
}
