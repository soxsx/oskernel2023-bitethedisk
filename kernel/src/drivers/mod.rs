//! Drivers on BTD-OS, used by [board].
//!
//! [board]: crate::board

use alloc::sync::Arc;
use fat32::BlockDevice;

mod fu740;
mod qemu;

#[cfg(feature = "fu740")]
use fu740::BlockDeviceImpl;
#[cfg(not(feature = "fu740"))]
use qemu::BlockDeviceImpl;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}

/// Initialize platform specific device drivers.
pub fn init() {
    #[cfg(feature = "fu740")]
    fu740::init_plic();
}
