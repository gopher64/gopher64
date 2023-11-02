use std::os::unix::io::{AsRawFd, RawFd};

use super::{Interest, Mode, PollEvent, Readiness, Token};

use nix::sys::{
    epoll::{
        epoll_create1, epoll_ctl, epoll_wait, EpollCreateFlags, EpollEvent, EpollFlags, EpollOp,
    },
    time::TimeSpec,
    timerfd::{ClockId, Expiration, TimerFd, TimerFlags, TimerSetTimeFlags},
};

pub struct Epoll {
    epoll_fd: RawFd,
    timer_fd: Option<TimerFd>,
}

const TIMER_DATA: u64 = u64::MAX;

fn make_flags(interest: Interest, mode: Mode) -> EpollFlags {
    let mut flags = EpollFlags::empty();
    if interest.readable {
        flags |= EpollFlags::EPOLLIN;
    }
    if interest.writable {
        flags |= EpollFlags::EPOLLOUT;
    }
    match mode {
        Mode::Level => { /* This is the default */ }
        Mode::Edge => flags |= EpollFlags::EPOLLET,
        Mode::OneShot => flags |= EpollFlags::EPOLLONESHOT,
    }
    flags
}

fn flags_to_readiness(flags: EpollFlags) -> Readiness {
    Readiness {
        readable: flags.contains(EpollFlags::EPOLLIN),
        writable: flags.contains(EpollFlags::EPOLLOUT),
        error: flags.contains(EpollFlags::EPOLLERR),
    }
}

impl Epoll {
    pub(crate) fn new(high_precision: bool) -> crate::Result<Epoll> {
        let epoll_fd = epoll_create1(EpollCreateFlags::EPOLL_CLOEXEC)?;
        let mut timer_fd = None;
        if high_precision {
            // Prepare a timerfd for precise time tracking and register it to the event queue
            // This timerfd allows for nanosecond precision in setting the timout up (though in practice
            // we rather get ~10 microsecond precision), while epoll_wait() API only allows millisecond
            // granularity
            let timer = TimerFd::new(
                ClockId::CLOCK_MONOTONIC,
                TimerFlags::TFD_CLOEXEC | TimerFlags::TFD_NONBLOCK,
            )?;
            let mut timer_event = EpollEvent::new(EpollFlags::EPOLLIN, TIMER_DATA);
            epoll_ctl(
                epoll_fd,
                EpollOp::EpollCtlAdd,
                timer.as_raw_fd(),
                &mut timer_event,
            )?;
            timer_fd = Some(timer);
        }
        Ok(Epoll { epoll_fd, timer_fd })
    }

    pub(crate) fn poll(
        &mut self,
        timeout: Option<std::time::Duration>,
    ) -> crate::Result<Vec<PollEvent>> {
        let mut buffer = [EpollEvent::empty(); 32];
        if let Some(ref timer) = self.timer_fd {
            if let Some(timeout) = timeout {
                // Set up the precise timer
                timer.set(
                    Expiration::OneShot(TimeSpec::from_duration(timeout)),
                    TimerSetTimeFlags::empty(),
                )?;
            }
        }
        // add 1 to the millisecond wait, to round up for timer tracking. If the high precision timer is set up
        // it'll fire before that timeout
        let timeout = timeout.map(|d| (d.as_millis() + 1) as isize).unwrap_or(-1);
        let n_ready = epoll_wait(self.epoll_fd, &mut buffer, timeout)?;
        let events = buffer
            .iter()
            .take(n_ready)
            .flat_map(|event| {
                if event.data() == TIMER_DATA {
                    // We woke up because the high-precision timer fired, we need to disarm it by reading its
                    // contents to ensure it will be ready for next time
                    // Timer is created in non-blocking mode, and should have already fired anyway, this
                    // cannot possibly block
                    let _ = self
                        .timer_fd
                        .as_ref()
                        .expect("Got an event from high-precision timer while it is not set up?!")
                        .wait();
                    // don't forward this event to downstream
                    None
                } else {
                    // In C, the underlying data type is a union including a void
                    // pointer; in Rust's FFI bindings, it only exposes the u64. The
                    // round-trip conversion is valid however.
                    let token_ptr = event.data() as usize as *const Token;
                    Some(PollEvent {
                        readiness: flags_to_readiness(event.events()),
                        // Why this is safe: it points to memory boxed and owned by
                        // the parent Poller type.
                        token: unsafe { *token_ptr },
                    })
                }
            })
            .collect();
        if let Some(ref timer) = self.timer_fd {
            // in all cases, disarm the timer
            timer.unset()?;
            // clear the timer in case it fired between epoll_wait and now, as timer is in
            // non-blocking mode, this will return Err(WouldBlock) if it had not fired, so
            // we ignore the error
            let _ = timer.wait();
        }
        Ok(events)
    }

    pub fn register(
        &mut self,
        fd: RawFd,
        interest: Interest,
        mode: Mode,
        token: *const Token,
    ) -> crate::Result<()> {
        let mut event = EpollEvent::new(make_flags(interest, mode), token as usize as u64);
        epoll_ctl(self.epoll_fd, EpollOp::EpollCtlAdd, fd, &mut event).map_err(Into::into)
    }

    pub fn reregister(
        &mut self,
        fd: RawFd,
        interest: Interest,
        mode: Mode,
        token: *const Token,
    ) -> crate::Result<()> {
        let mut event = EpollEvent::new(make_flags(interest, mode), token as usize as u64);
        epoll_ctl(self.epoll_fd, EpollOp::EpollCtlMod, fd, &mut event).map_err(Into::into)
    }

    pub fn unregister(&mut self, fd: RawFd) -> crate::Result<()> {
        epoll_ctl(self.epoll_fd, EpollOp::EpollCtlDel, fd, None).map_err(Into::into)
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.epoll_fd);
    }
}
