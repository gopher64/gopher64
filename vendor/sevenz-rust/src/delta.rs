use std::io::Read;

const MAX_DISTANCE: usize = 256;
const _MIN_DISTANCE: usize = 1;
const DIS_MASK: usize = MAX_DISTANCE - 1;
struct Delta {
    distance: usize,
    history: [u8; MAX_DISTANCE],
    pos: u8,
}

impl Delta {
    pub fn new(distance: usize) -> Self {
        Self {
            distance,
            history: [0; MAX_DISTANCE],
            pos: 0,
        }
    }

    pub fn decode(&mut self, buf: &mut [u8]) {
        for i in 0..buf.len() {
            let pos = self.pos as usize;
            let h = self.history[(self.distance.wrapping_add(pos)) & DIS_MASK];
            buf[i] = buf[i].wrapping_add(h);
            self.history[pos & DIS_MASK] = buf[i];
            self.pos = self.pos.wrapping_sub(1);
        }
    }
}

pub struct DeltaReader<R> {
    inner: R,
    delta: Delta,
}

impl<R> DeltaReader<R> {
    pub fn new(inner: R, distance: usize) -> Self {
        Self {
            inner,
            delta: Delta::new(distance),
        }
    }
}

impl<R: Read> Read for DeltaReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 {
            return Ok(n);
        }
        self.delta.decode(&mut buf[..n]);
        Ok(n)
    }
}
