/// 提供了自旋锁
/// 但是未经过测试
pub(crate) mod mutex;
/// ### 允许在单核处理器上将引用做全局变量使用
/// `os/src/sync/mod.rs`
///
mod up;

pub use up::UPSafeCell;
