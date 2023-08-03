use alloc::vec::Vec;

/// 应用地址空间中的一段缓冲区(即内存)的抽象
///
/// - `buffers`: 位于应用地址空间中, 内核无法直接通过用户地址空间的虚拟地址来访问, 因此需要进行封装
#[derive(Debug)]
pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

#[allow(unused)]
impl UserBuffer {
    /// 使用 `buffer` 创建一个新的缓冲区实例
    pub fn empty() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }
    pub fn wrap(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }

        total
    }
    // 将一个Buffer的数据写入UserBuffer, 返回写入长度
    pub fn write(&mut self, buff: &[u8]) -> usize {
        let len = self.len().min(buff.len());
        let mut current = 0;
        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                (*sub_buff)[j] = buff[current];
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        len
    }
    pub fn write_zeros(&mut self) -> usize {
        let len = self.len();
        let mut current = 0;
        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                (*sub_buff)[j] = 0;
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        len
    }

    pub fn write_at(&mut self, offset: usize, buff: &[u8]) -> isize {
        let len = buff.len();
        if offset + len > self.len() {
            panic!();
        }
        let mut head = 0; // offset of slice in UBuffer
        let mut current = 0; // current offset of buff
        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            if head + sblen < offset {
                head += sblen;
                continue;
            } else if head < offset {
                for j in (offset - head)..sblen {
                    (*sub_buff)[j] = buff[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            } else {
                //head + sblen > offset and head > offset
                for j in 0..sblen {
                    (*sub_buff)[j] = buff[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            }
            head += sblen;
        }
        0
    }

    // 将UserBuffer的数据读入一个Buffer, 返回读取长度
    pub fn read(&self, buff: &mut [u8]) -> usize {
        let len = self.len().min(buff.len());
        let mut current = 0;
        for sub_buff in self.buffers.iter() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                buff[current] = (*sub_buff)[j];
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        return len;
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
