#![no_std]
#![no_main]
// Features, need nightly toolchain.
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
#![feature(error_in_core)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

#[cfg(feature = "time-tracer")]
#[macro_use]
extern crate time_tracer;

#[macro_use]
mod macros;
#[macro_use]
mod console;

#[path = "boards/qemu.rs"]
mod board;

mod consts;
mod drivers;
mod fs;
mod logging;
mod mm;
mod panic;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

use sbi::sbi_start_hart;

use crate::consts::NCPU;
use core::{arch::global_asm, slice, sync::atomic::AtomicBool};

global_asm!(include_str!("entry.S"));

lazy_static! {
    static ref BOOTED: AtomicBool = AtomicBool::new(false);
}

#[no_mangle]
pub fn meow() -> ! {
    if BOOTED.load(core::sync::atomic::Ordering::Relaxed) {
        other_harts()
    }

    println!("Boot hart: {}", hartid!());

    init_bss();
    logging::init();
    mm::init();
    trap::init();
    trap::enable_stimer_interrupt();
    timer::set_next_trigger();
    fs::init();
    task::add_initproc();

    BOOTED.store(true, core::sync::atomic::Ordering::Relaxed);
    wake_other_harts_hsm();

    task::run_tasks();
    unreachable!()
}

fn wake_other_harts_hsm() {
    extern "C" {
        fn _entry();
    }
    let boot_hartid = hartid!();
    for i in 1..NCPU {
        sbi_start_hart((boot_hartid + i) % NCPU, _entry as usize, 0).unwrap();
    }
}

#[allow(unused)]
fn wake_other_harts_ipi() {
    use sbi::sbi_send_ipi;
    let boot_hart = hartid!();
    let target_harts_mask = ((1 << NCPU) - 1) ^ boot_hart;
    sbi_send_ipi(target_harts_mask, (&target_harts_mask) as *const _ as usize).unwrap();
}

fn other_harts() -> ! {
    info!("hart {} has been started", hartid!());
    mm::enable_mmu();
    trap::init();
    trap::enable_stimer_interrupt();
    timer::set_next_trigger();
    task::run_tasks();
    unreachable!()
}

fn init_bss() {
    extern "C" {
        fn ekstack0();
        fn ebss();
    }
    unsafe {
        let sbss = ekstack0 as usize as *mut u8;
        let ebss = ebss as usize as *mut u8;
        slice::from_mut_ptr_range(sbss..ebss)
            .into_iter()
            .for_each(|byte| (byte as *mut u8).write_volatile(0));
    }
}
