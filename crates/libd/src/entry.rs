use core::{arch::asm, ffi::CStr};

use alloc::{ffi::CString, vec::Vec};

use crate::{heap, syscall::exit};

#[linkage = "weak"]
#[link_section = ".text.entry"]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    let mut argc: usize;
    let mut argv: usize;
    asm! {
        "ld a0, 0(sp)",
        "ld a1, 8(sp)",
        out("a0") argc,
        out("a1") argv,
    }

    if let Err(_) = heap::init() {
        panic!("heap init failed");
    };

    let mut v: Vec<CString> = Vec::new();
    for i in 0..argc {
        let c_char_ptr =
            ((argv + i * core::mem::size_of::<usize>()) as *const u8).read_volatile() as *mut i8;
        v.push(CString::from(CStr::from_ptr(c_char_ptr)));
    }
    exit(main(argc, v));
}

#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: Vec<CString>) -> isize {
    panic!("cannot find main!");
}
