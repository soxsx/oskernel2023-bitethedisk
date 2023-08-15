mod virtio_blk;
mod virtio_impl;

use virtio_blk::*;

pub type BlockDeviceImpl = VirtIOBlock;
