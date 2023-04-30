use crate::fs::{open, open_flags::CreateMode, OpenFlags};

pub fn pre_init(create: bool) {
    if !create {
        return;
    }

    // 预创建文件/文件夹
    open(
        "/",
        "proc",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/",
        "tmp",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/",
        "dev",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/",
        "var",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/dev",
        "misc",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open(
        "/var",
        "tmp",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
    open("/dev", "null", OpenFlags::O_CREATE, CreateMode::empty());
    open("/dev", "zero", OpenFlags::O_CREATE, CreateMode::empty());
    open("/proc", "mounts", OpenFlags::O_CREATE, CreateMode::empty());
    open("/proc", "meminfo", OpenFlags::O_CREATE, CreateMode::empty());
    open("/dev/misc", "rtc", OpenFlags::O_CREATE, CreateMode::empty());
    open(
        "/var/tmp",
        "lmbench",
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    );
}
