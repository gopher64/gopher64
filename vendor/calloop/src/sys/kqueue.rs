use std::{io, os::unix::io::RawFd};

use nix::{
    libc::{c_long, time_t, timespec},
    sys::event::{kevent_ts, kqueue, EventFilter, EventFlag, FilterFlag, KEvent},
};

use super::{Interest, Mode, PollEvent, Readiness, Token};

pub struct Kqueue {
    kq: RawFd,
}

fn mode_to_flag(mode: Mode) -> EventFlag {
    match mode {
        Mode::Level => EventFlag::empty(),
        Mode::OneShot => EventFlag::EV_DISPATCH,
        Mode::Edge => EventFlag::EV_CLEAR,
    }
}

impl Kqueue {
    // Kqueue is always high precision
    pub(crate) fn new(_high_precision: bool) -> crate::Result<Kqueue> {
        let kq = kqueue()?;
        Ok(Kqueue { kq })
    }

    pub(crate) fn poll(
        &mut self,
        timeout: Option<std::time::Duration>,
    ) -> crate::Result<Vec<PollEvent>> {
        let mut buffer = [KEvent::new(
            0,
            EventFilter::EVFILT_READ,
            EventFlag::empty(),
            FilterFlag::empty(),
            0,
            0,
        ); 32];

        let nevents = kevent_ts(
            self.kq,
            &[],
            &mut buffer,
            timeout.map(|d| timespec {
                tv_sec: d.as_secs() as time_t,
                tv_nsec: d.subsec_nanos() as c_long,
            }),
        )?;

        let ret = buffer
            .iter()
            .take(nevents)
            .map(|event| {
                // The kevent data field in Rust's libc FFI bindings is an
                // intptr_t, which is specified to allow this kind of
                // conversion.
                let token_ptr = event.udata() as usize as *const Token;
                PollEvent {
                    readiness: Readiness {
                        readable: event.filter() == Ok(EventFilter::EVFILT_READ),
                        writable: event.filter() == Ok(EventFilter::EVFILT_WRITE),
                        error: event.flags().contains(EventFlag::EV_ERROR) && event.data() != 0,
                    },
                    // Why this is safe: it points to memory boxed and owned by
                    // the parent Poller type.
                    token: unsafe { *token_ptr },
                }
            })
            .collect();
        Ok(ret)
    }

    pub fn register(
        &mut self,
        fd: RawFd,
        interest: Interest,
        mode: Mode,
        token: *const Token,
    ) -> crate::Result<()> {
        self.reregister(fd, interest, mode, token)
    }

    pub fn reregister(
        &mut self,
        fd: RawFd,
        interest: Interest,
        mode: Mode,
        token: *const Token,
    ) -> crate::Result<()> {
        let write_flags = if interest.writable {
            EventFlag::EV_ADD | EventFlag::EV_RECEIPT | mode_to_flag(mode)
        } else {
            EventFlag::EV_DELETE
        };
        let read_flags = if interest.readable {
            EventFlag::EV_ADD | EventFlag::EV_RECEIPT | mode_to_flag(mode)
        } else {
            EventFlag::EV_DELETE
        };

        let changes = [
            KEvent::new(
                fd as usize,
                EventFilter::EVFILT_WRITE,
                write_flags,
                FilterFlag::empty(),
                0,
                token as usize as isize,
            ),
            KEvent::new(
                fd as usize,
                EventFilter::EVFILT_READ,
                read_flags,
                FilterFlag::empty(),
                0,
                token as usize as isize,
            ),
        ];

        let mut out = [
            KEvent::new(
                0,
                EventFilter::EVFILT_WRITE,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            ),
            KEvent::new(
                0,
                EventFilter::EVFILT_READ,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            ),
        ];

        kevent_ts(self.kq, &changes, &mut out, None)?;

        for o in &out {
            if o.flags().contains(EventFlag::EV_ERROR) && o.data() != 0 {
                let e = io::Error::from_raw_os_error(o.data() as i32);
                // ignore NotFound error which is raised if we tried to remove a non-existent filter
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    pub fn unregister(&mut self, fd: RawFd) -> crate::Result<()> {
        let changes = [
            KEvent::new(
                fd as usize,
                EventFilter::EVFILT_WRITE,
                EventFlag::EV_DELETE,
                FilterFlag::empty(),
                0,
                0,
            ),
            KEvent::new(
                fd as usize,
                EventFilter::EVFILT_READ,
                EventFlag::EV_DELETE,
                FilterFlag::empty(),
                0,
                0,
            ),
        ];

        let mut out = [
            KEvent::new(
                0,
                EventFilter::EVFILT_WRITE,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            ),
            KEvent::new(
                0,
                EventFilter::EVFILT_READ,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            ),
        ];

        kevent_ts(self.kq, &changes, &mut out, None)?;

        // Report an error if *both* fd were missing, meaning we were not registered at all
        let mut notfound = 0;

        for o in &out {
            if o.flags().contains(EventFlag::EV_ERROR) && o.data() != 0 {
                let e = io::Error::from_raw_os_error(o.data() as i32);
                // ignore NotFound error which is raised if we tried to remove a non-existent filter
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(e.into());
                } else {
                    notfound += 1;
                }
            }
        }

        if notfound == 2 {
            return Err(std::io::Error::from(io::ErrorKind::NotFound).into());
        }

        Ok(())
    }
}

impl Drop for Kqueue {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.kq);
    }
}
