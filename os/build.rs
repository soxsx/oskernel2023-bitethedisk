static LINKER_PATH: &str = "src/linker-qemu.ld";

fn main() {
    println!("cargo:rerun-if-changed={}", LINKER_PATH);
}
