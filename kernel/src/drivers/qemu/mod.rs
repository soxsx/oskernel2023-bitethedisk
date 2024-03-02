mod virtio_blk;
mod virtio_gpu;
mod virtio_impl;

use virtio_blk::*;

pub use virtio_gpu::GpuDevice;
pub use virtio_gpu::VirtIOGpuWrapper;
use virtio_gpu::*;
pub type BlockDeviceImpl = VirtIOBlock;
