/// Get the current CPU id.
///
/// According to the RISC-V SBI specification, SBI will read `mhartid` to the `a0` register.
///
/// We read `a0` to `tp` in `entry.S`, and then do not change the value in `tp`.
macro_rules! hartid {
    () => {{
        let hartid: usize;
        unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
        hartid
    }};
}
