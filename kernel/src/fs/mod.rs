//! 内核 fs

mod dirent;
mod fat32;
mod fdset;
mod file;
mod mount;
mod open_flags;
mod page_cache;
mod pipe;
mod stat;
mod stdio;
pub use self::fat32::*;
pub use dirent::*;
pub use fdset::*;
pub use file::*;
pub use mount::*;
pub use open_flags::*;
pub use page_cache::*;
pub use path::*;
pub use pipe::*;
pub use stat::*;
pub use stdio::*;

mod page;
pub use page::*;

use alloc::string::ToString;
use path::AbsolutePath;

pub fn init() {
    // 预创建文件/文件夹
    open(
        "/proc".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/tmp".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/dev".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/var".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/dev/misc".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/dev/shm".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/var/tmp".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open("/dev/null".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();
    open("/dev/zero".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();
    open(
        "/proc/mounts".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/proc/meminfo".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/dev/misc/rtc".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/var/tmp/lmbench".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();

    // sys_clock_getres
    // 应用程序可以通过打开 /dev/cpu_dma_latency 设备文件, 并向其写入一个非负整数, 来请求将 CPU 切换到低延迟模式.
    // 写入的整数值表示请求的最大延迟时间, 单位为微秒
    open(
        "/dev/cpu_dma_latency".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();

    open("/dev/tty".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();
    open("/lat_sig".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();

    println!("===+ Files Loaded +===");
    list_apps(AbsolutePath::from_string("/".to_string()));
    println!("===+==============+===");
}
