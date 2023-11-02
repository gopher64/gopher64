use byteorder::{BigEndian, ReadBytesExt};

use super::*;

use std::io::ErrorKind;
use std::io::{Read, Result};

pub trait RangeSource {
    fn next_byte(&mut self) -> Result<u8>;
    fn next_u32(&mut self) -> Result<u32>;
}
impl<T: Read> RangeSource for T {
    fn next_byte(&mut self) -> Result<u8> {
        self.read_u8()
    }
    fn next_u32(&mut self) -> Result<u32> {
        self.read_u32::<BigEndian>()
    }
}

pub struct RangeDecoder<R> {
    inner: R,
    range: u32,
    code: u32,
}
impl RangeDecoder<RangeDecoderBuffer> {
    pub fn new_buffer(len: usize) -> Self {
        Self {
            inner: RangeDecoderBuffer::new(len - 5),
            code: 0,
            range: 0,
        }
    }
}

impl<R: RangeSource> RangeDecoder<R> {
    pub fn new_stream(mut inner: R) -> Result<Self> {
        let b = inner.next_byte()?;
        if b != 0x00 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "range decoder first byte is 0",
            ));
        }
        let code = inner.next_u32()?;
        Ok(Self {
            inner,
            code,
            range: (0xFFFFFFFFu32),
        })
    }

    pub fn is_stream_finished(&self) -> bool {
        self.code == 0
    }
}

impl<R: RangeSource> RangeDecoder<R> {
    pub fn normalize(&mut self) -> Result<()> {
        if self.range < 0x0100_0000 {
            let b = self.inner.next_byte()? as u32;
            let code = ((self.code) << SHIFT_BITS) | b;
            self.code = code;
            let range = (self.range) << SHIFT_BITS;
            self.range = range;
        }
        Ok(())
    }

    pub fn decode_bit(&mut self, prob: &mut u16) -> Result<i32> {
        self.normalize()?;
        let bound = (self.range >> (BIT_MODEL_TOTAL_BITS as i32)) * (*prob as u32);
        // let mask = 0x80000000u32;
        // let cm = self.code ^ mask;
        // let bm = bound ^ mask;
        if self.code < bound {
            self.range = bound;
            *prob += (BIT_MODEL_TOTAL as u16 - *prob) >> (MOVE_BITS as u16);
            Ok(0)
        } else {
            self.range = self.range - (bound);
            self.code = self.code - (bound);
            *prob -= *prob >> (MOVE_BITS as u16);
            Ok(1)
        }
    }

    pub fn decode_bit_tree(&mut self, probs: &mut [u16]) -> Result<i32> {
        let mut symbol = 1;
        loop {
            symbol = (symbol << 1) | self.decode_bit(&mut probs[symbol as usize])?;
            if symbol >= probs.len() as i32 {
                break;
            }
        }
        Ok(symbol - probs.len() as i32)
    }

    pub fn decode_reverse_bit_tree(&mut self, probs: &mut [u16]) -> Result<i32> {
        let mut symbol = 1;
        let mut i = 0;
        let mut result = 0;
        loop {
            let bit = self.decode_bit(&mut probs[symbol as usize])?;
            symbol = (symbol << 1) | bit;
            result |= bit << i;
            i += 1;
            if symbol >= probs.len() as i32 {
                break;
            }
        }
        Ok(result as i32)
    }

    pub fn decode_direct_bits(&mut self, count: u32) -> Result<i32> {
        let mut result = 0;
        for _ in 0..count {
            // }
            // loop {
            self.normalize()?;
            self.range = self.range >> 1;
            let t = (self.code.wrapping_sub(self.range)) >> 31;
            self.code -= self.range & (t.wrapping_sub(1));
            result = (result << 1) | (1u32.wrapping_sub(t));
            // count -= 1;
            // if count == 0 {
            //     break;
            // }
        }
        Ok(result as _)
    }
}

pub struct RangeDecoderBuffer {
    buf: Vec<u8>,
    pos: usize,
}
impl RangeDecoder<RangeDecoderBuffer> {
    pub fn prepare<R: Read>(&mut self, mut reader: R, len: usize) -> Result<()> {
        if len < 5 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "buffer len must >= 5",
            ));
        }

        let b = reader.read_u8()?;
        if b != 0x00 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "first byte is 0",
            ));
        }
        self.code = reader.read_u32::<BigEndian>()?;

        self.range = 0xFFFFFFFFu32;
        let len = len - 5;
        let pos = self.inner.buf.len() - len;
        let end = pos + len;
        self.inner.pos = pos;
        reader.read_exact(&mut self.inner.buf[pos..end])
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
        self.inner.pos == self.inner.buf.len() && self.code == 0
    }
}

impl RangeDecoderBuffer {
    pub fn new(len: usize) -> Self {
        Self {
            buf: vec![0; len],
            pos: len,
        }
    }
}
impl RangeSource for RangeDecoderBuffer {
    fn next_byte(&mut self) -> Result<u8> {
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn next_u32(&mut self) -> Result<u32> {
        let buf = [
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ];
        let b = u32::from_be_bytes(buf);
        self.pos += 4;
        Ok(b)
    }
}
