static LINKER_PATH: &str = "src/linker-qemu.ld";

fn main() {
    // 由于我们在内核中使用的 linker.ld 是动态生成的，我们需要在 cargo build 前检查
    // linker.ld 模板（这里是 linker-qemu.ld）是否发生了改变。因为这个文件并没有被当前
    // rust 项目识别为与项目有关的文件，所以需要我们手动来监控。
    println!("cargo:rerun-if-changed={}", LINKER_PATH);
}
