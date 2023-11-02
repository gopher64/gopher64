mod arm;
mod ppc;
mod sparc;
mod x86;
use std::io::Read;

pub struct BCJFilter {
    is_encoder: bool,
    pos: usize,
    prev_mask: u32,
    filter: FilterFn,
}
pub type FilterFn = fn(filter: &mut BCJFilter, buf: &mut [u8]) -> usize;

impl BCJFilter {
    #[inline]
    fn code(&mut self, buf: &mut [u8]) -> usize {
        let filter = self.filter;
        filter(self, buf)
    }
}
const FILTER_BUF_SIZE: usize = 4096;
pub struct SimpleReader<R> {
    inner: R,
    filter: BCJFilter,
    state: State,
    err: Option<std::io::Error>,
}

#[derive(Debug)]
struct State {
    filter_buf: Vec<u8>,
    pos: usize,
    filtered: usize,
    unfiltered: usize,
    end_reached: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            filter_buf: Default::default(),
            pos: Default::default(),
            filtered: Default::default(),
            unfiltered: Default::default(),
            end_reached: false,
        }
    }
}
impl<R> SimpleReader<R> {
    fn new(inner: R, filter: BCJFilter) -> Self {
        Self {
            inner,
            filter,
            state: State {
                filter_buf: vec![0; FILTER_BUF_SIZE],
                ..Default::default()
            },
            err: None,
        }
    }
    #[inline]
    pub fn new_x86(inner: R) -> Self {
        Self::new(inner, BCJFilter::new_x86(0, false))
    }

    #[inline]
    pub fn new_arm(inner: R) -> Self {
        Self::new(inner, BCJFilter::new_arm(0, false))
    }
    #[inline]
    pub fn new_arm_thumb(inner: R) -> Self {
        Self::new(inner, BCJFilter::new_arm_thumb(0, false))
    }
    #[inline]
    pub fn new_ppc(inner: R) -> Self {
        Self::new(inner, BCJFilter::new_power_pc(0, false))
    }
    #[inline]
    pub fn new_sparc(inner: R) -> Self {
        Self::new(inner, BCJFilter::new_sparc(0, false))
    }
}
impl<R: Read> Read for SimpleReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        if let Some(e) = self.err.as_ref() {
            return Err(std::io::Error::new(e.kind(), e.to_string()));
        }
        let mut len = buf.len();
        let mut state = std::mem::replace(&mut self.state, State::default());
        let mut off = 0;
        let mut size = 0;

        loop {
            // Copy filtered data into the caller-provided buffer.
            if state.filtered > 0 {
                let copy_size = state.filtered.min(len);
                let pos = state.pos;
                buf[off..(off + copy_size)]
                    .copy_from_slice(&state.filter_buf[pos..(pos + copy_size)]);
                state.pos += copy_size;
                state.filtered -= copy_size;
                off += copy_size;
                len -= copy_size;
                size += copy_size;
            }

            // If end of filterBuf was reached, move the pending data to
            // the beginning of the buffer so that more data can be
            // copied into filterBuf on the next loop iteration.
            if state.pos + state.filtered + state.unfiltered == FILTER_BUF_SIZE {
                

                // state.filter_buf.copy_from_slice(src);
                state.filter_buf.rotate_left(state.pos);
                state.pos = 0;
            }

            if len == 0 || state.end_reached {
                self.state = state;
                return Ok(if size > 0 { size } else { 0 });
            }

            assert!(state.filtered == 0);
            // Get more data into the temporary buffer.
            let mut in_size = FILTER_BUF_SIZE - (state.pos + state.filtered + state.unfiltered);
            let start = state.pos + state.filtered + state.unfiltered;
            let temp = &mut state.filter_buf[start..(start + in_size)];
            in_size = match self.inner.read(temp) {
                Ok(s) => s,
                Err(e) => {
                    let err = std::io::Error::new(e.kind(), e.to_string());
                    self.err = Some(err);
                    self.state = state;
                    return Err(e);
                }
            };

            if in_size == 0 {
                // Mark the remaining unfiltered bytes to be ready
                // to be copied out.
                state.end_reached = true;
                state.filtered = state.unfiltered;
                state.unfiltered = 0;
            } else {
                // Filter the data in filterBuf.
                state.unfiltered += in_size;
                state.filtered = self
                    .filter
                    .code(&mut state.filter_buf[state.pos..(state.pos + state.unfiltered)]);
                assert!(state.filtered <= state.unfiltered);
                state.unfiltered -= state.filtered;
            }
        }
    }
}
