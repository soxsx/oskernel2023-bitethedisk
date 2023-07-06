use core::arch::asm;

use alloc::vec::Vec;

use crate::{heap, syscall::exit};


#[linkage = "weak"]
#[link_section = ".text.entry"]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    if let Err(_) = unsafe { heap::init() } {
        panic!("heap init failed");
    };

    let argc: usize;
    let argv: usize;
    unsafe {
        asm!(
            "ld a0, 0(sp)",
            "ld a1, 8(sp)",
            out("a0") argc,
            out("a1") argv
        );
    }

    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }

    exit(main(argc, v.as_slice()));
}

#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: &[&str]) -> isize {
    panic!("cannot find main!");
}
