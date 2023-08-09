//! Drivers on BTD-OS, used by [board].
//!
//! [board]: crate::board

use alloc::sync::Arc;
use fat32::BlockDevice;

pub mod block;

use block::virtioblk::BlockDeviceImpl;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}
