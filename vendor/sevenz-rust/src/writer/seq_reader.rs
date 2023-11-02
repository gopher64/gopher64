use std::{
    fs::File,
    io::{self, Read},
    ops::Deref,
    path::{Path, PathBuf},
};
#[derive(Default)]
pub struct SeqReader<R> {
    readers: Vec<R>,
    current: usize,
}

impl<R> From<Vec<R>> for SeqReader<R> {
    fn from(value: Vec<R>) -> Self {
        Self::new(value)
    }
}

impl<R> Deref for SeqReader<R> {
    type Target = [R];

    fn deref(&self) -> &Self::Target {
        &self.readers
    }
}

impl<R> AsRef<[R]> for SeqReader<R> {
    fn as_ref(&self) -> &[R] {
        &self.readers
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SeqReader<File> {
    pub fn from_path_iter<'a>(paths: impl Iterator<Item = &'a Path>) -> std::io::Result<Self> {
        let mut readers = Vec::new();
        for path in paths {
            readers.push(File::open(path)?);
        }
        Ok(Self::new(readers))
    }
}

impl<R> SeqReader<R> {
    pub fn new(readers: Vec<R>) -> Self {
        Self {
            readers,
            current: 0,
        }
    }

    pub fn reader_len(&self) -> usize {
        self.readers.len()
    }
}

impl<R: Read> Read for SeqReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut i = 0;
        while self.current < self.readers.len() {
            let r = &mut self.readers[self.current];
            i = r.read(buf)?;
            if i == 0 {
                self.current += 1;
            } else {
                break;
            }
        }

        Ok(i)
    }
}

pub struct SourceReader<R> {
    reader: R,
    size: usize,
    crc: crc::Digest<'static, u32>,
    crc_value: u32,
}

impl<R> From<R> for SourceReader<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<R: Read> Read for SourceReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf)?;
        if self.crc_value == 0 {
            if n > 0 {
                self.size += n;
                self.crc.update(&buf[..n]);
            } else {
                let crc = std::mem::replace(&mut self.crc, crate::reader::CRC32.digest());
                self.crc_value = crc.finalize();
            }
        }
        Ok(n)
    }
}

impl<R> SourceReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            size: 0,
            crc: crate::reader::CRC32.digest(),
            crc_value: 0,
        }
    }

    pub fn read_count(&self) -> usize {
        self.size
    }
    pub fn crc_value(&self) -> u32 {
        self.crc_value
    }
}

pub(crate) struct LazyFileReader {
    path: PathBuf,
    reader: Option<File>,
    end: bool,
}

impl LazyFileReader {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            reader: None,
            end: false,
        }
    }
}

impl Read for LazyFileReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.end {
            return Ok(0);
        }
        if self.reader.is_none() {
            self.reader = Some(File::open(&self.path)?);
        }
        let n = self.reader.as_mut().unwrap().read(buf)?;
        if n == 0 {
            self.end = true;
            self.reader = None;
        }
        Ok(n)
    }
}
