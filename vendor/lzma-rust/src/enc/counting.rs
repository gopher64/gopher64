use std::{cell::Cell, io::Write, rc::Rc};

pub struct CountingWriter<W> {
    inner: W,
    counting: Rc<Cell<usize>>,
    writed_bytes: usize,
}

impl<W: Write> CountingWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            counting: Rc::new(Cell::new(0)),
            writed_bytes: 0,
        }
    }
    pub fn writed_bytes(&self) -> usize {
        self.writed_bytes
    }

    pub fn counting(&self) -> Rc<Cell<usize>> {
        Rc::clone(&self.counting)
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = self.inner.write(buf)?;
        self.writed_bytes += len;
        self.counting.set(self.writed_bytes);
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
