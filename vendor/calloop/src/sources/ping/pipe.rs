//! Pipe based implementation of the ping event source, using the pipe or pipe2
//! syscall. Sending a ping involves writing to one end of a pipe, and the other
//! end becoming readable is what wakes up the event loop.

use std::{os::unix::io::RawFd, sync::Arc};

use nix::fcntl::OFlag;
use nix::unistd::{close, read, write};

use super::{CloseOnDrop, PingError};
use crate::{
    generic::Generic, EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory,
};

#[cfg(target_os = "macos")]
#[inline]
fn make_ends() -> std::io::Result<(RawFd, RawFd)> {
    // macOS does not have pipe2, but we can emulate the behavior of pipe2 by
    // setting the flags after calling pipe.
    use nix::{
        fcntl::{fcntl, FcntlArg},
        unistd::pipe,
    };

    let (read, write) = pipe()?;

    let read_flags = OFlag::from_bits_truncate(fcntl(read, FcntlArg::F_GETFD)?)
        | OFlag::O_CLOEXEC
        | OFlag::O_NONBLOCK;
    let write_flags = OFlag::from_bits_truncate(fcntl(write, FcntlArg::F_GETFD)?)
        | OFlag::O_CLOEXEC
        | OFlag::O_NONBLOCK;

    fcntl(read, FcntlArg::F_SETFL(read_flags))?;
    fcntl(write, FcntlArg::F_SETFL(write_flags))?;

    Ok((read, write))
}

#[cfg(not(target_os = "macos"))]
#[inline]
fn make_ends() -> std::io::Result<(RawFd, RawFd)> {
    Ok(nix::unistd::pipe2(OFlag::O_CLOEXEC | OFlag::O_NONBLOCK)?)
}

#[inline]
pub fn make_ping() -> std::io::Result<(Ping, PingSource)> {
    let (read, write) = make_ends()?;

    let source = PingSource {
        pipe: Generic::new(read, Interest::READ, Mode::Level),
    };
    let ping = Ping {
        pipe: Arc::new(CloseOnDrop(write)),
    };
    Ok((ping, source))
}

// Helper functions for the event source IO.

#[inline]
fn send_ping(fd: RawFd) -> std::io::Result<()> {
    write(fd, &[0u8])?;
    Ok(())
}

// The event source is simply a generic source with the FD of the read end of
// the pipe.
#[derive(Debug)]
pub struct PingSource {
    pipe: Generic<RawFd>,
}

impl EventSource for PingSource {
    type Event = ();
    type Metadata = ();
    type Ret = ();
    type Error = PingError;

    fn process_events<C>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: C,
    ) -> Result<PostAction, Self::Error>
    where
        C: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.pipe
            .process_events(readiness, token, |_, &mut fd| {
                let mut buf = [0u8; 32];
                let mut read_something = false;
                let mut action = PostAction::Continue;

                loop {
                    match read(fd, &mut buf) {
                        Ok(0) => {
                            // The other end of the pipe was closed, mark ourselves
                            // for removal.
                            action = PostAction::Remove;
                            break;
                        }

                        // Got one or more pings.
                        Ok(_) => read_something = true,

                        // Nothing more to read.
                        Err(nix::errno::Errno::EAGAIN) => break,

                        // Propagate error.
                        Err(e) => return Err(e.into()),
                    }
                }

                if read_something {
                    callback((), &mut ());
                }
                Ok(action)
            })
            .map_err(|e| PingError(e.into()))
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> crate::Result<()> {
        self.pipe.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> crate::Result<()> {
        self.pipe.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> crate::Result<()> {
        self.pipe.unregister(poll)
    }
}

impl Drop for PingSource {
    fn drop(&mut self) {
        if let Err(e) = close(self.pipe.file) {
            log::warn!("[calloop] Failed to close read ping: {:?}", e);
        }
    }
}

// The sending end of the ping writes zeroes to the write end of the pipe.
#[derive(Clone, Debug)]
pub struct Ping {
    pipe: Arc<CloseOnDrop>,
}

// The sending end of the ping writes zeroes to the write end of the pipe.
impl Ping {
    /// Send a ping to the `PingSource`
    pub fn ping(&self) {
        if let Err(e) = send_ping(self.pipe.0) {
            log::warn!("[calloop] Failed to write a ping: {:?}", e);
        }
    }
}
