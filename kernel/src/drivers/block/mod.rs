mod sdcard;
mod virtioblk;

use alloc::sync::Arc;

pub use virtioblk::*;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<BlockDeviceImpl> = Arc::new(BlockDeviceImpl::new());
}
