use core::fmt::Debug;
use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    /// 通用寄存器`x[0]~x[31]`
    pub x: [usize; 32],
    /// 提供状态信息
    pub sstatus: Sstatus,
    /// 记录 Trap 发生之前执行的最后一条指令的地址
    pub sepc: usize,
    // 一下数据在应用初始化的时候由内核写入应用地址空间中的TrapContext中的相应位置，此后不再修改
    /// 内核地址空间的 token ，即内核页表的起始物理地址
    pub kernel_satp: usize,
    /// 当前应用在内核地址空间中的内核栈栈顶的虚拟地址
    pub kernel_sp: usize,
    /// 内核中 trap handler 入口点的虚拟地址
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        unsafe {
            sstatus::set_spp(SPP::User);
        }
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // Trap 返回后到程序入口
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}

impl Debug for TrapContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TrapContext")
            .field("x0", &self.x[0])
            .field("ra", &self.x[1])
            .field("sp", &self.x[2])
            .field("gp", &self.x[3])
            .field("tp", &self.x[4])
            .field("t0", &self.x[5])
            .field("t1", &self.x[6])
            .field("t2", &self.x[7])
            .field("s0/fp", &self.x[8])
            .field("s1", &self.x[9])
            .field("a0", &self.x[10])
            .field("a1", &self.x[11])
            .field("a2", &self.x[12])
            .field("a3", &self.x[13])
            .field("a4", &self.x[14])
            .field("a5", &self.x[15])
            .field("a6", &self.x[16])
            .field("a7", &self.x[17])
            .field("s2", &self.x[18])
            .field("s3", &self.x[19])
            .field("s4", &self.x[20])
            .field("s5", &self.x[21])
            .field("s6", &self.x[22])
            .field("s7", &self.x[23])
            .field("s8", &self.x[24])
            .field("s9", &self.x[25])
            .field("s10", &self.x[26])
            .field("s11", &self.x[27])
            .field("t3", &self.x[28])
            .field("t4", &self.x[29])
            .field("t5", &self.x[30])
            .field("t6", &self.x[31])
            .field("sstatus", &self.sstatus)
            .field("sepc", &self.sepc)
            .field("kernel_satp", &self.kernel_satp)
            .field("kernel_sp", &self.kernel_sp)
            .field("trap_handler", &self.trap_handler)
            .finish()
    }
}
