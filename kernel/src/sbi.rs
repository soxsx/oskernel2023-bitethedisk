use core::arch::asm;

const SBI_SET_TIMER: i32 = 0;
const SBI_CONSOLE_PUTCHAR: i32 = 1;
const SBI_CONSOLE_GETCHAR: i32 = 2;
const SBI_SHUTDOWN: i32 = 8;

#[inline(always)]
fn sbi_call(which: i32, arg0: i32, arg1: i32, arg2: i32) -> i32 {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") which,
        );
    }
    ret
}

pub fn set_timer(stime_value: i32) -> i32 {
    sbi_call(SBI_SET_TIMER, stime_value, 0, 0)
}

pub fn console_putchar(c: i32) -> i32 {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0)
}

pub fn console_getchar() -> i32 {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    unreachable!("kernel has already shutdown");
}
