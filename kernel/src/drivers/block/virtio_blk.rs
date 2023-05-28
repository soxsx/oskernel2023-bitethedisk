//! # VirtIO 总线架构下的块设备
//! `os/src/drivers/block/virtio_blk.rs`
//! ```
//! pub struct VirtIOBlock
//! pub fn virtio_dma_alloc ()
//! pub fn virtio_dma_dealloc ()
//! pub fn virtio_phys_to_virt ()
//! pub fn virtio_virt_to_phys ()
//! ```
use crate::{
    kernel_token,
    mm::{
        alloc_frame, dealloc_frame, FrameTracker, PageTable, PhysAddr as KPhysAddr, PhysPageNum,
        StepByOne, VirtAddr,
    },
};

use core::ptr::NonNull;
use fat32::{BlockDevice, BlockDeviceError, BLOCK_SIZE};
use spin::Mutex;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        self,
        mmio::{MmioTransport, VirtIOHeader},
    },
};

use super::virtio_impl::HalImpl;

#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

/// ### VirtIO 总线架构下的块设备
/// 将 `virtio-drivers` crate 提供的 VirtIO 块设备抽象 `VirtIOBlk` 包装为我们自己的 `VirtIOBlock` ,
/// 实质上只是加上了一层互斥锁, 生成一个新的类型来实现 easy-fs 需要的 `BlockDevice` Trait
/// ```
/// fn read_block()
/// fn write_block()
/// pub fn new()
/// ```
///

pub struct VirtIOBlock(Mutex<VirtIOBlk<HalImpl, MmioTransport>>);

unsafe impl Send for VirtIOBlock {}
unsafe impl Sync for VirtIOBlock {}

impl BlockDevice for VirtIOBlock {
    fn read_blocks(
        &self,
        buf: &mut [u8],
        offset: usize,
        _block_cnt: usize,
    ) -> Result<(), BlockDeviceError> {
        let block_id = offset / BLOCK_SIZE;

        // VirtIOBlk::read_block() 只能读取一个块
        assert_eq!(buf.len(), BLOCK_SIZE);
        assert!(offset % BLOCK_SIZE == 0);

        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
        Ok(())
    }
    fn write_blocks(
        &self,
        buf: &[u8],
        offset: usize,
        _block_cnt: usize,
    ) -> Result<(), BlockDeviceError> {
        let block_id = offset / BLOCK_SIZE;
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
        Ok(())
    }
}

impl VirtIOBlock {
    pub fn new() -> Self {
        let header = NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap();
        let blk = match unsafe { MmioTransport::new(header) } {
            Err(e) => {
                panic!("Error creating VirtIO MMIO transport: {}", e)
            }
            Ok(transport) => VirtIOBlk::<HalImpl, MmioTransport>::new(transport)
                .expect("failed to create blk driver"),
        };

        Self(Mutex::new(blk))
    }
}
