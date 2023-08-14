//! Block devices under VirtIO bus architecture
//!
//! The examples provided by both rCore-tutorial and virtio_drivers serve as references for implementation.
//! [reference](https://github.com/rcore-os/virtio-drivers/tree/master/examples/riscv)

use super::virtio_impl::HalImpl;
use core::ptr::NonNull;
use fat32::{BlockDevice, BlockDeviceError, BLOCK_SIZE};
use spin::Mutex;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::mmio::{MmioTransport, VirtIOHeader},
};

#[allow(unused)]
const VIRTIO0: usize = 0x10001000; // TODO ???

pub struct VirtIOBlock(Mutex<VirtIOBlk<HalImpl, MmioTransport>>);

// TODO In order to pass the compilation (due to the constraints of the BlockDevice trait).
// Don’t know if it will cause problems.
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
        // VirtIOBlk::read_block() only one block at a time
        assert_eq!(buf.len(), BLOCK_SIZE);
        assert!(offset % BLOCK_SIZE == 0);
        self.0
            .lock()
            .read_blocks(block_id, buf)
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
            .write_blocks(block_id, buf)
            .expect("Error when writing VirtIOBlk");
        Ok(())
    }
}

/// Refer to the examples provided by virtio_drivers for implementation.
/// [reference](https://github.com/rcore-os/virtio-drivers/tree/master/examples/riscv)
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
