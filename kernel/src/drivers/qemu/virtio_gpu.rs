//!  Block device under VirtIO.
use super::virtio_impl::HalImpl;
use core::ptr::NonNull;
use fat32::{BlockDevice, BLOCK_SIZE};
use embedded_graphics::pixelcolor::Rgb888;
use spin::Mutex;
use core::any::Any;
use alloc::vec::Vec;
use virtio_drivers::{
    device::gpu::VirtIOGpu,
    transport::mmio::{MmioTransport, VirtIOHeader},
};
use tinybmp::Bmp;

const VIRTIO7: usize = 0x10007000;
pub trait GpuDevice: Send + Sync + Any {
    fn update_cursor(&self);
    fn get_framebuffer(&self) -> &mut [u8];
    fn flush(&self);
}


unsafe impl Send for VirtIOGpuWrapper{}
unsafe impl Sync for VirtIOGpuWrapper{}

pub struct VirtIOGpuWrapper {
    gpu: Mutex<VirtIOGpu<HalImpl, MmioTransport>>,
    fb: &'static [u8],
}
static BMP_DATA: &[u8] = include_bytes!("../../assert/mouse.bmp");
impl VirtIOGpuWrapper {
    pub fn new() -> Self {
        unsafe {
	    let header = NonNull::new(VIRTIO7 as *mut VirtIOHeader).unwrap();		
	    let mut virtio= match unsafe {MmioTransport::new(header)}{
		Err(e) =>{
                    panic!("Error creating GPU VirtIO MMIO transport: {}", e)
		}
		Ok(transport)=>
		    VirtIOGpu::<HalImpl,MmioTransport>::new(transport)
		    .expect("failed to create gpu driver")
	    };
            let fbuffer = virtio.setup_framebuffer().unwrap();
            let len = fbuffer.len();
            let ptr = fbuffer.as_mut_ptr();
            let fb = core::slice::from_raw_parts_mut(ptr, len);

            let bmp = Bmp::<Rgb888>::from_slice(BMP_DATA).unwrap();
            let raw = bmp.as_raw();
            let mut b = Vec::new();
            for i in raw.image_data().chunks(3) {
                let mut v = i.to_vec();
                b.append(&mut v);
                if i == [255, 255, 255] {
                    b.push(0x0)
                } else {
                    b.push(0xff)
                }
            }
            virtio.setup_cursor(b.as_slice(), 50, 50, 50, 50).unwrap();
            Self {
                gpu: Mutex::new(virtio),
                fb,
            }
        }
    }
}

impl GpuDevice for VirtIOGpuWrapper {
    fn flush(&self) {
        self.gpu.lock().flush().unwrap();
    }
    fn get_framebuffer(&self) -> &mut [u8] {
        unsafe {
            let ptr = self.fb.as_ptr() as *const _ as *mut u8;
            core::slice::from_raw_parts_mut(ptr, self.fb.len())
        }
    }
    fn update_cursor(&self) {}
}
