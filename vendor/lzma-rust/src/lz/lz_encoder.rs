use std::{
    io::{Result, Write},
    ops::Deref,
};

use super::{bt4::BT4, hc4::HC4};

pub trait MatchFind {
    fn find_matches(&mut self, encoder: &mut LZEncoderData, matches: &mut Matches);
    fn skip(&mut self, encoder: &mut LZEncoderData, len: usize);
}
pub enum MatchFinders {
    HC4(HC4),
    BT4(BT4),
}

impl MatchFind for MatchFinders {
    fn find_matches(&mut self, encoder: &mut LZEncoderData, matches: &mut Matches) {
        match self {
            MatchFinders::HC4(m) => m.find_matches(encoder, matches),
            MatchFinders::BT4(m) => m.find_matches(encoder, matches),
        }
    }

    fn skip(&mut self, encoder: &mut LZEncoderData, len: usize) {
        match self {
            MatchFinders::HC4(m) => m.skip(encoder, len),
            MatchFinders::BT4(m) => m.skip(encoder, len),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MFType {
    HC4,
    BT4,
}

impl Default for MFType {
    fn default() -> Self {
        Self::HC4
    }
}
impl MFType {
    #[inline]
    fn get_memery_usage(self, dict_size: u32) -> u32 {
        match self {
            MFType::HC4 => HC4::get_mem_usage(dict_size),
            MFType::BT4 => BT4::get_mem_usage(dict_size),
        }
    }
}
pub struct LZEncoder {
    pub(crate) data: LZEncoderData,
    pub(crate) matches: Matches,
    pub(crate) match_finder: MatchFinders,
}

pub struct LZEncoderData {
    pub(crate) keep_size_before: u32,
    pub(crate) keep_size_after: u32,
    pub(crate) match_len_max: u32,
    pub(crate) nice_len: u32,
    pub(crate) buf: Vec<u8>,
    pub(crate) buf_size: u32,
    pub(crate) read_pos: i32,
    pub(crate) read_limit: i32,
    pub(crate) finishing: bool,
    pub(crate) write_pos: i32,
    pub(crate) pending_size: u32,
}

pub struct Matches {
    pub len: Vec<u32>,
    pub dist: Vec<i32>,
    pub count: u32,
}
impl Matches {
    pub fn new(count_max: usize) -> Self {
        Self {
            len: vec![0; count_max],
            dist: vec![0; count_max],
            count: 0,
        }
    }
}

impl LZEncoder {
    pub fn get_memery_usage(
        dict_size: u32,
        extra_size_before: u32,
        extra_size_after: u32,
        match_len_max: u32,
        mf: MFType,
    ) -> u32 {
        let m = get_buf_size(
            dict_size,
            extra_size_before,
            extra_size_after,
            match_len_max,
        ) + mf.get_memery_usage(dict_size);

        m
    }

    pub fn new_hc4(
        dict_size: u32,
        extra_size_before: u32,
        extra_size_after: u32,
        nice_len: u32,
        match_len_max: u32,
        depth_limit: i32,
    ) -> Self {
        Self::new(
            dict_size,
            extra_size_before,
            extra_size_after,
            nice_len,
            match_len_max,
            MatchFinders::HC4(HC4::new(dict_size, nice_len, depth_limit)),
        )
    }

    pub fn new_bt4(
        dict_size: u32,
        extra_size_before: u32,
        extra_size_after: u32,
        nice_len: u32,
        match_len_max: u32,
        depth_limit: i32,
    ) -> Self {
        Self::new(
            dict_size,
            extra_size_before,
            extra_size_after,
            nice_len,
            match_len_max,
            MatchFinders::BT4(BT4::new(dict_size, nice_len, depth_limit)),
        )
    }
    fn new(
        dict_size: u32,
        extra_size_before: u32,
        extra_size_after: u32,
        nice_len: u32,
        match_len_max: u32,
        match_finder: MatchFinders,
    ) -> Self {
        let buf_size = get_buf_size(
            dict_size,
            extra_size_before,
            extra_size_after,
            match_len_max,
        );
        let buf = vec![0; buf_size as usize];

        let keep_size_before = extra_size_before + dict_size;
        let keep_size_after = extra_size_after + match_len_max;
        Self {
            data: LZEncoderData {
                keep_size_before,
                keep_size_after,
                match_len_max,
                nice_len,
                buf,
                buf_size,
                read_pos: -1,
                read_limit: -1,
                finishing: false,
                write_pos: 0,
                pending_size: 0,
            },
            matches: Matches::new(nice_len as usize - 1),
            match_finder,
        }
    }

    pub(super) fn normalize(positions: &mut [i32], norm_offset: i32) {
        for p in positions {
            if *p <= norm_offset {
                *p = 0;
            } else {
                *p = *p - norm_offset;
            }
        }
    }

    pub fn find_matches(&mut self) {
        self.match_finder
            .find_matches(&mut self.data, &mut self.matches)
    }

    pub fn matches(&mut self) -> &mut Matches {
        &mut self.matches
    }

    pub fn skip(&mut self, len: usize) {
        self.match_finder.skip(&mut self.data, len)
    }

    pub fn set_preset_dict(&mut self, dict_size: u32, preset_dict: &[u8]) {
        self.data
            .set_preset_dict(dict_size, preset_dict, &mut self.match_finder)
    }

    pub fn set_finishing(&mut self) {
        self.data.set_finishing(&mut self.match_finder)
    }
    pub fn fill_window(&mut self, input: &[u8]) -> usize {
        self.data.fill_window(input, &mut self.match_finder)
    }
    pub fn set_flushing(&mut self) {
        self.data.set_flushing(&mut self.match_finder)
    }

    pub fn verify_matches(&self) -> bool {
        self.data.verify_matches(&self.matches)
    }
}

impl LZEncoderData {
    pub fn is_started(&self) -> bool {
        self.read_pos != -1
    }

    pub(super) fn buf_mut(&mut self) -> &mut [u8] {
        &mut self.buf[self.read_pos as usize..]
    }
    fn set_preset_dict(
        &mut self,
        dict_size: u32,
        preset_dict: &[u8],
        match_finder: &mut dyn MatchFind,
    ) {
        assert!(!self.is_started());
        assert!(self.write_pos == 0);
        let copy_size = preset_dict.len().min(dict_size as usize);
        let offset = preset_dict.len() - copy_size;
        self.buf[0..copy_size].copy_from_slice(&preset_dict[offset..(offset + copy_size)]);
        self.write_pos += copy_size as i32;
        match_finder.skip(self, copy_size);
    }

    fn move_window(&mut self) {
        let move_offset = (self.read_pos + 1 - self.keep_size_before as i32) & !15;
        let move_size = self.write_pos as i32 - move_offset;
        assert!(move_size >= 0);
        assert!(move_offset >= 0);
        let move_size = move_size as usize;
        let offset = move_offset as usize;
        let end = offset + move_size;
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.buf[offset..end].as_ptr(),
                self.buf[0..].as_mut_ptr(),
                move_size,
            );
        }
        self.read_pos -= move_offset;
        self.read_limit -= move_offset;
        self.write_pos -= move_offset;
    }

    fn fill_window(&mut self, input: &[u8], match_finder: &mut dyn MatchFind) -> usize {
        assert!(!self.finishing);
        if self.read_pos >= (self.buf_size as i32 - self.keep_size_after as i32) {
            self.move_window();
        }
        let len = if input.len() as i32 > self.buf_size as i32 - self.write_pos {
            (self.buf_size as i32 - self.write_pos) as usize
        } else {
            input.len()
        };
        let d_start = self.write_pos as usize;
        let d_end = d_start + len;
        self.buf[d_start..d_end].copy_from_slice(&input[..len]);
        self.write_pos += len as i32;
        if self.write_pos >= self.keep_size_after as i32 {
            self.read_limit = self.write_pos - self.keep_size_after as i32;
        }
        self.process_pending_bytes(match_finder);
        len
    }

    fn process_pending_bytes(&mut self, match_finder: &mut dyn MatchFind) {
        if self.pending_size > 0 && self.read_pos < self.read_limit {
            self.read_pos -= self.pending_size as i32;
            let old_pending = self.pending_size;
            self.pending_size = 0;
            match_finder.skip(self, old_pending as _);
            assert!(self.pending_size < old_pending)
        }
    }

    fn set_flushing(&mut self, match_finder: &mut dyn MatchFind) {
        self.read_limit = self.write_pos - 1;
        self.process_pending_bytes(match_finder);
    }
    fn set_finishing(&mut self, match_finder: &mut dyn MatchFind) {
        self.read_limit = self.write_pos - 1;
        self.finishing = true;
        self.process_pending_bytes(match_finder);
    }

    pub fn has_enough_data(&self, already_read_len: i32) -> bool {
        self.read_pos - already_read_len < self.read_limit
    }
    pub fn copy_uncompressed<W: Write>(
        &self,
        out: &mut W,
        backward: i32,
        len: usize,
    ) -> Result<()> {
        let start = (self.read_pos + 1 - backward) as usize;
        out.write_all(&self.buf[start..(start + len)])
    }

    pub fn get_avail(&self) -> i32 {
        assert_ne!(self.read_pos, -1);
        self.write_pos - self.read_pos
    }

    pub fn get_pos(&self) -> i32 {
        self.read_pos
    }

    pub fn get_byte(&self, forward: i32, backward: i32) -> u8 {
        let start = self.read_pos + forward - backward;
        self.buf[start as usize]
    }

    pub fn get_byte_backward(&self, backward: i32) -> u8 {
        self.buf[(self.read_pos - backward) as usize]
    }

    pub fn get_current_byte(&self) -> u8 {
        self.buf[self.read_pos as usize]
    }

    pub fn get_match_len(&self, dist: i32, len_limit: i32) -> usize {
        let back_pos = self.read_pos - dist - 1;
        let mut len = 0;

        while len < len_limit
            && self.buf[(self.read_pos + len) as usize] == self.buf[(back_pos + len) as usize]
        {
            len += 1;
        }

        len as usize
    }

    pub fn get_match_len2(&self, forward: i32, dist: i32, len_limit: i32) -> u32 {
        let cur_pos = (self.read_pos + forward) as usize;
        let back_pos = cur_pos - dist as usize - 1;
        let mut len = 0;

        while len < len_limit
            && self.buf[cur_pos + len as usize] == self.buf[back_pos + len as usize]
        {
            len += 1;
        }
        return len as _;
    }

    fn verify_matches(&self, matches: &Matches) -> bool {
        let len_limit = self.get_avail().min(self.match_len_max as i32);
        for i in 0..matches.count as usize {
            if self.get_match_len(matches.dist[i] as i32, len_limit) != matches.len[i] as _ {
                return false;
            }
        }
        true
    }

    pub(super) fn move_pos(
        &mut self,
        required_for_flushing: i32,
        required_for_finishing: i32,
    ) -> i32 {
        assert!(required_for_flushing >= required_for_finishing);
        self.read_pos += 1;
        let mut avail = self.write_pos - self.read_pos;
        if avail < required_for_flushing {
            if avail < required_for_finishing || !self.finishing {
                self.pending_size += 1;
                avail = 0;
            }
        }
        avail
    }
}
impl Deref for LZEncoder {
    type Target = LZEncoderData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

fn get_buf_size(
    dict_size: u32,
    extra_size_before: u32,
    extra_size_after: u32,
    match_len_max: u32,
) -> u32 {
    let keep_size_before = extra_size_before + dict_size;
    let keep_size_after = extra_size_after + match_len_max;
    let reserve_size = (dict_size / 2 + (256 << 10)).min(512 << 20);
    keep_size_before + keep_size_after + reserve_size
}
