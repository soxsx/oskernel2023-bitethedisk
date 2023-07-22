use super::address::{PhysAddr, PhysPageNum};
use crate::consts::PHYS_END;
use alloc::{collections::BTreeMap, vec::Vec};
use core::fmt::{self, Debug, Formatter};
use spin::Mutex;

/// 物理页帧，代表 RAM 上一段实际的物理页，通过物理页号标识
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    /// 通过物理页号创建一个物理页帧的结构体，创建时初始化内存空间
    pub fn new(ppn: PhysPageNum) -> Self {
        let bytes_arr = ppn.as_bytes_array();
        bytes_arr.into_iter().for_each(|b| *b = 0);

        frame_add_ref(ppn);

        Self { ppn }
    }

    pub fn from_ppn(ppn: PhysPageNum) -> Self {
        frame_add_ref(ppn);
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        dealloc_frame(self.ppn);
    }
}
impl Clone for FrameTracker {
    fn clone(&self) -> Self {
        FrameTracker::from_ppn(self.ppn)
    }
}

/// 物理页帧管理器
trait FrameAllocator {
    /// 新建一个实例，在使用前需要初始化
    fn new() -> Self;
    /// 从空闲物理页中分配一个物理页
    fn alloc(&mut self) -> Option<PhysPageNum>;
    /// 回收物理页
    fn dealloc(&mut self, ppn: PhysPageNum);

    fn add_ref(&mut self, ppn: PhysPageNum);

    fn enquire_ref(&self, ppn: PhysPageNum) -> usize;

    fn usage(&self) -> (usize, usize, usize, usize);
}

/// 栈式物理页帧管理器
pub struct StackFrameAllocator {
    /// 管理内存的起始物理页号
    base_num: usize,
    /// 管理内存的结束物理页号
    end: usize,
    /// 空闲内存的起始物理页号
    current: usize,
    /// 以后入先出的方式保存被回收的物理页号
    recycled: Vec<usize>,
    /// 引用计数器
    refcounter: BTreeMap<usize, u8>,
}

impl StackFrameAllocator {
    /// 初始化栈式物理页管理器
    /// - `l`:空闲内存起始页号
    /// - `r`:空闲内存结束页号
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        self.base_num = l.0;
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            base_num: 0,
            current: 0,
            end: 0,
            recycled: Vec::new(),
            refcounter: BTreeMap::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        // 首先检查栈 recycled 内有没有之前回收的物理页号，如果有的话直接弹出栈顶并返回
        if let Some(ppn) = self.recycled.pop() {
            self.refcounter.insert(ppn, 0);
            Some(ppn.into())
        }
        // 空间满返回 None
        else if self.current == self.end {
            None
        }
        // 否则就返回最低的物理页号
        else {
            self.current += 1;
            self.refcounter.insert(self.current - 1, 0);
            Some((self.current - 1).into())
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        let ref_times = self.refcounter.get_mut(&ppn).unwrap();
        assert!(
            *ref_times > 0,
            "[StackFrameAllocator::dealloc] Frame ppn={:#x} has no reference!",
            ppn
        );
        *ref_times -= 1;
        if *ref_times == 0 {
            self.refcounter.remove(&ppn);
            // 验证物理页号有效性，PPN大于已分配的最高内存或已释放栈中存在这个物理页号
            if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
                panic!(
                    "[StackFrameAllocator::dealloc] Frame ppn={:#x} has not been allocated!",
                    ppn
                );
            }
            // 回收，压栈
            self.recycled.push(ppn);
        }
    }
    fn usage(&self) -> (usize, usize, usize, usize) {
        (self.current, self.recycled.len(), self.end, self.base_num)
    }
    fn add_ref(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        let ref_times = self.refcounter.get_mut(&ppn).unwrap();
        *ref_times += 1;
    }
    fn enquire_ref(&self, ppn: PhysPageNum) -> usize {
        let ppn = ppn.0;
        let ref_times = self.refcounter.get(&ppn).unwrap();
        (*ref_times).clone() as usize
    }
}

/// 物理页帧管理器实例类型
type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// 物理页帧管理器实例
    /// - 全局变量，管理除内核空间外的内存空间
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> =
        Mutex::new(FrameAllocatorImpl::new());
}

/// 初始化物理页帧管理器
/// - 物理页帧范围
///     - 对 `ekernel` 物理地址上取整获得起始物理页号
///     - 对 `PHYS_END` 物理地址下取整获得结束物理页号
pub fn init() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.lock().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(PHYS_END).floor(),
    );
}

/// 分配物理页帧
pub fn alloc_frame() -> Option<FrameTracker> {
    let ppn = FRAME_ALLOCATOR.lock().alloc()?;
    Some(FrameTracker::new(ppn))
}

/// 回收物理页帧
pub fn dealloc_frame(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.lock().dealloc(ppn);
}

pub fn frame_add_ref(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.lock().add_ref(ppn)
}

pub fn enquire_refcount(ppn: PhysPageNum) -> usize {
    FRAME_ALLOCATOR.lock().enquire_ref(ppn)
}
