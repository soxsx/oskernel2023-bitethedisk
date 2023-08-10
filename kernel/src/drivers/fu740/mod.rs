mod plic;
mod sdcard;
mod spi;

pub use plic::init_plic;

pub type BlockDeviceImpl = sdcard::SDCardWrapper;
