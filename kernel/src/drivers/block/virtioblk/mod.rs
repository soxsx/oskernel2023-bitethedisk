//! 块设备驱动层

mod virtio_blk;
mod virtio_impl;

pub use virtio_blk::VirtIOBlock;

use crate::board::BlockDeviceImpl;
use alloc::sync::Arc;
use fat32::{BlockDevice, BLOCK_SIZE};

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}

#[allow(unused)]
pub fn block_device_test() {
    let block_device = BLOCK_DEVICE.clone();
    let mut write_buffer = [0u8; 512];
    let mut read_buffer = [0u8; 512];
    for i in 0..512 {
        for byte in write_buffer.iter_mut() {
            *byte = i as u8;
        }
        block_device.write_blocks(&write_buffer, i as usize * BLOCK_SIZE, 1);
        block_device.read_blocks(&mut read_buffer, i as usize * BLOCK_SIZE, 1);
        assert_eq!(write_buffer, read_buffer);
    }
    println!("block device test passed!");
}
