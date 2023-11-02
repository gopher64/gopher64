use libc::c_int;

use crate::FromEnvErrorInner;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::mem;
use std::mem::MaybeUninit;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::{Arc, Once};
use std::thread::{self, Builder, JoinHandle};
use std::time::Duration;

#[derive(Debug)]
pub enum Client {
    /// `--jobserver-auth=R,W`
    Pipe { read: File, write: File },
    /// `--jobserver-auth=fifo:PATH`
    Fifo { file: File, path: PathBuf },
}

#[derive(Debug)]
pub struct Acquired {
    byte: u8,
}

impl Client {
    pub fn new(mut limit: usize) -> io::Result<Client> {
        let client = unsafe { Client::mk()? };

        // I don't think the character written here matters, but I could be
        // wrong!
        const BUFFER: [u8; 128] = [b'|'; 128];

        let mut write = client.write();

        set_nonblocking(write.as_raw_fd(), true)?;

        while limit > 0 {
            let n = limit.min(BUFFER.len());

            write.write_all(&BUFFER[..n])?;
            limit -= n;
        }

        set_nonblocking(write.as_raw_fd(), false)?;

        Ok(client)
    }

    unsafe fn mk() -> io::Result<Client> {
        let mut pipes = [0; 2];

        // Attempt atomically-create-with-cloexec if we can on Linux,
        // detected by using the `syscall` function in `libc` to try to work
        // with as many kernels/glibc implementations as possible.
        #[cfg(target_os = "linux")]
        {
            use std::sync::atomic::{AtomicBool, Ordering};

            static PIPE2_AVAILABLE: AtomicBool = AtomicBool::new(true);
            if PIPE2_AVAILABLE.load(Ordering::SeqCst) {
                match libc::syscall(libc::SYS_pipe2, pipes.as_mut_ptr(), libc::O_CLOEXEC) {
                    -1 => {
                        let err = io::Error::last_os_error();
                        if err.raw_os_error() == Some(libc::ENOSYS) {
                            PIPE2_AVAILABLE.store(false, Ordering::SeqCst);
                        } else {
                            return Err(err);
                        }
                    }
                    _ => return Ok(Client::from_fds(pipes[0], pipes[1])),
                }
            }
        }

        cvt(libc::pipe(pipes.as_mut_ptr()))?;
        drop(set_cloexec(pipes[0], true));
        drop(set_cloexec(pipes[1], true));
        Ok(Client::from_fds(pipes[0], pipes[1]))
    }

    pub(crate) unsafe fn open(s: &str, check_pipe: bool) -> Result<Client, FromEnvErrorInner> {
        if let Some(client) = Self::from_fifo(s)? {
            return Ok(client);
        }
        if let Some(client) = Self::from_pipe(s, check_pipe)? {
            return Ok(client);
        }
        Err(FromEnvErrorInner::CannotParse(format!(
            "expected `fifo:PATH` or `R,W`, found `{s}`"
        )))
    }

    /// `--jobserver-auth=fifo:PATH`
    fn from_fifo(s: &str) -> Result<Option<Client>, FromEnvErrorInner> {
        let mut parts = s.splitn(2, ':');
        if parts.next().unwrap() != "fifo" {
            return Ok(None);
        }
        let path_str = parts.next().ok_or_else(|| {
            FromEnvErrorInner::CannotParse("expected a path after `fifo:`".to_string())
        })?;
        let path = Path::new(path_str);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|err| FromEnvErrorInner::CannotOpenPath(path_str.to_string(), err))?;
        Ok(Some(Client::Fifo {
            file,
            path: path.into(),
        }))
    }

    /// `--jobserver-auth=R,W`
    unsafe fn from_pipe(s: &str, check_pipe: bool) -> Result<Option<Client>, FromEnvErrorInner> {
        let mut parts = s.splitn(2, ',');
        let read = parts.next().unwrap();
        let write = match parts.next() {
            Some(w) => w,
            None => return Ok(None),
        };
        let read = read
            .parse()
            .map_err(|e| FromEnvErrorInner::CannotParse(format!("cannot parse `read` fd: {e}")))?;
        let write = write
            .parse()
            .map_err(|e| FromEnvErrorInner::CannotParse(format!("cannot parse `write` fd: {e}")))?;

        // Ok so we've got two integers that look like file descriptors, but
        // for extra sanity checking let's see if they actually look like
        // valid files and instances of a pipe if feature enabled before we
        // return the client.
        //
        // If we're called from `make` *without* the leading + on our rule
        // then we'll have `MAKEFLAGS` env vars but won't actually have
        // access to the file descriptors.
        //
        // `NotAPipe` is a worse error, return it if it's reported for any of the two fds.
        match (fd_check(read, check_pipe), fd_check(write, check_pipe)) {
            (read_err @ Err(FromEnvErrorInner::NotAPipe(..)), _) => read_err?,
            (_, write_err @ Err(FromEnvErrorInner::NotAPipe(..))) => write_err?,
            (read_err, write_err) => {
                read_err?;
                write_err?;
            }
        }

        drop(set_cloexec(read, true));
        drop(set_cloexec(write, true));
        Ok(Some(Client::from_fds(read, write)))
    }

    unsafe fn from_fds(read: c_int, write: c_int) -> Client {
        Client::Pipe {
            read: File::from_raw_fd(read),
            write: File::from_raw_fd(write),
        }
    }

    /// Gets the read end of our jobserver client.
    fn read(&self) -> &File {
        match self {
            Client::Pipe { read, .. } => read,
            Client::Fifo { file, .. } => file,
        }
    }

    /// Gets the write end of our jobserver client.
    fn write(&self) -> &File {
        match self {
            Client::Pipe { write, .. } => write,
            Client::Fifo { file, .. } => file,
        }
    }

    pub fn acquire(&self) -> io::Result<Acquired> {
        // Ignore interrupts and keep trying if that happens
        loop {
            if let Some(token) = self.acquire_allow_interrupts()? {
                return Ok(token);
            }
        }
    }

    /// Block waiting for a token, returning `None` if we're interrupted with
    /// EINTR.
    fn acquire_allow_interrupts(&self) -> io::Result<Option<Acquired>> {
        // We don't actually know if the file descriptor here is set in
        // blocking or nonblocking mode. AFAIK all released versions of
        // `make` use blocking fds for the jobserver, but the unreleased
        // version of `make` doesn't. In the unreleased version jobserver
        // fds are set to nonblocking and combined with `pselect`
        // internally.
        //
        // Here we try to be compatible with both strategies. We optimistically
        // try to read from the file descriptor which then may block, return
        // a token or indicate that polling is needed.
        // Blocking reads (if possible) allows the kernel to be more selective
        // about which readers to wake up when a token is written to the pipe.
        //
        // We use `poll` here to block this thread waiting for read
        // readiness, and then afterwards we perform the `read` itself. If
        // the `read` returns that it would block then we start over and try
        // again.
        //
        // Also note that we explicitly don't handle EINTR here. That's used
        // to shut us down, so we otherwise punt all errors upwards.
        unsafe {
            let mut fd: libc::pollfd = mem::zeroed();
            let mut read = self.read();
            fd.fd = read.as_raw_fd();
            fd.events = libc::POLLIN;
            loop {
                let mut buf = [0];
                match read.read(&mut buf) {
                    Ok(1) => return Ok(Some(Acquired { byte: buf[0] })),
                    Ok(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "early EOF on jobserver pipe",
                        ));
                    }
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => { /* fall through to polling */ }
                        io::ErrorKind::Interrupted => return Ok(None),
                        _ => return Err(e),
                    },
                }

                loop {
                    fd.revents = 0;
                    if libc::poll(&mut fd, 1, -1) == -1 {
                        let e = io::Error::last_os_error();
                        return match e.kind() {
                            io::ErrorKind::Interrupted => Ok(None),
                            _ => Err(e),
                        };
                    }
                    if fd.revents != 0 {
                        break;
                    }
                }
            }
        }
    }

    pub fn release(&self, data: Option<&Acquired>) -> io::Result<()> {
        // Note that the fd may be nonblocking but we're going to go ahead
        // and assume that the writes here are always nonblocking (we can
        // always quickly release a token). If that turns out to not be the
        // case we'll get an error anyway!
        let byte = data.map(|d| d.byte).unwrap_or(b'+');
        match self.write().write(&[byte])? {
            1 => Ok(()),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to write token back to jobserver",
            )),
        }
    }

    pub fn string_arg(&self) -> String {
        match self {
            Client::Pipe { read, write } => format!("{},{}", read.as_raw_fd(), write.as_raw_fd()),
            Client::Fifo { path, .. } => format!("fifo:{}", path.to_str().unwrap()),
        }
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut len = MaybeUninit::<c_int>::uninit();
        cvt(unsafe { libc::ioctl(self.read().as_raw_fd(), libc::FIONREAD, len.as_mut_ptr()) })?;
        Ok(unsafe { len.assume_init() } as usize)
    }

    pub fn configure(&self, cmd: &mut Command) {
        match self {
            // We `File::open`ed it when inheriting from environment,
            // so no need to set cloexec for fifo.
            Client::Fifo { .. } => return,
            Client::Pipe { .. } => {}
        };
        // Here we basically just want to say that in the child process
        // we'll configure the read/write file descriptors to *not* be
        // cloexec, so they're inherited across the exec and specified as
        // integers through `string_arg` above.
        let read = self.read().as_raw_fd();
        let write = self.write().as_raw_fd();
        unsafe {
            cmd.pre_exec(move || {
                set_cloexec(read, false)?;
                set_cloexec(write, false)?;
                Ok(())
            });
        }
    }
}

#[derive(Debug)]
pub struct Helper {
    thread: JoinHandle<()>,
    state: Arc<super::HelperState>,
}

pub(crate) fn spawn_helper(
    client: crate::Client,
    state: Arc<super::HelperState>,
    mut f: Box<dyn FnMut(io::Result<crate::Acquired>) + Send>,
) -> io::Result<Helper> {
    static USR1_INIT: Once = Once::new();
    let mut err = None;
    USR1_INIT.call_once(|| unsafe {
        let mut new: libc::sigaction = mem::zeroed();
        #[cfg(target_os = "aix")]
        {
            new.sa_union.__su_sigaction = sigusr1_handler;
        }
        #[cfg(not(target_os = "aix"))]
        {
            new.sa_sigaction = sigusr1_handler as usize;
        }
        new.sa_flags = libc::SA_SIGINFO as _;
        if libc::sigaction(libc::SIGUSR1, &new, ptr::null_mut()) != 0 {
            err = Some(io::Error::last_os_error());
        }
    });

    if let Some(e) = err.take() {
        return Err(e);
    }

    let state2 = state.clone();
    let thread = Builder::new().spawn(move || {
        state2.for_each_request(|helper| loop {
            match client.inner.acquire_allow_interrupts() {
                Ok(Some(data)) => {
                    break f(Ok(crate::Acquired {
                        client: client.inner.clone(),
                        data,
                        disabled: false,
                    }));
                }
                Err(e) => break f(Err(e)),
                Ok(None) if helper.producer_done() => break,
                Ok(None) => {}
            }
        });
    })?;

    Ok(Helper { thread, state })
}

impl Helper {
    pub fn join(self) {
        let dur = Duration::from_millis(10);
        let mut state = self.state.lock();
        debug_assert!(state.producer_done);

        // We need to join our helper thread, and it could be blocked in one
        // of two locations. First is the wait for a request, but the
        // initial drop of `HelperState` will take care of that. Otherwise
        // it may be blocked in `client.acquire()`. We actually have no way
        // of interrupting that, so resort to `pthread_kill` as a fallback.
        // This signal should interrupt any blocking `read` call with
        // `io::ErrorKind::Interrupt` and cause the thread to cleanly exit.
        //
        // Note that we don't do this forever though since there's a chance
        // of bugs, so only do this opportunistically to make a best effort
        // at clearing ourselves up.
        for _ in 0..100 {
            if state.consumer_done {
                break;
            }
            unsafe {
                // Ignore the return value here of `pthread_kill`,
                // apparently on OSX if you kill a dead thread it will
                // return an error, but on other platforms it may not. In
                // that sense we don't actually know if this will succeed or
                // not!
                libc::pthread_kill(self.thread.as_pthread_t() as _, libc::SIGUSR1);
            }
            state = self
                .state
                .cvar
                .wait_timeout(state, dur)
                .unwrap_or_else(|e| e.into_inner())
                .0;
            thread::yield_now(); // we really want the other thread to run
        }

        // If we managed to actually see the consumer get done, then we can
        // definitely wait for the thread. Otherwise it's... off in the ether
        // I guess?
        if state.consumer_done {
            drop(self.thread.join());
        }
    }
}

unsafe fn fcntl_check(fd: c_int) -> Result<(), FromEnvErrorInner> {
    match libc::fcntl(fd, libc::F_GETFD) {
        -1 => Err(FromEnvErrorInner::CannotOpenFd(
            fd,
            io::Error::last_os_error(),
        )),
        _ => Ok(()),
    }
}

unsafe fn fd_check(fd: c_int, check_pipe: bool) -> Result<(), FromEnvErrorInner> {
    if check_pipe {
        let mut stat = mem::zeroed();
        if libc::fstat(fd, &mut stat) == -1 {
            let last_os_error = io::Error::last_os_error();
            fcntl_check(fd)?;
            Err(FromEnvErrorInner::NotAPipe(fd, Some(last_os_error)))
        } else {
            // On android arm and i686 mode_t is u16 and st_mode is u32,
            // this generates a type mismatch when S_IFIFO (declared as mode_t)
            // is used in operations with st_mode, so we use this workaround
            // to get the value of S_IFIFO with the same type of st_mode.
            #[allow(unused_assignments)]
            let mut s_ififo = stat.st_mode;
            s_ififo = libc::S_IFIFO as _;
            if stat.st_mode & s_ififo == s_ififo {
                return Ok(());
            }
            Err(FromEnvErrorInner::NotAPipe(fd, None))
        }
    } else {
        fcntl_check(fd)
    }
}

fn set_cloexec(fd: c_int, set: bool) -> io::Result<()> {
    unsafe {
        let previous = cvt(libc::fcntl(fd, libc::F_GETFD))?;
        let new = if set {
            previous | libc::FD_CLOEXEC
        } else {
            previous & !libc::FD_CLOEXEC
        };
        if new != previous {
            cvt(libc::fcntl(fd, libc::F_SETFD, new))?;
        }
        Ok(())
    }
}

fn set_nonblocking(fd: c_int, set: bool) -> io::Result<()> {
    let status_flag = if set { libc::O_NONBLOCK } else { 0 };

    unsafe {
        cvt(libc::fcntl(fd, libc::F_SETFL, status_flag))?;
    }

    Ok(())
}

fn cvt(t: c_int) -> io::Result<c_int> {
    if t == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(t)
    }
}

extern "C" fn sigusr1_handler(
    _signum: c_int,
    _info: *mut libc::siginfo_t,
    _ptr: *mut libc::c_void,
) {
    // nothing to do
}
