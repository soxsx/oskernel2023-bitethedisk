pub struct SigSet {
    // pub bits: [usize; SIGSET_LEN],
    pub bits: [u8; 128],
}

impl SigSet {
    pub fn new() -> Self {
        Self { bits: [0; 128] }
    }
}
