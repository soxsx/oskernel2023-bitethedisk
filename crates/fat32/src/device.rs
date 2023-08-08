//! Block device trait for FAT32.

use crate::Error;

use core::any::Any;
use core::marker::{Send, Sync};
use core::result::Result;

pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, blk_id: usize, buf: &mut [u8]) -> Result<(), Error>;
    fn write_block(&self, blk_id: usize, buf: &[u8]) -> Result<(), Error>;
}
