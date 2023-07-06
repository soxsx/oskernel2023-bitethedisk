use crate::{heap, syscall::exit};

#[link_section = ".text.entry"]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    if let Err(_) = unsafe { heap::init() } {
        panic!("heap init failed");
    };
    exit(main());
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> isize {
    panic!("cannot find main!");
}
