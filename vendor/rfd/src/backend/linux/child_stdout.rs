use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use async_io::Async;
use futures_util::AsyncRead;

pub struct ChildStdout(Async<std::process::ChildStdout>);

impl ChildStdout {
    pub fn new(stdout: std::process::ChildStdout) -> io::Result<Self> {
        Async::new(stdout).map(Self)
    }
}

impl AsyncRead for ChildStdout {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
