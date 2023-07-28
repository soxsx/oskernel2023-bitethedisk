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
    pub fn from_vm_prot(prot: VmProt) -> Self {
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
            VmProt::PROT_READ,  MapPermission::R
            VmProt::PROT_WRITE, MapPermission::W
            VmProt::PROT_EXEC,  MapPermission::X
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

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct VmFlags: isize {
        /// 用于兼容的标志，可忽略
        const MAP_FILE = 0;

        /// 进程间共享，对当前虚拟地址映射空间的更改对其他进程可见
        const MAP_SHARED = 0x01;

        /// 进程私有，copy-on-write，需要将父子进程的 prot 设置为只读，
        /// 由此引起写操作时的缺页异常，再进行处理
        const MAP_PRIVATE = 0x02;

        /// mmap 失败时返回的值，严格来说并不是 [`VmFlags`] 的一部分
        const MAP_FAILED = (usize::MAX - 1) as isize;
    }

    #[derive(Clone, Copy, Debug)]
    pub struct VmProt: isize {
        /// 不可访问
        const PROT_NONE  = 0;
        /// 可读
        const PROT_READ  = 1 << 0;
        /// 可写
        const PROT_WRITE = 1 << 1;
        /// 可执行
        const PROT_EXEC  = 1 << 2;

        const PROT_GROWSDOWN = 0x01000000;
        const PROT_GROWSUP = 0x02000000;
    }
}
