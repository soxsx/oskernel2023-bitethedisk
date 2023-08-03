//! 虚拟地址空间映射的标志性字段

#![allow(unused)]

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct MapPermission: u16 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

impl MapPermission {
    pub fn from_vm_prot(prot: MmapProts) -> Self {
        if prot.bits() == 0 {
            return MapPermission::empty();
        }

        macro_rules! prot2flags {
            ($flags:expr, $($prot_bit:expr, $flag_bit:expr)*) => {
                $(
                    if prot.contains($prot_bit) {
                        $flags |= $flag_bit;
                    }
                )*
            };
        }

        let mut flags = MapPermission::empty();

        prot2flags! {
            flags,
            MmapProts::PROT_READ,  MapPermission::R
            MmapProts::PROT_WRITE, MapPermission::W
            MmapProts::PROT_EXEC,  MapPermission::X
        }

        flags
    }

    pub fn readable(self) -> bool {
        self.contains(MapPermission::R)
    }

    pub fn writable(self) -> bool {
        self.contains(MapPermission::W)
    }

    pub fn executable(self) -> bool {
        self.contains(MapPermission::X)
    }

    pub fn is_user(self) -> bool {
        self.contains(MapPermission::U)
    }
}

// see [man mmap](https://man7.org/linux/man-pages/man2/mmap.2.html)
bitflags! {
#[derive(Clone, Copy, Debug)]
    pub struct MmapProts: usize {
        const PROT_NONE = 0;  // 不可访问 用于实现防范攻击的 guard page 等
        const PROT_READ = 1 << 0;
        const PROT_WRITE = 1 << 1;
        const PROT_EXEC  = 1 << 2;
        const PROT_GROWSDOWN = 0x01000000;
        const PROT_GROWSUP = 0x02000000;
    }
}

bitflags! {
#[derive(Clone, Copy, Debug)]
    pub struct MmapFlags: usize {
        /// 文件映射, 使用文件内容初始化内存 (用于兼容的标志, 可忽略)
        const MAP_FILE = 0;
        /// 进程间共享, 对当前虚拟地址映射空间的更改对其他进程可见
        const MAP_SHARED = 0x01;
        /// 进程私有, copy-on-write, 需要将父子进程的 prot 设置为只读. 由此引起写操作时的缺页异常, 再进行处理
        const MAP_PRIVATE = 0x02;
        /// 将mmap空间放在addr指定的内存地址上, 若与现有映射页面重叠, 则丢弃重叠部分. 如果指定的地址不能使用, mmap将失败.
        const MAP_FIXED = 0x10;
        /// 匿名映射, 初始化全为 0 的内存空间. 当 fd 为 -1 且存在 MAP_ANONYMOUS 标志时, mmap 将创建一个匿名映射
        const MAP_ANONYMOUS = 0x20;
    }
}
