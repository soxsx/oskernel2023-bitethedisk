// use crate::error::Error;
use alloc::{collections::VecDeque, string::String};
use core::fmt::{Debug, Formatter};
use core::ops::{Deref, DerefMut};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path {
    pub components: VecDeque<String>,
}

impl Debug for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (idx, p) in self.components.iter().enumerate() {
            if idx == 0 && p == "/" {
                write!(f, "/")?;
                continue;
            }
            write!(f, "{}/", p)?;
        }
        Ok(())
    }
}

impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap()
    }
}

impl Deref for Path {
    type Target = VecDeque<String>;

    fn deref(&self) -> &Self::Target {
        &self.components
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.components
    }
}

impl Path {
    pub fn from_string(path: String) -> Option<Self> {
        let mut temp: VecDeque<String> = path.split('/').map(|s| String::from(s)).collect();

        if path.starts_with('/') {
            temp.push_front(String::from("/"));
        }

        let mut components = VecDeque::new();
        for name in temp {
            if name == "." || name == "" {
                continue;
            } else if name == ".." {
                let ret = components.pop_back();
                if ret.is_none() {
                    return None;
                }
            } else {
                components.push_back(name);
            }
        }
        Some(Self { components })
    }

    pub fn from_str(str: &str) -> Option<Path> {
        Path::from_string(String::from(str))
    }

    pub fn as_string(&self) -> String {
        format!("{:?}", self)
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

    pub fn without_prefix(&self, prefix: &Path) -> Self {
        assert!(self.starts_with(prefix), "not prefix");
        let mut new = self.clone();
        for _ in 0..prefix.len() {
            new.pop_front();
        }
        new
    }

    /// Whether it is started with the prefix
    pub fn starts_with(&self, prefix: &Path) -> bool {
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
}

// 绝对路径
pub struct AbsolutePath(Path);

impl AbsolutePath {
    pub fn from_string(path: String) -> Option<Self> {
        let path = Path::from_string(path)?;
        if path.is_root() {
            Some(Self(path))
        } else {
            None
        }
    }

    pub fn from_str(str: &str) -> Option<Self> {
        Self::from_string(String::from(str))
    }

    pub fn as_string(&self) -> String {
        self.0.as_string()
    }

    // pub fn as_str(&self) -> &str {
    //     self.0.as_string().as_str()
    // }

    // pub fn as_bytes(&self) -> &[u8] {
    //     self.0.as_string().as_bytes()
    // }
}

impl Debug for AbsolutePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (idx, p) in self.0.components.iter().enumerate() {
            if idx == 0 {
                assert!(p == "/");
                write!(f, "/")?;
                continue;
            }
            write!(f, "{}/", p)?;
        }
        Ok(())
    }
}

#[allow(unused)]
pub fn path_test() {
    let path = Path::from_string(String::from("/a/b/c/d/")).unwrap();
    println!("path = {:?}", path);
    let path = Path::from_string(String::from("/abcdefg/asdsd/asdasd")).unwrap();
    println!("path = {:?}", path);
    let path = Path::from_string(String::from("aa/../bb/../cc/././."));
    println!("path = {:?}", path);
    let path = Path::from_string(String::from("aa/../.."));
    println!("path = {:?}", path);
    let path = Path::from_string(String::from("./././."));
    println!("path = {:?}", path);
}
