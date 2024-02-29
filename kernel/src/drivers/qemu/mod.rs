mod virtio_blk;
mod virtio_gpu;
mod virtio_impl;

use virtio_blk::*;

use virtio_gpu::*;
pub use virtio_gpu::VirtIOGpuWrapper;
pub use virtio_gpu::GpuDevice;
pub type BlockDeviceImpl = VirtIOBlock;
