// 虚拟页面映射到物理页帧的方式
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical, // 恒等映射，一般用在内核空间（空间已分配）
    Framed, // 对于每个虚拟页面都有一个新分配的物理页帧与之对应，虚地址与物理地址的映射关系是相对随机的
}

bitflags! {
    /// 页表项标志位 `PTE Flags` 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
    #[derive(Clone, Copy)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

impl MapPermission {
    pub fn is_read(self) -> bool {
        self.bits() & 1 << 1 == 1 << 1
    }
    pub fn is_write(self) -> bool {
        self.bits() & 1 << 2 == 1 << 2
    }
    pub fn is_execute(self) -> bool {
        self.bits() & 1 << 3 == 1 << 3
    }
    pub fn is_user(self) -> bool {
        self.bits() & 1 << 4 == 1 << 4
    }
}