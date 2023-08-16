use riscv::register::sstatus::{self, Sstatus, SPP};

#[derive(Clone, Debug)]
#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    sstatus: Sstatus,
    pub sepc: usize,
    /// Kernel space address translation and protection
    /// (KSpace Page Table Token)
    kernel_satp: usize,
    /// Process kernel stack pointer(virtual address)
    pub kernel_sp: usize,
    /// Trap handler entry point(virtual address)
    trap_handler: usize,
    /// The Hart ID of the executor that this context belongs to
    executor_id: usize,
    // Floating point registers
    pub freg: [usize; 32],
}

impl TrapContext {
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let sstatus = sstatus::read();
        unsafe { sstatus::set_spp(SPP::User) }
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // ELF entry point
            kernel_satp,
            kernel_sp,
            trap_handler,
            executor_id: hartid!(),
            freg: [0; 32],
        };
        cx.set_sp(sp);
        cx
    }

    /// General purpose register `x0`
    #[allow(unused)]
    #[inline(always)]
    pub fn zero(&self) -> usize {
        0
    }
}

macro_rules! gen_register_getter_setter {
    ($trap_cx:ident, $($reg:ident, $offset:expr)+) => {
        #[allow(unused)]
        impl $trap_cx {
            paste::paste! {
            $(
                #[inline(always)]
                pub fn [<set_ $reg>](&mut self, val: usize) {
                    self.x[$offset] = val;
                }


                #[inline(always)]
                pub fn [<$reg>](&self) -> usize {
                    self.x[$offset]
                }
            )+
            }
        }
    };
}

gen_register_getter_setter! {
    TrapContext,

    ra, 1
    sp, 2
    gp, 3
    tp, 4
    t0, 5
    t1, 6
    t2, 7

    s0, 8
    fp, 8

    s1, 9
    a0, 10
    a1, 11
    a2, 12
    a3, 13
    a4, 14
    a5, 15
    a6, 16
    a7, 17
    s2, 18
    s3, 19
    s4, 20
    s5, 21
    s6, 22
    s7, 23
    s8, 24
    s9, 25
    s10, 26
    s11, 27
    t3, 28
    t4, 29
    t5, 30
    t6, 31
}
