use crate::range_dec::RangeSource;

use super::lz::LZDecoder;
use super::range_dec::RangeDecoder;
use super::*;

use std::{
    io::{Read, Result},
    ops::{Deref, DerefMut},
};

pub struct LZMADecoder {
    coder: LZMACoder,
    literal_decoder: LiteralDecoder,
    match_len_decoder: LengthCoder,
    rep_len_decoder: LengthCoder,
}

impl Deref for LZMADecoder {
    type Target = LZMACoder;

    fn deref(&self) -> &Self::Target {
        &self.coder
    }
}
impl DerefMut for LZMADecoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.coder
    }
}

impl LZMADecoder {
    pub fn new(lc: u32, lp: u32, pb: u32) -> Self {
        let mut literal_decoder = LiteralDecoder::new(lc, lp);
        literal_decoder.reset();
        let match_len_decoder = {
            let mut l = LengthCoder::new();
            l.reset();
            l
        };
        let rep_len_decoder = {
            let mut l = LengthCoder::new();
            l.reset();
            l
        };
        Self {
            coder: LZMACoder::new(pb as _),
            literal_decoder,
            match_len_decoder,
            rep_len_decoder,
        }
    }

    pub fn reset(&mut self) {
        self.coder.reset();
        self.literal_decoder.reset();
        self.match_len_decoder.reset();
        self.rep_len_decoder.reset();
    }

    pub fn end_marker_detected(&self) -> bool {
        self.reps[0] == -1
    }

    pub fn decode<R: RangeSource>(&mut self, lz: &mut LZDecoder, rc: &mut RangeDecoder<R>) -> Result<()> {
        lz.repeat_pending()?;
        while lz.has_space() {
            let pos_state = lz.get_pos() as u32 & self.pos_mask;
            let i = self.state.get() as usize;
            let probs = &mut self.is_match[i];
            let bit = rc.decode_bit(&mut probs[pos_state as usize])?;
            if bit == 0 {
                self.literal_decoder.decode(&mut self.coder, lz, rc)?;
            } else {
                let index = self.state.get() as usize;
                let len = if rc.decode_bit(&mut self.is_rep[index])? == 0 {
                    self.decode_match(pos_state, rc)?
                } else {
                    self.decode_rep_match(pos_state, rc)?
                };
                lz.repeat(self.reps[0] as _, len as _)?;
            }
        }
        rc.normalize()?;
        Ok(())
    }

    fn decode_match<R: RangeSource>(&mut self, pos_state: u32, rc: &mut RangeDecoder<R>) -> Result<u32> {
        self.state.update_match();
        self.reps[3] = self.reps[2];
        self.reps[2] = self.reps[1];
        self.reps[1] = self.reps[0];

        let len = self.match_len_decoder.decode(pos_state as _, rc)?;
        let dist_slot = rc.decode_bit_tree(&mut self.dist_slots[coder_get_dict_size(len as _)])?;

        if dist_slot < DIST_MODEL_START as i32 {
            self.reps[0] = dist_slot as _;
        } else {
            let limit = (dist_slot >> 1) - 1;
            self.reps[0] = (2 | (dist_slot & 1)) << limit;
            if dist_slot < DIST_MODEL_END as i32 {
                let probs = self.get_dist_special((dist_slot - DIST_MODEL_START as i32) as usize);
                self.reps[0] |= rc.decode_reverse_bit_tree(probs)?;
            } else {
                let r0 = rc.decode_direct_bits(limit as u32 - ALIGN_BITS as u32)? << ALIGN_BITS;
                self.reps[0] = self.reps[0] | r0;
                self.reps[0] |= rc.decode_reverse_bit_tree(&mut self.dist_align)?;
            }
        }

        Ok(len as _)
    }

    fn decode_rep_match<R: RangeSource>(
        &mut self,
        pos_state: u32,
        rc: &mut RangeDecoder<R>,
    ) -> Result<u32> {
        let index = self.state.get() as usize;
        if rc.decode_bit(&mut self.is_rep0[index])? == 0 {
            let index: usize = self.state.get() as usize;
            if rc.decode_bit(&mut self.is_rep0_long[index][pos_state as usize])? == 0 {
                self.state.update_short_rep();
                return Ok(1);
            }
        } else {
            let tmp;
            let s = self.state.get() as usize;
            if rc.decode_bit(&mut self.is_rep1[s])? == 0 {
                tmp = self.reps[1];
            } else {
                if rc.decode_bit(&mut self.is_rep2[s])? == 0 {
                    tmp = self.reps[2];
                } else {
                    tmp = self.reps[3];
                    self.reps[3] = self.reps[2];
                }
                self.reps[2] = self.reps[1];
            }
            self.reps[1] = self.reps[0];
            self.reps[0] = tmp;
        }

        self.state.update_long_rep();
        self.rep_len_decoder
            .decode(pos_state as _, rc)
            .map(|i| i as u32)
    }
}
pub struct LiteralDecoder {
    coder: LiteralCoder,
    sub_decoders: Vec<LiteralSubdecoder>,
}

impl LiteralDecoder {
    fn new(lc: u32, lp: u32) -> Self {
        let coder = LiteralCoder::new(lc, lp);
        let sub_decoders = vec![LiteralSubdecoder::new(); (1 << (lc + lp)) as _];

        Self {
            coder,
            sub_decoders,
        }
    }

    fn reset(&mut self) {
        for ele in self.sub_decoders.iter_mut() {
            ele.coder.reset()
        }
    }

    fn decode<R: RangeSource>(
        &mut self,
        coder: &mut LZMACoder,
        lz: &mut LZDecoder,
        rc: &mut RangeDecoder<R>,
    ) -> Result<()> {
        let i = self
            .coder
            .get_sub_coder_index(lz.get_byte(0) as _, lz.get_pos() as _);
        let d = &mut self.sub_decoders[i as usize];
        d.decode(coder, lz, rc)
    }
}

#[derive(Clone)]
struct LiteralSubdecoder {
    coder: LiteralSubcoder,
}

impl LiteralSubdecoder {
    fn new() -> Self {
        Self {
            coder: LiteralSubcoder::new(),
        }
    }
    pub fn decode<R: RangeSource>(
        &mut self,
        coder: &mut LZMACoder,
        lz: &mut LZDecoder,
        rc: &mut RangeDecoder<R>,
    ) -> Result<()> {
        let mut symbol: u32 = 1;
        let liter = coder.state.is_literal();
        if liter {
            loop {
                let b = rc.decode_bit(&mut self.coder.probs[symbol as usize])? as u32;
                symbol = (symbol << 1) | b;
                if symbol >= 0x100 {
                    break;
                }
            }
        } else {
            let r = coder.reps[0];
            let mut match_byte = lz.get_byte(r as usize) as u32;
            let mut offset = 0x100;
            let mut match_bit;
            let mut bit;

            loop {
                match_byte = match_byte << 1;
                match_bit = match_byte & offset;
                bit = rc
                    .decode_bit(&mut self.coder.probs[(offset + match_bit + symbol) as usize])?
                    as u32;
                symbol = (symbol << 1) | bit;
                offset &= (0u32.wrapping_sub(bit)) ^ !match_bit;
                if symbol >= 0x100 {
                    break;
                }
            }
        }
        lz.put_byte(symbol as u8);
        coder.state.update_literal();
        Ok(())
    }
}

impl LengthCoder {
    fn decode<R: RangeSource>(&mut self, pos_state: usize, rc: &mut RangeDecoder<R>) -> Result<i32> {
        if rc.decode_bit(&mut self.choice[0])? == 0 {
            return Ok(rc
                .decode_bit_tree(&mut self.low[pos_state])?
                .wrapping_add(MATCH_LEN_MIN as _));
        }

        if rc.decode_bit(&mut self.choice[1])? == 0 {
            return Ok(rc
                .decode_bit_tree(&mut self.mid[pos_state])?
                .wrapping_add(MATCH_LEN_MIN as _)
                .wrapping_add(LOW_SYMBOLS as _));
        }

        let r = rc
            .decode_bit_tree(&mut self.high)?
            .wrapping_add(MATCH_LEN_MIN as _)
            .wrapping_add(LOW_SYMBOLS as _)
            .wrapping_add(MID_SYMBOLS as _);
        Ok(r)
    }
}
