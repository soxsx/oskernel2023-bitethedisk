//! VirtIO 总线架构下的块设备

use super::BlockDevice;
use core::ptr::NonNull;
use spin::Mutex;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::mmio::{MmioTransport, VirtIOHeader},
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

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
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
