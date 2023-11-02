use std::{
    fs, io,
    os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    sync::{Arc, Mutex},
};

use wayland_client::protocol::wl_data_device_manager::DndAction;
use wayland_client::protocol::wl_data_offer;
use wayland_client::Main;

#[derive(Debug)]
struct Inner {
    mime_types: Vec<String>,
    actions: DndAction,
    current_action: DndAction,
    serial: u32,
}

/// A data offer for receiving data though copy/paste or
/// drag and drop
#[derive(Debug)]
pub struct DataOffer {
    pub(crate) offer: wl_data_offer::WlDataOffer,
    inner: Arc<Mutex<Inner>>,
}

impl DataOffer {
    pub(crate) fn new(offer: Main<wl_data_offer::WlDataOffer>) -> DataOffer {
        let inner = Arc::new(Mutex::new(Inner {
            mime_types: Vec::new(),
            actions: DndAction::None,
            current_action: DndAction::None,
            serial: 0,
        }));
        let inner2 = inner.clone();
        offer.quick_assign(move |_, event, _| {
            use self::wl_data_offer::Event;
            let mut inner = inner2.lock().unwrap();
            match event {
                Event::Offer { mime_type } => {
                    inner.mime_types.push(mime_type);
                }
                Event::SourceActions { source_actions } => {
                    inner.actions = source_actions;
                }
                Event::Action { dnd_action } => {
                    inner.current_action = dnd_action;
                }
                _ => unreachable!(),
            }
        });

        DataOffer { offer: offer.detach(), inner }
    }

    /// Access the list of mime types proposed by this offer
    pub fn with_mime_types<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&[String]) -> T,
    {
        let inner = self.inner.lock().unwrap();
        f(&inner.mime_types)
    }

    /// Get the list of available actions for this offer
    pub fn get_available_actions(&self) -> DndAction {
        self.inner.lock().unwrap().actions
    }

    /// Get the currently set final action for this offer
    pub fn get_current_action(&self) -> DndAction {
        self.inner.lock().unwrap().current_action
    }

    /// Accept a mime type for receiving data through this offer
    pub fn accept(&self, mime_type: Option<String>) {
        let serial = self.inner.lock().unwrap().serial;
        self.offer.accept(serial, mime_type);
    }

    /// Request to receive the data of a given mime type
    ///
    /// You can do this several times, as a reaction to motion of
    /// the dnd cursor, or to inspect the data in order to choose your
    /// response.
    ///
    /// Note that you should *not* read the contents right away in a
    /// blocking way, as you may deadlock your application doing so.
    /// At least make sure you flush your events to the server before
    /// doing so.
    ///
    /// Fails if too many file descriptors were already open and a pipe
    /// could not be created.
    pub fn receive(&self, mime_type: String) -> std::io::Result<ReadPipe> {
        use nix::fcntl::OFlag;
        use nix::unistd::{close, pipe2};
        // create a pipe
        let (readfd, writefd) = pipe2(OFlag::O_CLOEXEC)?;

        self.offer.receive(mime_type, writefd);

        if let Err(err) = close(writefd) {
            log::warn!("Failed to close write pipe: {}", err);
        }

        Ok(unsafe { FromRawFd::from_raw_fd(readfd) })
    }

    /// Receive data to the write end of a raw file descriptor. If you have the read end, you can read from it.
    ///
    /// You can do this several times, as a reaction to motion of
    /// the dnd cursor, or to inspect the data in order to choose your
    /// response.
    ///
    /// Note that you should *not* read the contents right away in a
    /// blocking way, as you may deadlock your application doing so.
    /// At least make sure you flush your events to the server before
    /// doing so.
    ///
    /// # Safety
    ///
    /// The provided file destructor must be a valid FD for writing, and will be closed
    /// once the contents are written.
    pub unsafe fn receive_to_fd(&self, mime_type: String, writefd: RawFd) {
        use nix::unistd::close;

        self.offer.receive(mime_type, writefd);

        if let Err(err) = close(writefd) {
            log::warn!("Failed to close write pipe: {}", err);
        }
    }

    /// Notify the send and compositor of the dnd actions you accept
    ///
    /// You need to provide the set of supported actions, as well as
    /// a single preferred action.
    pub fn set_actions(&self, supported: DndAction, preferred: DndAction) {
        self.offer.set_actions(supported, preferred);
    }

    /// Notify that you are finished with this offer, and will no longer
    /// be using it
    ///
    /// Note that it is a protocol error to finish if no action or mime
    /// type was accepted.
    pub fn finish(&self) {
        self.offer.finish();
        self.offer.destroy();
    }
}

impl Drop for DataOffer {
    fn drop(&mut self) {
        self.offer.destroy();
    }
}

/// A file descriptor that can only be read from
///
/// If the `calloop` cargo feature is enabled, this can be used
/// as an `EventSource` in a calloop event loop.
#[derive(Debug)]
pub struct ReadPipe {
    #[cfg(feature = "calloop")]
    file: calloop::generic::Generic<fs::File>,
    #[cfg(not(feature = "calloop"))]
    file: fs::File,
}

#[cfg(feature = "calloop")]
impl io::Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.file.read(buf)
    }
}

#[cfg(not(feature = "calloop"))]
impl io::Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

#[cfg(feature = "calloop")]
impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe {
            file: calloop::generic::Generic::new(
                FromRawFd::from_raw_fd(fd),
                calloop::Interest::READ,
                calloop::Mode::Level,
            ),
        }
    }
}

#[cfg(not(feature = "calloop"))]
impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe { file: FromRawFd::from_raw_fd(fd) }
    }
}

#[cfg(feature = "calloop")]
impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.file.as_raw_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.file.into_raw_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl calloop::EventSource for ReadPipe {
    type Event = ();
    type Error = std::io::Error;
    type Metadata = fs::File;
    type Ret = ();

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<calloop::PostAction>
    where
        F: FnMut((), &mut fs::File),
    {
        self.file.process_events(readiness, token, |_, file| {
            callback((), file);
            Ok(calloop::PostAction::Continue)
        })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.file.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.file.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.file.unregister(poll)
    }
}
