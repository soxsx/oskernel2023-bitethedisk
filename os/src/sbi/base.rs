//! Reference:
//! <https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc#4-base-extension-eid-0x10>
use super::{sbi_call, SbiRet, EID_BASE};
use core::arch::asm;

/// 返回当前 SBI 实现的版本号，这个调用由规范保证永远不会失败，即造成 panic
///
/// 小版本号被编码在第 24 位，主版本号编码在剩下的 7 位。
///
/// 注意这是 32 位的，但我们的 kernel 是 64 位的，所以使用需要注意
pub fn sbi_spec_version() -> usize {
    let ret = sbi_call(EID_BASE, 0, 0, 0);
    let version = ret.value;

    version
}

/// 返回当前 SBI 实现的 id，不同的实现比如 RustSBI, OpenSBI
///
/// 相关的实现 id 可以从下面的网址找到：
/// <https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc#sbi-implementation-ids>
pub fn sbi_impl_id() -> usize {
    let ret = sbi_call(EID_BASE, 1, 0, 0);
    let impl_id = ret.value;

    impl_id
}
