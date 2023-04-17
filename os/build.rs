fn main() {
    println!(
        "cargo:rerun-if-changed={}",
        "../misc/tests_booter/src/tests_booter.c"
    );
}
