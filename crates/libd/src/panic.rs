use crate::syscall::exit;

#[panic_handler]
fn _panic(info: &core::panic::PanicInfo) -> ! {
    let err = info.message().unwrap();
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            err
        );
    } else {
        println!("Panicked: {}", err);
    }
    exit(-1)
}
