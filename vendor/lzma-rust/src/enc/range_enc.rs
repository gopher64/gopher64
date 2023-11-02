use std::io::{Result, Write};

use byteorder::WriteBytesExt;

use crate::{BIT_MODEL_TOTAL, BIT_MODEL_TOTAL_BITS, MOVE_BITS, SHIFT_BITS, TOP_MASK};

const MOVE_REDUCING_BITS: usize = 4;
const BIT_PRICE_SHIFT_BITS: usize = 4;

const PRICES: &[u32] = &[
    0x80, 0x67, 0x5b, 0x54, 0x4e, 0x49, 0x45, 0x42, 0x3f, 0x3d, 0x3a, 0x38, 0x36, 0x34, 0x33, 0x31,
    0x30, 0x2e, 0x2d, 0x2c, 0x2b, 0x2a, 0x29, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x22, 0x21,
    0x20, 0x1f, 0x1f, 0x1e, 0x1d, 0x1d, 0x1c, 0x1c, 0x1b, 0x1a, 0x1a, 0x19, 0x19, 0x18, 0x18, 0x17,
    0x17, 0x16, 0x16, 0x16, 0x15, 0x15, 0x14, 0x14, 0x13, 0x13, 0x13, 0x12, 0x12, 0x11, 0x11, 0x11,
    0x10, 0x10, 0x10, 0xf, 0xf, 0xf, 0xe, 0xe, 0xe, 0xd, 0xd, 0xd, 0xc, 0xc, 0xc, 0xb, 0xb, 0xb,
    0xb, 0xa, 0xa, 0xa, 0xa, 0x9, 0x9, 0x9, 0x9, 0x8, 0x8, 0x8, 0x8, 0x7, 0x7, 0x7, 0x7, 0x6, 0x6,
    0x6, 0x6, 0x5, 0x5, 0x5, 0x5, 0x5, 0x4, 0x4, 0x4, 0x4, 0x3, 0x3, 0x3, 0x3, 0x3, 0x2, 0x2, 0x2,
    0x2, 0x2, 0x2, 0x1, 0x1, 0x1, 0x1, 0x1,
];

pub struct RangeEncoder<W> {
    low: u64,
    range: u32,
    cache_size: u32,
    cache: u8,
    inner: W,
}

impl<W: Write> RangeEncoder<W> {
    pub fn new(inner: W) -> Self {
        let mut e = Self {
            low: 0,
            range: 0,
            cache_size: 0,
            cache: 0,
            inner,
        };
        e.reset();
        e
    }

    #[inline]
    pub fn inner(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn reset(&mut self) {
        self.low = 0;
        self.range = 0xFFFFFFFFu32;
        self.cache = 0;
        self.cache_size = 1;
    }

    pub fn finish(&mut self) -> Result<Option<usize>> {
        for _i in 0..5 {
            self.shift_low()?;
        }
        Ok(None)
    }

    fn write_byte(&mut self, b: u8) -> std::io::Result<()> {
        self.inner.write_u8(b)
    }

    fn shift_low(&mut self) -> std::io::Result<()> {
        let low_hi = (self.low >> 32) as i32;
        if low_hi != 0 || self.low < 0xFF000000u64 {
            let mut temp = self.cache;
            loop {
                self.write_byte((temp as i32 + low_hi) as u8)?;
                temp = 0xFF;
                self.cache_size -= 1;
                if self.cache_size == 0 {
                    break;
                }
            }
            self.cache = (self.low >> 24) as u8;
        }
        self.cache_size += 1;
        self.low = (self.low & 0x00FFFFFF) << 8;

        Ok(())
    }

    pub fn encode_bit(&mut self, probs: &mut [u16], index: usize, bit: u32) -> std::io::Result<()> {
        let prob = &mut probs[index];
        let bound = (self.range >> BIT_MODEL_TOTAL_BITS) * (*prob as u32);
        if bit == 0 {
            self.range = bound;
            *prob += ((BIT_MODEL_TOTAL.wrapping_sub(*prob as u32)) >> MOVE_BITS) as u16;
        } else {
            self.low += (bound & 0xFFFFFFFFu32) as u64;
            self.range = self.range.wrapping_sub(bound);
            *prob -= (*prob) >> (MOVE_BITS as u16);
        }
        if self.range & TOP_MASK == 0 {
            self.range = self.range << SHIFT_BITS;
            self.shift_low()?;
        }
        Ok(())
    }

    pub fn encode_bit_tree(&mut self, probs: &mut [u16], symbol: u32) -> std::io::Result<()> {
        let mut index = 1;
        let mut mask = probs.len() as u32;
        loop {
            mask >>= 1;
            let bit = symbol & mask;
            self.encode_bit(probs, index, bit)?;
            index <<= 1;
            if bit != 0 {
                index = index | 1;
            }
            if mask == 1 {
                break;
            }
        }

        Ok(())
    }

    pub fn encode_reverse_bit_tree(
        &mut self,
        probs: &mut [u16],
        symbol: u32,
    ) -> std::io::Result<()> {
        let mut index = 1u32;
        let mut symbol = symbol | probs.len() as u32;
        loop {
            let bit = symbol & 1;
            symbol >>= 1;
            self.encode_bit(probs, index as usize, bit)?;
            index = (index << 1) | bit;
            if symbol == 1 {
                break;
            }
        }
        Ok(())
    }

    pub fn encode_direct_bits(&mut self, value: u32, mut count: u32) -> std::io::Result<()> {
        loop {
            self.range = self.range >> 1;
            count = count - 1;
            let m = 0u32.wrapping_sub((value >> count) & 1);
            self.low += (self.range & m) as u64;

            if self.range & TOP_MASK == 0 {
                self.range = self.range << SHIFT_BITS;
                self.shift_low()?;
            }
            if count == 0 {
                break;
            }
        }
        Ok(())
    }
}

impl RangeEncoder<()> {
    pub fn get_bit_price(prob: u32, bit: i32) -> u32 {
        assert!(bit == 0 || bit == 1);
        let i = (prob ^ ((-bit) as u32 & (BIT_MODEL_TOTAL - 1))) >> MOVE_REDUCING_BITS;
        PRICES[i as usize]
    }
    pub fn get_bit_tree_price(probs: &mut [u16], symbol: u32) -> u32 {
        let mut price = 0;
        let mut symbol = symbol | probs.len() as u32;
        loop {
            let bit = symbol & 1;
            symbol >>= 1;
            price += Self::get_bit_price(probs[symbol as usize] as u32, bit as i32);
            if symbol == 1 {
                break;
            }
        }
        price
    }
    pub fn get_reverse_bit_tree_price(probs: &mut [u16], symbol: u32) -> u32 {
        let mut price = 0;
        let mut index = 1u32;
        let mut symbol = symbol | probs.len() as u32;
        loop {
            let bit = symbol & 1;
            symbol >>= 1;
            price += Self::get_bit_price(probs[index as usize] as u32, bit as i32);
            index = (index << 1) | bit;
            if symbol == 1 {
                break;
            }
        }
        price
    }

    pub fn get_direct_bits_price(count: u32) -> u32 {
        count << BIT_PRICE_SHIFT_BITS
    }
}

impl RangeEncoder<RangeEncoderBuffer> {
    pub fn write_to<W: Write>(&self, out: &mut W) -> Result<()> {
        self.inner.write_to(out)
    }

    pub fn finish_buffer(&mut self) -> Result<Option<usize>> {
        self.finish()?;
        Ok(Some(self.inner.pos))
    }

    pub fn new_buffer(buf_size: usize) -> Self {
        Self::new(RangeEncoderBuffer::new(buf_size))
    }

    pub fn reset_buffer(&mut self) {
        self.reset();
        self.inner.pos = 0;
    }

    #[inline]
    pub fn get_pending_size(&self) -> u32 {
        let w = &self.inner;
        w.pos as u32 + self.cache_size + 5 - 1
    }
}

pub struct RangeEncoderBuffer {
    buf: Vec<u8>,
    pos: usize,
}

impl RangeEncoderBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            buf: vec![0; size],
            pos: 0,
        }
    }
    pub fn write_to<W: Write>(&self, out: &mut W) -> Result<()> {
        out.write_all(&self.buf[..self.pos])
    }
}

impl Write for RangeEncoderBuffer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let size = buf.len().min(self.buf.len() - self.pos);
        if size == 0 {
            return Ok(0);
        }
        self.buf[self.pos..(self.pos + size)].copy_from_slice(&buf[..size]);
        self.pos += size;
        Ok(size)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
