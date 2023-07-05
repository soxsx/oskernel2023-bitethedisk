use crate::syscall::exit;

#[link_section = ".text.entry"]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    exit(main());
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> isize {
    panic!("cannot find main!");
}
