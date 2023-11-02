use super::{
    encoder_fast::FashEncoderMode,
    encoder_normal::NormalEncoderMode,
    lz::{LZEncoder, MFType},
    range_enc::{RangeEncoder, RangeEncoderBuffer},
    *,
};
use std::{
    io::Write,
    ops::{Deref, DerefMut},
    vec,
};

const LZMA2_UNCOMPRESSED_LIMIT: u32 = (2 << 20) - MATCH_LEN_MAX as u32;
const LZMA2_COMPRESSED_LIMIT: u32 = (64 << 10) - 26;

const DIST_PRICE_UPDATE_INTERVAL: u32 = FULL_DISTANCES as u32;
const ALIGN_PRICE_UPDATE_INTERVAL: u32 = ALIGN_SIZE as u32;
const PRICE_UPDATE_INTERVAL: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeMode {
    Fast,
    Normal,
}
pub trait LZMAEncoderTrait {
    fn get_next_symbol(&mut self, encoder: &mut LZMAEncoder) -> u32;
    fn reset(&mut self) {}
}

pub enum LZMAEncoderModes {
    Fast(FashEncoderMode),
    Normal(NormalEncoderMode),
}
impl LZMAEncoderTrait for LZMAEncoderModes {
    fn get_next_symbol(&mut self, encoder: &mut LZMAEncoder) -> u32 {
        match self {
            LZMAEncoderModes::Fast(a) => a.get_next_symbol(encoder),
            LZMAEncoderModes::Normal(a) => a.get_next_symbol(encoder),
        }
    }

    fn reset(&mut self) {
        match self {
            LZMAEncoderModes::Fast(a) => a.reset(),
            LZMAEncoderModes::Normal(a) => a.reset(),
        }
    }
}

pub struct LZMAEncoder {
    coder: LZMACoder,
    pub(crate) lz: LZEncoder,
    pub(super) literal_encoder: LiteralEncoder,
    pub(super) match_len_encoder: LengthEncoder,
    pub(super) rep_len_encoder: LengthEncoder,
    pub(super) data: LZMAEncData,
}

pub(super) struct LZMAEncData {
    pub(super) nice_len: usize,
    dist_price_count: i32,
    align_price_count: i32,
    dist_slot_prices_size: u32,
    dist_slot_prices: Vec<Vec<u32>>,
    full_dist_prices: [[u32; FULL_DISTANCES]; DIST_STATES],
    align_prices: [u32; ALIGN_SIZE],
    pub(super) back: i32,
    pub(super) read_ahead: i32,
    pub(super) uncompressed_size: u32,
}

impl LZMAEncoder {
    pub fn get_dist_slot(dist: u32) -> u32 {
        if dist <= DIST_MODEL_START as u32 {
            return dist;
        }
        let mut n = dist;
        let mut i = 31;

        if (n & 0xFFFF0000) == 0 {
            n <<= 16;
            i = 15;
        }

        if (n & 0xFF000000) == 0 {
            n <<= 8;
            i -= 8;
        }

        if (n & 0xF0000000) == 0 {
            n <<= 4;
            i -= 4;
        }

        if (n & 0xC0000000) == 0 {
            n <<= 2;
            i -= 2;
        }

        if (n & 0x80000000) == 0 {
            i -= 1;
        }

        return (i << 1) + ((dist >> (i - 1)) & 1);
    }

    pub fn get_mem_usage(
        mode: EncodeMode,
        dict_size: u32,
        extra_size_before: u32,
        mf: MFType,
    ) -> u32 {
        let mut m = 80;
        match mode {
            EncodeMode::Fast => {
                m += FashEncoderMode::get_memery_usage(dict_size, extra_size_before, mf);
            }
            EncodeMode::Normal => {
                m += NormalEncoderMode::get_memery_usage(dict_size, extra_size_before, mf);
            }
        }
        m
    }
}

impl LZMAEncoder {
    pub fn new(
        mode: EncodeMode,
        lc: u32,
        lp: u32,
        pb: u32,
        mf: MFType,
        depth_limit: i32,
        dict_size: u32,
        nice_len: usize,
    ) -> (Self, LZMAEncoderModes) {
        let fast_mode = mode == EncodeMode::Fast;
        let mut mode: LZMAEncoderModes = if fast_mode {
            LZMAEncoderModes::Fast(FashEncoderMode::default())
        } else {
            LZMAEncoderModes::Normal(NormalEncoderMode::new())
        };
        let (extra_size_before, extra_size_after) = if fast_mode {
            (
                FashEncoderMode::EXTRA_SIZE_BEFORE,
                FashEncoderMode::EXTRA_SIZE_AFTER,
            )
        } else {
            (
                NormalEncoderMode::EXTRA_SIZE_BEFORE,
                NormalEncoderMode::EXTRA_SIZE_AFTER,
            )
        };
        let lz = match mf {
            MFType::HC4 => LZEncoder::new_hc4(
                dict_size,
                extra_size_before,
                extra_size_after,
                nice_len as _,
                MATCH_LEN_MAX as _,
                depth_limit,
            ),
            MFType::BT4 => LZEncoder::new_bt4(
                dict_size,
                extra_size_before,
                extra_size_after,
                nice_len as _,
                MATCH_LEN_MAX as _,
                depth_limit,
            ),
        };

        let literal_encoder = LiteralEncoder::new(lc, lp);
        let match_len_encoder = LengthEncoder::new(pb, nice_len as usize);
        let rep_len_encoder = LengthEncoder::new(pb, nice_len as usize);
        let dist_slot_price_size = LZMAEncoder::get_dist_slot(dict_size - 1) + 1;
        let mut e = Self {
            coder: LZMACoder::new(pb as usize),
            lz,
            literal_encoder,
            match_len_encoder,
            rep_len_encoder,
            data: LZMAEncData {
                nice_len,
                dist_price_count: 0,
                align_price_count: 0,
                dist_slot_prices_size: dist_slot_price_size,
                dist_slot_prices: vec![vec![0; dist_slot_price_size as usize]; DIST_STATES],
                full_dist_prices: [[0; FULL_DISTANCES]; DIST_STATES],
                align_prices: Default::default(),
                back: 0,
                read_ahead: -1,
                uncompressed_size: 0,
            },
        };
        e.reset(&mut mode);

        (e, mode)
    }

    pub fn reset(&mut self, mode: &mut dyn LZMAEncoderTrait) {
        self.coder.reset();
        self.literal_encoder.reset();
        self.match_len_encoder.reset();
        self.rep_len_encoder.reset();
        self.data.dist_price_count = 0;
        self.data.align_price_count = 0;
        self.data.uncompressed_size += (self.data.read_ahead + 1) as u32;
        self.data.read_ahead = -1;
        mode.reset();
    }

    #[inline(always)]
    pub fn reset_uncompressed_size(&mut self) {
        self.data.uncompressed_size = 0;
    }

    #[allow(unused)]
    pub fn encode_for_lzma1<W: Write>(
        &mut self,
        rc: &mut RangeEncoder<W>,
        mode: &mut dyn LZMAEncoderTrait,
    ) -> std::io::Result<()> {
        if !self.lz.is_started() && !self.encode_init(rc)? {
            return Ok(());
        }
        while self.encode_symbol(rc, mode)? {}
        Ok(())
    }

    #[allow(unused)]
    pub fn encode_lzma1_end_marker<W: Write>(
        &mut self,
        rc: &mut RangeEncoder<W>,
    ) -> std::io::Result<()> {
        let pos_state = (self.lz.get_pos() - self.data.read_ahead) as u32 & self.coder.pos_mask;
        rc.encode_bit(
            &mut self.coder.is_match[self.coder.state.get() as usize],
            pos_state as usize,
            1,
        )?;
        rc.encode_bit(&mut self.coder.is_rep, self.coder.state.get() as usize, 0)?;
        self.encode_match(u32::MAX, MATCH_LEN_MIN as u32, pos_state, rc)?;
        Ok(())
    }

    fn encode_init<W: Write>(&mut self, rc: &mut RangeEncoder<W>) -> std::io::Result<bool> {
        assert!(self.data.read_ahead == -1);
        if !self.lz.has_enough_data(0) {
            return Ok(false);
        }
        self.skip(1);
        let state = self.state.get() as usize;
        rc.encode_bit(&mut self.is_match[state], 0, 0)?;
        self.literal_encoder
            .encode_init(&self.lz, &self.data, &mut self.coder, rc)?;
        self.data.read_ahead -= 1;
        assert!(self.data.read_ahead == -1);
        self.data.uncompressed_size += 1;
        assert!(self.data.uncompressed_size == 1);
        Ok(true)
    }

    fn encode_symbol<W: Write>(
        &mut self,
        rc: &mut RangeEncoder<W>,
        mode: &mut dyn LZMAEncoderTrait,
    ) -> std::io::Result<bool> {
        if !self.lz.has_enough_data(self.data.read_ahead + 1) {
            return Ok(false);
        }
        let len = mode.get_next_symbol(self);

        assert!(self.data.read_ahead >= 0);
        let pos_state = (self.lz.get_pos() - self.data.read_ahead) as u32 & self.pos_mask;

        if self.data.back == -1 {
            assert!(len == 1);
            let state = self.state.get() as usize;
            rc.encode_bit(&mut self.is_match[state], pos_state as _, 0)?;
            self.literal_encoder
                .encode(&self.lz, &self.data, &mut self.coder, rc)?;
        } else {
            let state = self.state.get() as usize;
            rc.encode_bit(&mut self.is_match[state], pos_state as usize, 1)?;
            if self.data.back < REPS as i32 {
                let match_len2 = self.lz.get_match_len2(
                    -self.data.read_ahead,
                    self.reps[self.data.back as usize],
                    len as i32,
                );

                let start = (self.lz.read_pos - 20).max(0) as usize;
                let end = (self.lz.read_pos as usize + 20).min(self.lz.buf.len() - 1);
                assert_eq!(
                    match_len2,
                    len,
                    "read_ahead={},back={},read_pos={}, buf[{:?}]={:?}",
                    self.data.read_ahead,
                    self.data.back,
                    self.lz.read_pos,
                    start..end,
                    &self.lz.buf[start..end]
                );
                let state = self.state.get() as usize;
                rc.encode_bit(&mut self.is_rep, state, 1)?;
                self.encode_rep_match(self.data.back as u32, len, pos_state, rc)?;
            } else {
                let match_len2 = self.lz.get_match_len2(
                    -self.data.read_ahead,
                    self.data.back - REPS as i32,
                    len as i32,
                );
                assert_eq!(match_len2, len);
                let state = self.state.get() as usize;
                rc.encode_bit(&mut self.is_rep, state, 0)?;
                self.encode_match((self.data.back - REPS as i32) as u32, len, pos_state, rc)?;
            }
        }
        self.data.read_ahead -= len as i32;
        self.data.uncompressed_size += len;
        Ok(true)
    }

    fn encode_match<W: Write>(
        &mut self,
        dist: u32,
        len: u32,
        pos_state: u32,
        rc: &mut RangeEncoder<W>,
    ) -> std::io::Result<()> {
        self.state.update_match();
        self.match_len_encoder.encode(len, pos_state, rc)?;
        let dist_slot = LZMAEncoder::get_dist_slot(dist);
        rc.encode_bit_tree(
            &mut self.dist_slots[get_dist_state(len) as usize],
            dist_slot,
        )?;

        if dist_slot as usize >= DIST_MODEL_START {
            let footer_bits = (dist_slot >> 1).wrapping_sub(1);
            let base = (2 | (dist_slot & 1)) << footer_bits;
            let dist_reduced = dist - base;

            if dist_slot < DIST_MODEL_END as u32 {
                rc.encode_reverse_bit_tree(
                    &mut self.get_dist_special(dist_slot as usize - DIST_MODEL_START),
                    dist_reduced,
                )?;
            } else {
                rc.encode_direct_bits(dist_reduced >> ALIGN_BITS, footer_bits - ALIGN_BITS as u32)?;
                rc.encode_reverse_bit_tree(&mut self.dist_align, dist_reduced & ALIGN_MASK as u32)?;
                self.data.align_price_count = self.data.align_price_count - 1;
            }
        }

        self.reps[3] = self.reps[2];
        self.reps[2] = self.reps[1];
        self.reps[1] = self.reps[0];
        self.reps[0] = dist as i32;

        self.data.dist_price_count = self.data.dist_price_count - 1;
        Ok(())
    }

    fn encode_rep_match<W: Write>(
        &mut self,
        rep: u32,
        len: u32,
        pos_state: u32,
        rc: &mut RangeEncoder<W>,
    ) -> std::io::Result<()> {
        if rep == 0 {
            let state = self.state.get() as usize;
            rc.encode_bit(&mut self.is_rep0, state as usize, 0)?;
            let state = self.state.get() as usize;
            rc.encode_bit(
                &mut self.is_rep0_long[state],
                pos_state as usize,
                if len == 1 { 0 } else { 1 },
            )?;
        } else {
            let dist = self.reps[rep as usize];
            let state = self.state.get() as usize;

            rc.encode_bit(&mut self.is_rep0, state, 1)?;

            if rep == 1 {
                let state = self.state.get() as usize;
                rc.encode_bit(&mut self.is_rep1, state, 0)?;
            } else {
                let state = self.state.get() as usize;
                rc.encode_bit(&mut self.is_rep1, state, 1)?;
                let state = self.state.get() as usize;
                rc.encode_bit(&mut self.is_rep2, state, rep - 2)?;

                if rep == 3 {
                    self.reps[3] = self.reps[2];
                }
                self.reps[2] = self.reps[1];
            }

            self.reps[1] = self.reps[0];
            self.reps[0] = dist;
        }

        if len == 1 {
            self.state.update_short_rep();
        } else {
            self.rep_len_encoder.encode(len, pos_state, rc)?;
            self.state.update_long_rep();
        }
        Ok(())
    }

    pub(super) fn find_matches(&mut self) {
        self.data.read_ahead += 1;
        self.lz.find_matches();
        assert!(self.lz.verify_matches());
    }

    pub(super) fn skip(&mut self, len: usize) {
        self.data.read_ahead += len as i32;
        self.lz.skip(len)
    }

    pub(super) fn get_any_match_price(&self, state: &State, pos_state: u32) -> u32 {
        RangeEncoder::get_bit_price(
            self.is_match[state.get() as usize][pos_state as usize] as _,
            1,
        )
    }

    pub(super) fn get_normal_match_price(&self, any_match_price: u32, state: &State) -> u32 {
        any_match_price + RangeEncoder::get_bit_price(self.is_rep[state.get() as usize] as _, 0)
    }

    pub(super) fn get_any_rep_price(&self, any_match_price: u32, state: &State) -> u32 {
        any_match_price + RangeEncoder::get_bit_price(self.is_rep[state.get() as usize] as _, 1)
    }

    pub(super) fn get_short_rep_price(
        &self,
        any_rep_price: u32,
        state: &State,
        pos_state: u32,
    ) -> u32 {
        any_rep_price
            + RangeEncoder::get_bit_price(self.is_rep0[state.get() as usize] as _, 0)
            + RangeEncoder::get_bit_price(
                self.is_rep0_long[state.get() as usize][pos_state as usize] as _,
                0,
            )
    }

    pub(super) fn get_long_rep_price(
        &self,
        any_rep_price: u32,
        rep: u32,
        state: &State,
        pos_state: u32,
    ) -> u32 {
        let mut price = any_rep_price;

        if rep == 0 {
            price += RangeEncoder::get_bit_price(self.is_rep0[state.get() as usize] as _, 0)
                + RangeEncoder::get_bit_price(
                    self.is_rep0_long[state.get() as usize][pos_state as usize] as _,
                    1,
                );
        } else {
            price += RangeEncoder::get_bit_price(self.is_rep0[state.get() as usize] as _, 1);

            if rep == 1 {
                price += RangeEncoder::get_bit_price(self.is_rep1[state.get() as usize] as _, 0)
            } else {
                price += RangeEncoder::get_bit_price(self.is_rep1[state.get() as usize] as _, 1)
                    + RangeEncoder::get_bit_price(
                        self.is_rep2[state.get() as usize] as _,
                        rep as i32 - 2,
                    );
            }
        }

        price
    }

    pub(super) fn get_long_rep_and_len_price(
        &self,
        rep: u32,
        len: u32,
        state: &State,
        pos_state: u32,
    ) -> u32 {
        let any_match_price = self.get_any_match_price(state, pos_state);
        let any_rep_price = self.get_any_rep_price(any_match_price, state);
        let long_rep_price = self.get_long_rep_price(any_rep_price, rep, state, pos_state);
        return long_rep_price + self.rep_len_encoder.get_price(len as _, pos_state as _);
    }

    pub(super) fn get_match_and_len_price(
        &self,
        normal_match_price: u32,
        dist: u32,
        len: u32,
        pos_state: u32,
    ) -> u32 {
        let mut price =
            normal_match_price + self.match_len_encoder.get_price(len as _, pos_state as _);
        let dist_state = get_dist_state(len);

        if dist < FULL_DISTANCES as u32 {
            price += self.data.full_dist_prices[dist_state as usize][dist as usize];
        } else {
            // Note that distSlotPrices includes also
            // the price of direct bits.
            let dist_slot = LZMAEncoder::get_dist_slot(dist);
            price += self.data.dist_slot_prices[dist_state as usize][dist_slot as usize]
                + self.data.align_prices[(dist & ALIGN_MASK as u32) as usize];
        }

        return price;
    }

    pub(super) fn update_dist_prices(&mut self) {
        self.data.dist_price_count = DIST_PRICE_UPDATE_INTERVAL as _;

        for dist_state in 0..DIST_STATES {
            for dist_slot in 0..self.data.dist_slot_prices_size as usize {
                self.data.dist_slot_prices[dist_state][dist_slot] =
                    RangeEncoder::get_bit_tree_price(
                        &mut self.dist_slots[dist_state],
                        dist_slot as u32,
                    );
            }

            for dist_slot in DIST_MODEL_END as u32..self.data.dist_slot_prices_size {
                let count = (dist_slot >> 1) - 1 - ALIGN_BITS as u32;
                self.data.dist_slot_prices[dist_state][dist_slot as usize] +=
                    RangeEncoder::get_direct_bits_price(count);
            }

            for dist in 0..DIST_MODEL_START {
                self.data.full_dist_prices[dist_state][dist] =
                    self.data.dist_slot_prices[dist_state][dist];
            }
        }

        let mut dist = DIST_MODEL_START;
        for dist_slot in DIST_MODEL_START..DIST_MODEL_END {
            let footer_bits = (dist_slot >> 1) - 1;
            let base = (2 | (dist_slot & 1)) << footer_bits;

            let limit = self.get_dist_special(dist_slot - DIST_MODEL_START).len();
            for _i in 0..limit {
                let dist_reduced = dist - base;
                let price = RangeEncoder::get_reverse_bit_tree_price(
                    self.get_dist_special(dist_slot - DIST_MODEL_START),
                    dist_reduced as u32,
                );

                for dist_state in 0..DIST_STATES {
                    self.data.full_dist_prices[dist_state][dist] =
                        self.data.dist_slot_prices[dist_state][dist_slot] + price;
                }
                dist += 1;
            }
        }

        assert!(dist == FULL_DISTANCES);
    }
    fn update_align_prices(&mut self) {
        self.data.align_price_count = ALIGN_PRICE_UPDATE_INTERVAL as i32;

        for i in 0..ALIGN_SIZE {
            self.data.align_prices[i] =
                RangeEncoder::get_reverse_bit_tree_price(&mut self.dist_align, i as u32);
        }
    }

    pub(super) fn update_prices(&mut self) {
        if self.data.dist_price_count <= 0 {
            self.update_dist_prices();
        }

        if self.data.align_price_count <= 0 {
            self.update_align_prices();
        }
        self.match_len_encoder.update_prices();
        self.rep_len_encoder.update_prices();
    }
}

impl LZMAEncoder {
    pub fn encode_for_lzma2(
        &mut self,
        rc: &mut RangeEncoder<RangeEncoderBuffer>,
        mode: &mut dyn LZMAEncoderTrait,
    ) -> std::io::Result<bool> {
        if !self.lz.is_started() && !self.encode_init(rc)? {
            return Ok(false);
        }
        while self.data.uncompressed_size <= LZMA2_UNCOMPRESSED_LIMIT
            && rc.get_pending_size() <= LZMA2_COMPRESSED_LIMIT
        {
            if !self.encode_symbol(rc, mode)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl Deref for LZMAEncoder {
    type Target = LZMACoder;

    fn deref(&self) -> &Self::Target {
        &self.coder
    }
}

impl DerefMut for LZMAEncoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.coder
    }
}

pub(super) struct LiteralEncoder {
    coder: LiteralCoder,
    subencoders: Vec<LiteralSubencoder>,
}
#[derive(Clone)]
struct LiteralSubencoder {
    coder: LiteralSubcoder,
}

impl LiteralEncoder {
    pub fn new(lc: u32, lp: u32) -> Self {
        Self {
            coder: LiteralCoder::new(lc, lp),
            subencoders: vec![LiteralSubencoder::new(); 1 << (lc + lp)],
        }
    }

    pub fn reset(&mut self) {
        for ele in self.subencoders.iter_mut() {
            ele.reset();
        }
    }

    pub(super) fn encode_init<W: Write>(
        &mut self,
        lz: &LZEncoder,
        data: &LZMAEncData,
        coder: &mut LZMACoder,
        rc: &mut RangeEncoder<W>,
    ) -> std::io::Result<()> {
        assert!(data.read_ahead >= 0);
        self.subencoders[0].encode(lz, data, coder, rc)
    }

    pub(super) fn encode<W: Write>(
        &mut self,
        lz: &LZEncoder,
        data: &LZMAEncData,
        coder: &mut LZMACoder,
        rc: &mut RangeEncoder<W>,
    ) -> std::io::Result<()> {
        assert!(data.read_ahead >= 0);
        let i = self.coder.get_sub_coder_index(
            lz.get_byte_backward(1 + data.read_ahead) as _,
            (lz.get_pos() - data.read_ahead) as u32,
        );
        self.subencoders[i as usize].encode(lz, data, coder, rc)
    }

    pub(super) fn get_price(
        &self,
        encoder: &LZMAEncoder,
        cur_byte: u32,
        match_byte: u32,
        prev_byte: u32,
        pos: u32,
        state: &State,
    ) -> u32 {
        let mut price = RangeEncoder::get_bit_price(
            encoder.is_match[state.get() as usize][(pos & encoder.pos_mask) as usize] as _,
            0,
        );
        let i = self.coder.get_sub_coder_index(prev_byte, pos) as usize;
        price += if state.is_literal() {
            self.subencoders[i].get_normal_price(cur_byte)
        } else {
            self.subencoders[i].get_matched_price(cur_byte, match_byte)
        };
        price
    }
}

impl LiteralSubencoder {
    fn new() -> Self {
        Self {
            coder: LiteralSubcoder::new(),
        }
    }

    fn reset(&mut self) {
        self.coder.reset()
    }

    fn encode<W: Write>(
        &mut self,
        lz: &LZEncoder,
        data: &LZMAEncData,
        coder: &mut LZMACoder,
        rc: &mut RangeEncoder<W>,
    ) -> std::io::Result<()> {
        let mut symbol = (lz.get_byte_backward(data.read_ahead) as u32 | 0x100) as u32;

        if coder.state.is_literal() {
            let mut subencoder_index;
            let mut bit;

            loop {
                subencoder_index = symbol >> 8;
                bit = (symbol >> 7) & 1;
                rc.encode_bit(&mut self.coder.probs, subencoder_index as _, bit as _)?;
                symbol <<= 1;
                if symbol >= 0x10000 {
                    break;
                }
            }
        } else {
            let mut match_byte = lz.get_byte_backward(coder.reps[0] + 1 + data.read_ahead) as u32;
            let mut offset = 0x100;
            let mut subencoder_index;
            let mut match_bit;
            let mut bit;

            loop {
                match_byte = match_byte << 1;
                match_bit = match_byte & offset;
                subencoder_index = offset + match_bit + (symbol >> 8);
                bit = (symbol >> 7) & 1;
                rc.encode_bit(&mut self.coder.probs, subencoder_index as _, bit)?;
                symbol <<= 1;
                offset = offset & (!(match_byte ^ symbol));
                if symbol >= 0x10000 {
                    break;
                }
            }
        }

        coder.state.update_literal();
        Ok(())
    }

    fn get_normal_price(&self, symbol: u32) -> u32 {
        let mut price: u32 = 0;
        let mut subencoder_index;
        let mut bit;
        let mut symbol = symbol | 0x100;
        loop {
            subencoder_index = symbol >> 8;
            bit = (symbol >> 7) & 1;
            price += RangeEncoder::get_bit_price(
                self.coder.probs[subencoder_index as usize] as _,
                bit as _,
            );
            symbol <<= 1;
            if symbol >= (0x100 << 8) {
                break;
            }
        }
        price
    }

    fn get_matched_price(&self, symbol: u32, mut match_byte: u32) -> u32 {
        let mut price = 0;
        let mut offset = 0x100;
        let mut subencoder_index;
        let mut match_bit;
        let mut bit;
        let mut symbol = symbol | 0x100;
        loop {
            match_byte <<= 1;
            match_bit = match_byte & offset;
            subencoder_index = offset + match_bit + (symbol >> 8);
            bit = (symbol >> 7) & 1;
            price += RangeEncoder::get_bit_price(
                self.coder.probs[subencoder_index as usize] as _,
                bit as _,
            );
            symbol <<= 1;
            offset &= !(match_byte ^ symbol);
            if symbol >= (0x100 << 8) {
                break;
            }
        }
        return price;
    }
}

pub(super) struct LengthEncoder {
    coder: LengthCoder,
    counters: Vec<i32>,
    prices: Vec<Vec<u32>>,
}

impl LengthEncoder {
    pub fn new(pb: u32, nice_len: usize) -> Self {
        let pos_states = 1usize << pb;
        let counters = vec![0; pos_states];
        let len_symbols = (nice_len - MATCH_LEN_MIN + 1).max(LOW_SYMBOLS + MID_SYMBOLS);
        let prices = vec![vec![0; len_symbols]; pos_states];
        Self {
            coder: LengthCoder::new(),
            counters,
            prices,
        }
    }

    fn reset(&mut self) {
        self.coder.reset();
        self.counters.fill(0);
    }

    fn encode<W: Write>(
        &mut self,
        len: u32,
        pos_state: u32,
        rc: &mut RangeEncoder<W>,
    ) -> std::io::Result<()> {
        let mut len = len as usize - MATCH_LEN_MIN;
        if len < LOW_SYMBOLS {
            rc.encode_bit(&mut self.coder.choice, 0, 0)?;
            rc.encode_bit_tree(&mut self.coder.low[pos_state as usize], len as _)?;
        } else {
            rc.encode_bit(&mut self.coder.choice, 0, 1)?;
            len -= LOW_SYMBOLS;
            if len < MID_SYMBOLS {
                rc.encode_bit(&mut self.coder.choice, 1, 0)?;
                rc.encode_bit_tree(&mut self.coder.mid[pos_state as usize], len as _)?;
            } else {
                rc.encode_bit(&mut self.coder.choice, 1, 1)?;
                rc.encode_bit_tree(&mut self.coder.high, (len - MID_SYMBOLS) as _)?;
            }
        }
        self.counters[pos_state as usize] = self.counters[pos_state as usize].wrapping_sub(1);
        Ok(())
    }

    pub fn get_price(&self, len: usize, pos_state: usize) -> u32 {
        self.prices[pos_state][len - MATCH_LEN_MIN]
    }

    fn update_prices(&mut self) {
        for pos_state in 0..self.counters.len() {
            if self.counters[pos_state] <= 0 {
                self.counters[pos_state] = PRICE_UPDATE_INTERVAL as _;
                self.update_prices_with_state(pos_state);
            }
        }
    }

    fn update_prices_with_state(&mut self, pos_state: usize) {
        let mut choice0_price = RangeEncoder::get_bit_price(self.coder.choice[0] as _, 0);
        let mut start = 0;
        for i in start..LOW_SYMBOLS {
            self.prices[pos_state][i] = choice0_price
                + RangeEncoder::get_bit_tree_price(&mut self.coder.low[pos_state], i as _);
        }
        start = LOW_SYMBOLS;
        choice0_price = RangeEncoder::get_bit_price(self.coder.choice[0] as _, 1);
        let mut choice1_price = RangeEncoder::get_bit_price(self.coder.choice[1] as _, 0);
        for i in start..(LOW_SYMBOLS + MID_SYMBOLS) {
            self.prices[pos_state][i] = choice0_price
                + choice1_price
                + RangeEncoder::get_bit_tree_price(
                    &mut self.coder.mid[pos_state],
                    (i - start) as u32,
                );
        }
        start = LOW_SYMBOLS + MID_SYMBOLS;
        choice1_price = RangeEncoder::get_bit_price(self.coder.choice[1] as _, 1);
        for i in start..self.prices[pos_state].len() {
            self.prices[pos_state][i] = choice0_price
                + choice1_price
                + RangeEncoder::get_bit_tree_price(&mut self.coder.high, (i - start) as u32)
        }
    }
    //
}
