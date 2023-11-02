use std::{cell::RefCell, convert::TryInto, os::unix::io::RawFd, rc::Rc, time::Duration};
use vec_map::VecMap;

use crate::{loop_logic::CalloopKey, sources::timer::TimerWheel};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod epoll;
#[cfg(any(target_os = "linux", target_os = "android"))]
use epoll::Epoll as Poller;

#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "macos"
))]
mod kqueue;
#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "macos"
))]
use kqueue::Kqueue as Poller;

/// Possible modes for registering a file descriptor
#[derive(Copy, Clone, Debug)]
pub enum Mode {
    /// Single event generation
    ///
    /// This FD will be disabled as soon as it has generated one event.
    ///
    /// The user will need to use `LoopHandle::update()` to re-enable it if
    /// desired.
    OneShot,
    /// Level-triggering
    ///
    /// This FD will report events on every poll as long as the requested interests
    /// are available. If the same FD is inserted in multiple event loops, all of
    /// them are notified of readiness.
    Level,
    /// Edge-triggering
    ///
    /// This FD will report events only when it *gains* one of the requested interests.
    /// it must thus be fully processed before it'll generate events again. If the same
    /// FD is inserted on multiple event loops, it may be that not all of them are notified
    /// of readiness, and not necessarily always the same(s) (at least one is notified).
    Edge,
}

/// Interest to register regarding the file descriptor
#[derive(Copy, Clone, Debug)]
pub struct Interest {
    /// Wait for the FD to be readable
    pub readable: bool,
    /// Wait for the FD to be writable
    pub writable: bool,
}

impl Interest {
    /// Shorthand for empty interest
    pub const EMPTY: Interest = Interest {
        readable: false,
        writable: false,
    };
    /// Shorthand for read interest
    pub const READ: Interest = Interest {
        readable: true,
        writable: false,
    };
    /// Shorthand for write interest
    pub const WRITE: Interest = Interest {
        readable: false,
        writable: true,
    };
    /// Shorthand for read and write interest
    pub const BOTH: Interest = Interest {
        readable: true,
        writable: true,
    };
}

/// Readiness for a file descriptor notification
#[derive(Copy, Clone, Debug)]
pub struct Readiness {
    /// Is the FD readable
    pub readable: bool,
    /// Is the FD writable
    pub writable: bool,
    /// Is the FD in an error state
    pub error: bool,
}

impl Readiness {
    /// Shorthand for empty readiness
    pub const EMPTY: Readiness = Readiness {
        readable: false,
        writable: false,
        error: false,
    };
}

#[derive(Debug)]
pub(crate) struct PollEvent {
    pub(crate) readiness: Readiness,
    pub(crate) token: Token,
}

/// Factory for creating tokens in your registrations
///
/// When composing event sources, each sub-source needs to
/// have its own token to identify itself. This factory is
/// provided to produce such unique tokens.

#[derive(Debug)]
pub struct TokenFactory {
    key: CalloopKey,
    sub_id: u32,
}

impl TokenFactory {
    pub(crate) fn new(key: CalloopKey) -> TokenFactory {
        TokenFactory { key, sub_id: 0 }
    }

    /// Produce a new unique token
    pub fn token(&mut self) -> Token {
        let token = Token {
            key: self.key,
            sub_id: self.sub_id,
        };
        self.sub_id += 1;
        token
    }
}

/// A token (for implementation of the [`EventSource`](crate::EventSource) trait)
///
/// This token is produced by the [`TokenFactory`] and is used when calling the
/// [`EventSource`](crate::EventSource) implementations to process event, in order
/// to identify which sub-source produced them.
///
/// You should forward it to the [`Poll`] when registering your file descriptors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Token {
    pub(crate) key: CalloopKey,
    pub(crate) sub_id: u32,
}

/// The polling system
///
/// This type represents the polling system of calloop, on which you
/// can register your file descriptors. This interface is only accessible in
/// implementations of the [`EventSource`](crate::EventSource) trait.
///
/// You only need to interact with this type if you are implementing your
/// own event sources, while implementing the [`EventSource`](crate::EventSource) trait.
/// And even in this case, you can often just use the [`Generic`](crate::generic::Generic) event
/// source and delegate the implementations to it.
pub struct Poll {
    poller: Poller,

    // It is essential for safe use of this type that the pointers passed in to
    // the underlying poller API are properly managed. Each time an event source
    // is registered, the token it passes in is Boxed and converted to a raw
    // pointer to be passed to the polling system by FFI. This pointer is what's
    // stored in the map. When the event source is re- or unregistered, the same
    // raw pointer can then be converted back into the Box and dropped, safely
    // deallocating it. To put it another way, we effectively "own" the Token
    // memory on behalf of the underlying polling mechanism.
    //
    // All the platforms we currently support follow the rule that file
    // descriptors must be "small", positive integers. This means we can use a
    // VecMap which has that exact constraint for its keys. If that ever
    // changes, this will need to be changed to a different structure.
    tokens: VecMap<*mut Token>,
    pub(crate) timers: Rc<RefCell<TimerWheel>>,
}

impl std::fmt::Debug for Poll {
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Poll { ... }")
    }
}

impl Poll {
    pub(crate) fn new(high_precision: bool) -> crate::Result<Poll> {
        Ok(Poll {
            poller: Poller::new(high_precision)?,
            tokens: VecMap::new(),
            timers: Rc::new(RefCell::new(TimerWheel::new())),
        })
    }

    pub(crate) fn poll(
        &mut self,
        mut timeout: Option<std::time::Duration>,
    ) -> crate::Result<Vec<PollEvent>> {
        let now = std::time::Instant::now();
        // adjust the timeout for the timers
        if let Some(next_timeout) = self.timers.borrow().next_deadline() {
            if next_timeout <= now {
                timeout = Some(Duration::ZERO);
            } else if let Some(deadline) = timeout {
                timeout = Some(std::cmp::min(deadline, next_timeout - now));
            } else {
                timeout = Some(next_timeout - now);
            }
        };

        let mut events = self.poller.poll(timeout)?;

        // Update 'now' as some time may have elapsed in poll()
        let now = std::time::Instant::now();
        let mut timers = self.timers.borrow_mut();
        while let Some((_, token)) = timers.next_expired(now) {
            events.push(PollEvent {
                readiness: Readiness {
                    readable: true,
                    writable: false,
                    error: false,
                },
                token,
            });
        }

        Ok(events)
    }

    /// Register a new file descriptor for polling
    ///
    /// The file descriptor will be registered with given interest,
    /// mode and token. This function will fail if given a
    /// bad file descriptor or if the provided file descriptor is already
    /// registered.
    ///
    /// # Leaking tokens
    ///
    /// If your event source is dropped without being unregistered, the token
    /// passed in here will remain on the heap and continue to be used by the
    /// polling system even though no event source will match it.
    pub fn register(
        &mut self,
        fd: RawFd,
        interest: Interest,
        mode: Mode,
        token: Token,
    ) -> crate::Result<()> {
        let token_box = Box::new(token);
        let token_ptr = Box::into_raw(token_box);

        let registration_result = self.poller.register(fd, interest, mode, token_ptr);

        if registration_result.is_err() {
            // If registration did not work, do not add the file descriptor to
            // the token map. Instead, reconstruct the Box and drop it. This is
            // safe because it's from Box::into_raw() above.
            let token_box = unsafe { Box::from_raw(token_ptr) };
            std::mem::drop(token_box);
        } else {
            // Registration worked, keep the token pointer until it's replaced
            // or removed.
            let index = index_from_fd(fd);
            if self.tokens.insert(index, token_ptr).is_some() {
                // If there is already a file descriptor associated with a
                // token, then replacing that entry will leak the token, but
                // converting it back into a Box might leave a dangling pointer
                // somewhere. We can theoretically continue safely by choosing
                // to leak, but one of our assumptions is no longer valid, so
                // panic.
                panic!("File descriptor ({}) already registered", fd);
            }
        }

        registration_result
    }

    /// Update the registration for a file descriptor
    ///
    /// This allows you to change the interest, mode or token of a file
    /// descriptor. Fails if the provided fd is not currently registered.
    ///
    /// See note on [`register()`](Self::register()) regarding leaking.
    pub fn reregister(
        &mut self,
        fd: RawFd,
        interest: Interest,
        mode: Mode,
        token: Token,
    ) -> crate::Result<()> {
        let token_box = Box::new(token);
        let token_ptr = Box::into_raw(token_box);

        let reregistration_result = self.poller.reregister(fd, interest, mode, token_ptr);

        if reregistration_result.is_err() {
            // If registration did not work, do not add the file descriptor to
            // the token map. Instead, reconstruct the Box and drop it. This is
            // safe because it's from Box::into_raw() above.
            let token_box = unsafe { Box::from_raw(token_ptr) };
            std::mem::drop(token_box);
        } else {
            // Registration worked, drop the old token memory and keep the new
            // token pointer until it's replaced or removed.
            let index = index_from_fd(fd);
            if let Some(previous) = self.tokens.insert(index, token_ptr) {
                // This is safe because it's from Box::into_raw() from a
                // previous (re-)register() call.
                let token_box = unsafe { Box::from_raw(previous) };
                std::mem::drop(token_box);
            } else {
                // If there is no previous token registered for this file
                // descriptor, either the event source has wrongly called
                // reregister() without first being registered, or the
                // underlying poller has a dangling pointer. In the first case,
                // the reregistration should have failed; in the second case, we
                // cannot safely proceed.
                panic!("File descriptor ({}) had no previous registration", fd);
            }
        }

        reregistration_result
    }

    /// Unregister a file descriptor
    ///
    /// This file descriptor will no longer generate events. Fails if the
    /// provided file descriptor is not currently registered.
    pub fn unregister(&mut self, fd: RawFd) -> crate::Result<()> {
        let unregistration_result = self.poller.unregister(fd);

        if unregistration_result.is_ok() {
            // The source was unregistered, we can remove the old token data.
            let index = index_from_fd(fd);
            if let Some(previous) = self.tokens.remove(index) {
                // This is safe because it's from Box::into_raw() from a
                // previous (re-)register() call.
                let token_box = unsafe { Box::from_raw(previous) };
                std::mem::drop(token_box);
            } else {
                // If there is no previous token registered for this file
                // descriptor, either the event source has wrongly called
                // unregister() without first being registered, or the
                // underlying poller has a dangling pointer. In the first case,
                // the reregistration should have failed; in the second case, we
                // cannot safely proceed.
                panic!("File descriptor ({}) had no previous registration", fd);
            }
        }

        unregistration_result
    }
}

/// Converts a file descriptor into an index for the token map. Panics if the
/// file descriptor is negative.
fn index_from_fd(fd: RawFd) -> usize {
    fd.try_into()
        .unwrap_or_else(|_| panic!("File descriptor ({}) is invalid", fd))
}
