use alloc::{string::String, vec::Vec};

pub struct BuildinTestsManager {
    ntest: usize,              // 测试程序的总数目
    npassed: usize,            // 通过的数目
    failed_names: Vec<String>, // 失败的测试程序的名称
}
