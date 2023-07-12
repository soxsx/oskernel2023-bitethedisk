/// 获取当前 CPU id
///
/// 通过 RISC-V SBI 规范，SBI 会将 `mhartid` 读到 `a0` 寄存器
///
/// 我们在 `entry.S` 中将 `a0` 读到了 `tp` 中，之后不再更改 `tp` 中的值
macro_rules! hartid {
    () => { {
        let hartid: usize;
        unsafe {core::arch::asm!("mv {}, tp", out(reg) hartid)}

        hartid
    }};
}
