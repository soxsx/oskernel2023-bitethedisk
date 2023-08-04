//! Kernel file system
//!
//! The kernel uniformly borrows the VirtFile provided by the fat32 file system as the object for the kernel to operate files.

mod fat32;
mod file;
mod mount;
mod page_cache;
mod pipe;
mod stdio;
pub use self::fat32::*;
pub use file::*;
pub use mount::*;
pub use page_cache::*;
pub use path::*;
pub use pipe::*;
use spin::Mutex;
pub use stdio::*;

mod page;
pub use page::*;

use alloc::string::ToString;
use nix::{CreateMode, OpenFlags};
use path::AbsolutePath;
pub use path::*;

pub use crate::fs::fat32::{chdir, open};
pub use mount::MNT_TABLE;
pub use pipe::{make_pipe, Pipe};
pub use stdio::{Stdin, Stdout};

pub fn init() {
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

static INO_ALLOCATOR: Mutex<Allocator> = Mutex::new(Allocator::new());
struct Allocator {
    current: u64,
}
impl Allocator {
    pub const fn new() -> Self {
        Allocator { current: 0 }
    }
    fn fetch_add(&mut self) -> u64 {
        let id = self.current;
        self.current += 1;
        id
    }
    pub fn alloc(&mut self) -> u64 {
        self.fetch_add()
    }
}
pub fn ino_alloc() -> u64 {
    INO_ALLOCATOR.lock().alloc()
}
