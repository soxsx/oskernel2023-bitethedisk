static LINKER: &str = "src/linker.ld";

fn main() {
    println!("cargo:rerun-if-changed={}", LINKER);
}
