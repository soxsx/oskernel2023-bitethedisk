//! 虚实地址抽象

use super::PageTableEntry;
use crate::consts::PAGE_SIZE;
use core::fmt::Debug;

/// 页内偏移：12bit
pub const IN_PAGE_OFFSET: usize = 0xc;

/// 物理地址宽度：56bit
const PA_WIDTH_SV39: usize = 56;
/// 虚拟地址宽度：39bit
const VA_WIDTH_SV39: usize = 39;
/// 物理页号宽度：44bit
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - IN_PAGE_OFFSET;
/// 虚拟页号宽度：27bit
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - IN_PAGE_OFFSET;

macro_rules! derive_wrap {
    ($($type_def:item)*) => {
        $(
            #[repr(C)]
            #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
            $type_def
        )*
    };
}

derive_wrap! {
    pub struct PhysAddr(pub usize);
    pub struct VirtAddr(pub usize);
    pub struct PhysPageNum(pub usize);
    pub struct VirtPageNum(pub usize);
}

macro_rules! gen_into_usize {
    ($($addr_type:ident)*) => {
        $(
            impl From<$addr_type> for usize {
                fn from(value: $addr_type) -> Self {
                    value.0
                }
            }
        )*
    };
}

gen_into_usize! {
    PhysAddr
    VirtAddr
    PhysPageNum
    VirtPageNum
}

macro_rules! gen_from_usize {
    ($($addr_type:ident, $offset:expr)*) => {
        $(
            impl From<usize> for $addr_type {
                fn from(value: usize) -> Self {
                    Self(value & ((1 << $offset) - 1))
                }
            }
        )*
    };
}

gen_from_usize! {
    PhysAddr,    PA_WIDTH_SV39
    PhysPageNum, PPN_WIDTH_SV39
    VirtAddr,    VA_WIDTH_SV39
    VirtPageNum, VPN_WIDTH_SV39
}

macro_rules! mk_convertion_bridge {
    ($($from:ident <=> $into:ident)*) => {
        $(
            impl From<$from> for $into {
                fn from(value: $from) -> Self {
                    assert!(value.is_aligned(), "{:?} is not page aligned", value);
                    value.floor()
                }
            }

            impl From<$into> for $from {
                fn from(value: $into) -> Self {
                    Self(value.0 << IN_PAGE_OFFSET)
                }
            }
        )*
    };
}

mk_convertion_bridge! {
    PhysAddr <=> PhysPageNum
    VirtAddr <=> VirtPageNum
}

impl VirtAddr {
    /// 从虚拟地址计算虚拟页号（下取整）
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    /// 从虚拟地址计算虚拟页号（下取整）
    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }
    /// 从虚拟地址获取页内偏移（物理地址的低12位）
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// 判断虚拟地址是否与页面大小对齐
    pub fn is_aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

impl PhysAddr {
    /// 从物理地址计算物理页号（下取整）
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }

    /// 从物理地址计算物理页号（上取整）
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }

    /// 从物理地址获取页内偏移（物理地址的低12位）
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    /// 判断物理地址是否与页面大小对齐
    pub fn is_aligned(&self) -> bool {
        self.page_offset() == 0
    }

    /// 获取一个大小为 T 的不可变切片
    pub fn as_ref<T>(&self) -> &'static T {
        unsafe { (self.0 as *const T).as_ref().unwrap() }
    }

    /// 获取一个大小为 T 的可变切片
    pub fn as_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}

impl VirtPageNum {
    /// 取出虚拟页号的三级页索引，并按照从高到低的顺序返回
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511; // 取出低9位
            vpn >>= 9;
        }
        idx
    }
}

// 在实现方面，都是先把物理页号转为物理地址 PhysAddr ，然后再转成 usize 形式的物理地址。
// 接着，我们直接将它转为裸指针用来访问物理地址指向的物理内存。
// 在返回值类型上附加了静态生命周期泛型 'static ，这是为了绕过 Rust 编译器的借用检查，
// 实质上可以将返回的类型也看成一个裸指针，因为它也只是标识数据存放的位置以及类型。
// 但与裸指针不同的是，无需通过 unsafe 的解引用访问它指向的数据，而是可以像一个正常的可变引用一样直接访问
impl PhysPageNum {
    /// 根据自己的PPN取出当前节点的页表项数组
    pub fn as_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }

    /// 返回一个字节数组的可变引用，可以以字节为粒度对物理页帧上的数据进行访问
    pub fn as_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }

    /// 获取一个恰好放在一个物理页帧开头的类型为 T 的数据的可变引用
    pub fn as_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

pub trait StepByOne {
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}
impl StepByOne for PhysPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}

impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}

impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}

pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}

impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}

impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

pub type VPNRange = SimpleRange<VirtPageNum>;
