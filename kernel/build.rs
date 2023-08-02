use std::path::Path;

static INITPROC: &str = "../crates/libd/target/riscv64gc-unknown-none-elf/release/initproc";
static INITPROC_SRC: &str = "../crates/libd/src/bin/initproc.rs";

fn main() {
    if !Path::new(INITPROC).exists() {
        panic!("an initproc is needed, please run `cargo build --release` in `crates/libd` first.");
    }
    println!("cargo:rerun-if-changed={}", INITPROC);
    println!("cargo:rerun-if-changed={}", INITPROC_SRC);
}
