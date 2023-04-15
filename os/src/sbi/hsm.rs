//! Hart State Management Extension (EID #0x48534D "HSM")

use super::{sbi_call, SbiRet, EID_HSM};

/// Reference:
///
/// <https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc#table_hsm_states>
pub(crate) enum HartState {
    /// The hart is physically powered-up and executing normally.
    Started = 0,

    /// The hart is not executing in supervisor-mode or any lower privilege mode.
    /// It is probably powered-down by the SBI implementation if the underlying platform
    /// has a mechanism to physically power-down harts.
    Stopped = 1,

    /// Some other hart has requested to start (or power-up) the hart from the STOPPED
    /// state and the SBI implementation is still working to get the hart in the STARTED state.
    StartPending = 2,

    /// The hart has requested to stop (or power-down) itself from the STARTED state
    /// and the SBI implementation is still working to get the hart in the STOPPED state.
    StopPending = 3,

    /// This hart is in a platform specific suspend (or low power) state.
    Suspended = 4,

    /// The hart has requested to put itself in a platform specific low power state
    /// from the STARTED state and the SBI implementation is still working to get the
    /// hart in the platform specific SUSPENDED state.
    SuspendPending = 5,

    /// An interrupt or platform specific hardware event has caused the hart to
    /// resume normal execution from the SUSPENDED state and the SBI implementation
    /// is still working to get the hart in the STARTED state.
    ResumePending = 6,
}

// 这个函数放在这好像有点不太合适，需要再想想
#[inline(always)]
pub fn get_hartid() -> usize {
    let hart_id: usize;
    unsafe {
        core::arch::asm!("mv {}, tp", out(reg) hart_id);
    }

    hart_id
}

pub fn hart_stop() -> SbiRet {
    sbi_call(EID_HSM, 1, 0, 0)
}

pub fn hart_get_status(hartid: usize) -> SbiRet {
    sbi_call(EID_HSM, 2, hartid, 0)
}

pub fn hart_suspend() -> SbiRet {
    sbi_call(EID_HSM, 3, 0, 0)
}
