//! VirtIO 总线架构下的块设备

use super::BlockDevice;
use crate::{
    kernel_token,
    mm::{
        alloc_frame, dealloc_frame, FrameTracker, PageTable, PhysAddr, PhysPageNum, StepByOne,
        VirtAddr,
    },
};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr::NonNull;
use spin::Mutex;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        self,
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

use super::virtio_impl::HalImpl;

#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

/// VirtIO 总线架构下的块设备
///
/// 将 `virtio-drivers` crate 提供的 VirtIO 块设备抽象 `VirtIOBlk` 包装为我们自己的 `VirtIOBlock`
pub struct VirtIOBlock(Mutex<VirtIOBlk<HalImpl, MmioTransport>>);

unsafe impl Send for VirtIOBlock {}
unsafe impl Sync for VirtIOBlock {}

impl VirtIOBlock {
    pub fn new() -> Self {
        let header = NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap();
        let mut blk = match unsafe { MmioTransport::new(header) } {
            Err(e) => {
                panic!("Error creating VirtIO MMIO transport: {}", e)
            }
            Ok(transport) => VirtIOBlk::<HalImpl, MmioTransport>::new(transport)
                .expect("failed to create blk driver"),
        };

        Self(Mutex::new(blk))
    }
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        // println!("block_id:{}",block_id);
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
    fn handle_irq(&self) {
        todo!()
    }
}

// use virtio_drivers::{VirtIOBlk, VirtIOHeader};
// lazy_static! {
//     static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());
// }
// pub struct  VirtIOBlock(Mutex<VirtIOBlk<'static>>);
// impl VirtIOBlock {
//     #[allow(unused)]
//     pub fn new() -> Self {
//         unsafe {
//             Self(Mutex::new(
//                 // VirtIOHeader 实际上就代表以 MMIO 方式访问 VirtIO 设备所需的一组设备寄存器
//                 VirtIOBlk::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
//             ))
//         }
//     }
// }
// #[no_mangle]
// pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
//     let mut ppn_base = PhysPageNum(0);
//     for i in 0..pages {
//         let frame = alloc_frame().unwrap();
//         if i == 0 {
//             ppn_base = frame.ppn;
//         }
//         assert_eq!(frame.ppn.0, ppn_base.0 + i);
//         QUEUE_FRAMES.lock().push(frame);
//     }
//     ppn_base.into()
// }
// #[no_mangle]
// pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
//     let mut ppn_base: PhysPageNum = pa.into();
//     for _ in 0..pages {
//         dealloc_frame(ppn_base);
//         ppn_base.step();
//     }
//     0
// }
// #[no_mangle]
// pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
//     VirtAddr(paddr.0)
// }
// #[no_mangle]
// pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
//     PageTable::from_token(kernel_token!())
//         .translate_va(vaddr)
//         .unwrap()
// }
