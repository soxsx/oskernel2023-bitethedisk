#[cfg(feature = "fu740")]
mod fu740;
#[cfg(not(feature = "fu740"))]
mod qemu;

#[cfg(feature = "fu740")]
pub use fu740::*;
#[cfg(not(feature = "fu740"))]
pub use qemu::*;
