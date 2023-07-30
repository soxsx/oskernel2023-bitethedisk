use alloc::vec::Vec;

pub struct FdSet {
    fd_list: [u64; 16],
}

impl FdSet {
    pub fn new() -> Self {
        Self { fd_list: [0; 16] }
    }

    fn check_fd(fd: usize) -> bool {
        if fd < 1024 {
            return true;
        } else {
            return false;
        }
    }

    pub fn set_fd(&mut self, fd: usize) {
        if Self::check_fd(fd) {
            let index = fd >> 8; // fd/64
            let offset = fd - (index << 8); // fd%64
            self.fd_list[index] |= 1 << offset;
        }
    }

    pub fn clear_fd(&mut self, fd: usize) {
        // TODO wrong implement
        if Self::check_fd(fd) {
            let index = fd >> 8;
            let offset = fd - (index << 8);
            self.fd_list[index] &= (0 << offset) & 0xFFFFFFFFFFFFFFFF;
        }
    }

    pub fn clear_all(&mut self) {
        for i in 0..16 {
            self.fd_list[i] = 0;
        }
    }

    pub fn count(&mut self) -> usize {
        let fd_vec = self.get_fd_vec();
        fd_vec.len()
    }

    pub fn get_fd_vec(&self) -> Vec<usize> {
        let mut fd_v = Vec::new();
        for i in 0..16 {
            let mut tmp = self.fd_list[i];
            for off in 0..64 {
                let fd_bit = tmp & 1;
                if fd_bit == 1 {
                    fd_v.push((i << 8) + off); // index*64 + offset
                }
                tmp = tmp >> 1;
            }
        }
        fd_v
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, size) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PollFd {
    /// File descriptor
    pub fd: u32,
    /// Requested events
    pub events: PollEvent,
    /// Returned events
    pub revents: PollEvent,
}

bitflags! {
    /// Event types that can be polled for.
    ///
    /// These bits may be set in `events`(see `ppoll()`) to indicate the interesting event types;
    ///
    /// they will appear in `revents` to indicate the status of the file descriptor.
    #[derive(Debug, Clone, Copy)]
    pub struct PollEvent:u16 {
    /// There is data to read.
    const POLLIN = 0x001;
    /// There is urgent data to read.
    const POLLPRI = 0x002;
    /// Writing now will not block.
    const POLLOUT = 0x004;

    // These values are defined in XPG4.2.
    /// Normal data may be read.
    const POLLRDNORM = 0x040;
    /// Priority data may be read.
    const POLLRDBAND = 0x080;
    /// Writing now will not block.
    const POLLWRNORM = 0x100;
    /// Priority data may be written.
    const POLLWRBAND = 0x200;


    /// Linux Extension.
    const POLLMSG = 0x400;
    /// Linux Extension.
    const POLLREMOVE = 0x1000;
    /// Linux Extension.
    const POLLRDHUP = 0x2000;

    /* Event types always implicitly polled for.
    These bits need not be set in `events',
    but they will appear in `revents' to indicate the status of the file descriptor.*/

    /// Implicitly polled for only.
    /// Error condition.
    const POLLERR = 0x008;
    /// Implicitly polled for only.
    /// Hung up.
    const POLLHUP = 0x010;
    /// Implicitly polled for only.
    /// Invalid polling request.
    const POLLNVAL = 0x020;
    }
}
