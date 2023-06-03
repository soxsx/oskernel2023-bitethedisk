// use crate::error::Error;
use alloc::vec::Vec;
use alloc::{collections::VecDeque, string::String};
use core::fmt::{Debug, Formatter};
use core::ops::{Deref, DerefMut};

use crate::fs::path;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AbsolutePath {
    pub components: VecDeque<String>,
}

impl From<&str> for AbsolutePath {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl Deref for AbsolutePath {
    type Target = VecDeque<String>;

    fn deref(&self) -> &Self::Target {
        &self.components
    }
}

impl DerefMut for AbsolutePath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.components
    }
}

impl AbsolutePath {
    pub fn from_string(path: String) -> Self {
        let temp: VecDeque<String> = path.split('/').map(|s| String::from(s)).collect();

        let mut components = VecDeque::new();
        for name in temp {
            if name == "." || name == "" {
                continue;
            } else if name == ".." {
                let ret = components.pop_back();
                if ret.is_none() {
                    components.push_back(name);
                }
            } else {
                components.push_back(name);
            }
        }
        Self { components }
    }

    pub fn from_str(str: &str) -> AbsolutePath {
        AbsolutePath::from_string(String::from(str))
    }

    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }

    pub fn as_vec_str(&self) -> Vec<&str> {
        self.components.iter().map(|s| s.as_str()).collect()
    }

    /// Whether it is the root
    pub fn is_root(&self) -> bool {
        return self.components.len() == 0;
    }

    /// Get the tail of the path
    pub fn last(&self) -> String {
        if self.is_root() {
            panic!("is_root")
        }
        return self.components[self.len() - 1].clone();
    }

    #[allow(unused)]
    pub fn first(&self) -> String {
        return self.components[0].clone();
    }

    /// Remove the head of the path
    #[allow(unused)]
    pub fn remove_head(&self) -> Self {
        if self.is_root() {
            panic!("already root")
        }
        let mut new = self.clone();
        new.pop_front();
        new
    }

    /// Remove the tail of the path
    pub fn remove_tail(&self) -> Self {
        if self.is_root() {
            panic!("already root")
        }
        let mut new = self.clone();
        new.pop_back();
        new
    }

    pub fn without_prefix(&self, prefix: &AbsolutePath) -> Self {
        assert!(self.starts_with(prefix), "not prefix");
        let mut new = self.clone();
        for _ in 0..prefix.len() {
            new.pop_front();
        }
        new
    }

    /// Whether it is started with the prefix
    pub fn starts_with(&self, prefix: &AbsolutePath) -> bool {
        if prefix.len() == 0 {
            return true;
        }
        if prefix.len() > self.len() {
            return false;
        }
        for (this_i, pre_i) in self.components.iter().zip(prefix.components.iter()) {
            if this_i != pre_i {
                return false;
            }
        }
        true
    }

    // 一般来说, path 是相对路径
    pub fn join_string(&self, path: String) -> Self {
        if path.starts_with('/') {
            warn!("join_string: path starts with /");
            return Self::from_string(path);
        }
        let mut new = self.clone();
        let path = AbsolutePath::from_string(path);
        for p in path.components.iter() {
            new.push_back(p.clone());
        }
        new
    }

    /// 根据传入的 path 拼接成新的 absolute_path 并返回, 不改变原有的 absolute_path
    pub fn cd(&self, path: String) -> Option<Self> {
        let tmp = AbsolutePath::from_string(path.clone());
        if path.starts_with('/') {
            return Some(tmp);
        }
        let mut new = self.clone();
        for p in tmp.components.iter() {
            if p == ".." {
                new.pop_back()?;
            } else {
                new.push_back(p.clone());
            }
        }
        Some(new)
    }
}

impl Debug for AbsolutePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "/")?;
        for p in self.components.iter() {
            write!(f, "{}/", p)?;
        }
        Ok(())
    }
}

#[allow(unused)]
pub fn path_test() {
    let path = AbsolutePath::from_string(String::from("/a/b/c/d/"));
    println!("path = {:?}", path);
    let path = AbsolutePath::from_string(String::from("/abcdefg/asdsd/asdasd"));
    println!("path = {:?}", path);
    let path = AbsolutePath::from_string(String::from("aa/../bb/../cc/././."));
    println!("path = {:?}", path);
    let path = AbsolutePath::from_string(String::from("aa/../.."));
    println!("path = {:?}", path);
    let path = AbsolutePath::from_string(String::from("./././."));
    println!("path = {:?}", path);

    // test cd
    let abs_path = AbsolutePath::from_string(String::from("/a/b/c/d/"));
    let path = String::from("../e/../f/g");
    let new_path = abs_path.cd(path).unwrap();
    println!("new_path = {:?}", new_path);

    // test join
    let abs_path = AbsolutePath::from_string(String::from("/a/b/c/d/"));
    let new_path = abs_path.join_string(String::from("../e/../f/g"));
    println!("new_path = {:?}", new_path);
}
