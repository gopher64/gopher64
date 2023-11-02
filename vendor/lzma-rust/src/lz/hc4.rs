use std::vec;

use super::{
    hash234::Hash234,
    lz_encoder::{LZEncoder, MatchFind, Matches},
    LZEncoderData,
};

pub struct HC4 {
    hash: Hash234,
    chain: Vec<i32>,
    // matches: Matches,
    depth_limit: i32,
    cyclic_size: i32,
    cyclic_pos: i32,
    lz_pos: i32,
}

impl HC4 {
    pub fn get_mem_usage(dict_size: u32) -> u32 {
        Hash234::get_mem_usage(dict_size) + dict_size / (1024 / 4) + 10
    }

    pub fn new(dict_size: u32, nice_len: u32, depth_limit: i32) -> Self {
        Self {
            hash: Hash234::new(dict_size),
            chain: vec![0; dict_size as usize + 1],
            // matches: Matches::new(nice_len as usize - 1),
            depth_limit: if depth_limit > 0 {
                depth_limit as i32
            } else {
                4 + nice_len as i32 / 4
            },
            cyclic_size: dict_size as i32 + 1,
            cyclic_pos: -1,
            lz_pos: dict_size as i32 + 1,
        }
    }

    fn move_pos(&mut self, encoder: &mut LZEncoderData) -> i32 {
        let avail = encoder.move_pos(4, 4);
        if avail != 0 {
            self.lz_pos += 1;
            if self.lz_pos == 0x7fffffff {
                let norm_offset = 0x7fffffff - self.cyclic_size;
                self.hash.normalize(norm_offset);
                LZEncoder::normalize(&mut self.chain[..self.cyclic_size as usize], norm_offset);
                self.lz_pos = self.lz_pos.wrapping_sub(norm_offset);
            }

            self.cyclic_pos += 1;
            if self.cyclic_pos == self.cyclic_size as i32 {
                self.cyclic_pos = 0;
            }
        }

        avail
    }
}

impl MatchFind for HC4 {
    fn find_matches(
        &mut self,
        encoder: &mut super::lz_encoder::LZEncoderData,
        matches: &mut Matches,
    ) {
        matches.count = 0;
        let mut match_len_limit = encoder.match_len_max as i32;
        let mut nice_len_limit = encoder.nice_len as i32;
        let avail = self.move_pos(encoder);

        if avail < match_len_limit {
            if avail == 0 {
                return;
            }
            match_len_limit = avail;
            if nice_len_limit > avail {
                nice_len_limit = avail;
            }
        }
        self.hash.calc_hashes(encoder.buf_mut());
        let mut delta2 = self.lz_pos.wrapping_sub(self.hash.get_hash2_pos());
        let delta3 = self.lz_pos.wrapping_sub(self.hash.get_hash3_pos());
        let mut current_match = self.hash.get_hash4_pos();
        self.hash.update_tables(self.lz_pos);
        self.chain[self.cyclic_pos as usize] = current_match;
        let mut len_best = 0;

        if delta2 < self.cyclic_size
            && encoder.buf[encoder.read_pos as usize - delta2 as usize]
                == encoder.buf[encoder.read_pos as usize]
        {
            len_best = 2;
            matches.len[0] = 2;
            matches.dist[0] = delta2 - 1;
            matches.count = 1;
        }

        if delta2 != delta3
            && delta3 < self.cyclic_size
            && encoder.get_byte(0, delta3 as i32) == encoder.get_current_byte()
        {
            len_best = 3;
            let count = matches.count as usize;
            matches.dist[count] = delta3 - 1;
            matches.count += 1;
            delta2 = delta3;
        }

        if matches.count > 0 {
            while len_best < match_len_limit
                && encoder.get_byte(len_best, delta2 as _) == encoder.get_byte(len_best, 0)
            {
                len_best += 1;
            }

            let count = matches.count as usize;
            matches.len[count - 1] = len_best as u32;

            // Return if it is long enough (niceLen or reached the end of
            // the dictionary).
            if len_best >= nice_len_limit {
                return;
            }
        }

        if len_best < 3 {
            len_best = 3;
        }

        let mut depth = self.depth_limit;
        loop {
            let delta = (self.lz_pos - current_match) as i32;
            if {
                let tmp = depth;
                depth -= 1;
                tmp
            } == 0
                || delta >= self.cyclic_size as i32
            {
                return;
            }
            let i = self.cyclic_pos - delta
                + if delta > self.cyclic_pos {
                    self.cyclic_size as i32
                } else {
                    0
                };
            current_match = self.chain[i as usize];

            if encoder.get_byte(len_best, delta) == encoder.get_byte(len_best, 0)
                && encoder.get_byte(0, delta as i32) == encoder.get_current_byte()
            {
                // Calculate the length of the match.
                let mut len = 0;
                while {
                    len += 1;
                    len
                } < match_len_limit
                {
                    if encoder.get_byte(len, delta) != encoder.get_byte(len, 0) {
                        break;
                    }
                    // len += 1;
                }

                // Use the match if and only if it is better than the longest
                // match found so far.
                if len > len_best {
                    len_best = len;
                    let count = matches.count as usize;
                    matches.len[count] = len as _;
                    matches.dist[count] = (delta - 1) as _;
                    matches.count += 1;

                    // Return if it is long enough (niceLen or reached the
                    // end of the dictionary).
                    if len >= nice_len_limit {
                        return;
                    }
                }
            }
        }
    }

    fn skip(&mut self, encoder: &mut super::lz_encoder::LZEncoderData, mut len: usize) {
        while len > 0 {
            len -= 1;
            if self.move_pos(encoder) != 0 {
                self.hash.calc_hashes(encoder.buf_mut());
                self.chain[self.cyclic_pos as usize] = self.hash.get_hash4_pos();
                self.hash.update_tables(self.lz_pos);
            }
        }
    }

}
