#[derive(Clone, Copy, Debug)]
pub struct RobustList {
    pub head: usize,
    pub len: usize,
}

impl RobustList {
    pub const HEAD_SIZE: usize = 24;
}

impl Default for RobustList {
    fn default() -> Self {
        Self {
            head: 0,
            len: Self::HEAD_SIZE,
        }
    }
}
