use std::io::{ErrorKind, Read};

#[derive(Default)]
pub struct LZDecoder {
    buf: Vec<u8>,
    buf_size: usize,
    start: usize,
    pos: usize,
    full: usize,
    limit: usize,
    pending_len: usize,
    pending_dist: usize,
}

impl LZDecoder {
    pub fn new(dict_size: usize, preset_dict: Option<&[u8]>) -> Self {
        let mut buf = vec![0; dict_size];
        let mut pos = 0;
        let mut full = 0;
        let mut start = 0;
        if let Some(preset) = preset_dict {
            pos = preset.len().min(dict_size);
            full = pos;
            start = pos;
            let ps = preset.len() - pos;
            buf[0..pos].copy_from_slice(&preset[ps..]);
        }
        Self {
            buf,
            buf_size: dict_size,
            pos,
            full,
            start,
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.start = 0;
        self.pos = 0;
        self.full = 0;
        self.limit = 0;
        self.buf[self.buf_size - 1] = 0;
    }

    pub fn set_limit(&mut self, out_max: usize) {
        self.limit = (out_max + self.pos).min(self.buf_size);
    }

    pub fn has_space(&self) -> bool {
        self.pos < self.limit
    }

    pub fn has_pending(&self) -> bool {
        self.pending_len > 0
    }

    pub fn get_pos(&self) -> usize {
        self.pos
    }

    pub fn get_byte(&self, dist: usize) -> u8 {
        let offset = if dist >= self.pos {
            self.buf_size + self.pos - dist - 1
        } else {
            self.pos - dist - 1
        };
        self.buf[offset]
    }

    pub fn put_byte(&mut self, b: u8) {
        self.buf[self.pos] = b;
        self.pos += 1;
        if self.full < self.pos {
            self.full = self.pos;
        }
    }

    pub fn repeat(&mut self, dist: usize, len: usize) -> std::io::Result<()> {
        if dist >= self.full {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "dist overflow",
            ));
        }
        let mut left = usize::min(self.limit - self.pos, len);
        self.pending_len = len - left;
        self.pending_dist = dist;

        let back = if self.pos < dist + 1 {
            // The distance wraps around to the end of the cyclic dictionary
            // buffer. We cannot get here if the dictionary isn't full.
            assert!(self.full == self.buf_size);
            let mut back = self.buf_size + self.pos - dist - 1;

            // Here we will never copy more than dist + 1 bytes and
            // so the copying won't repeat from its own output.
            // Thus, we can always use std::ptr::copy safely.
            let copy_size = usize::min(self.buf_size - back, left);
            assert!(copy_size <= dist + 1);
            unsafe {
                let buf_ptr = self.buf.as_mut_ptr();
                let src = buf_ptr.add(back);
                let dest = buf_ptr.add(self.pos);
                std::ptr::copy_nonoverlapping(src, dest, copy_size);
            }
            self.pos += copy_size;
            back = 0;
            left -= copy_size;

            if left == 0 {
                return Ok(());
            }
            back
        } else {
            self.pos - dist - 1
        };

        assert!(back < self.pos);
        assert!(left > 0);

        loop {
            let copy_size = left.min(self.pos - back);
            let pos = self.pos;
            unsafe {
                let buf_ptr = self.buf.as_mut_ptr();
                let src = buf_ptr.add(back);
                let dest = buf_ptr.add(pos);
                std::ptr::copy_nonoverlapping(src, dest, copy_size);
            }

            self.pos += copy_size;
            left -= copy_size;
            if left == 0 {
                break;
            }
        }

        if self.full < self.pos {
            self.full = self.pos;
        }
        Ok(())
    }

    pub fn repeat_pending(&mut self) -> std::io::Result<()> {
        if self.pending_len > 0 {
            self.repeat(self.pending_dist, self.pending_len)?;
        }
        Ok(())
    }

    pub fn copy_uncompressed<R: Read>(
        &mut self,
        mut in_data: R,
        len: usize,
    ) -> std::io::Result<()> {
        let copy_size = (self.buf_size - self.pos).min(len);
        let buf = &mut self.buf[self.pos..(self.pos + copy_size)];
        in_data.read_exact(buf)?;
        self.pos += copy_size;
        if self.full < self.pos {
            self.full = self.pos;
        }
        Ok(())
    }

    pub fn flush(&mut self, out: &mut [u8], out_off: usize) -> usize {
        let copy_size = self.pos - self.start;
        if self.pos == self.buf_size {
            self.pos = 0;
        }
        out[out_off..(out_off + copy_size)]
            .copy_from_slice(&self.buf[self.start..(self.start + copy_size)]);

        self.start = self.pos;
        copy_size
    }
}

#[cfg(test)]
mod tests {}
