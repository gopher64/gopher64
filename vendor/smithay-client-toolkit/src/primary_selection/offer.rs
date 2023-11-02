use std::os::unix::io::FromRawFd;
use std::sync::{Arc, Mutex};

use wayland_client::Main;

use wayland_protocols::{
    misc::gtk_primary_selection::client::gtk_primary_selection_offer::{
        self, GtkPrimarySelectionOffer,
    },
    unstable::primary_selection::v1::client::zwp_primary_selection_offer_v1::{
        self, ZwpPrimarySelectionOfferV1,
    },
};

use crate::data_device::ReadPipe;

/// A primary selection offer for receiving data through copy/paste.
#[derive(Debug)]
pub struct PrimarySelectionOffer {
    pub(crate) offer: PrimarySelectionOfferImpl,
    inner: Arc<Mutex<PrimarySelectionOfferInner>>,
}

impl PrimarySelectionOffer {
    /// Access the list of mime types proposed by this offer.
    pub fn with_mime_types<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&[String]) -> T,
    {
        let inner = self.inner.lock().unwrap();
        f(&inner.mime_types)
    }

    /// Request to receive the data of a given mime type.
    ///
    /// Note that you should **not** read the contents right away in a blocking way,
    /// as you may deadlock your application.
    pub fn receive(&self, mime_type: String) -> Result<ReadPipe, std::io::Error> {
        use nix::fcntl::OFlag;
        use nix::unistd::{close, pipe2};
        // create a pipe
        let (readfd, writefd) = pipe2(OFlag::O_CLOEXEC)?;

        match &self.offer {
            PrimarySelectionOfferImpl::Zwp(offer) => {
                offer.receive(mime_type, writefd);
            }
            PrimarySelectionOfferImpl::Gtk(offer) => {
                offer.receive(mime_type, writefd);
            }
        }

        if let Err(err) = close(writefd) {
            log::warn!("Failed to close write pipe: {}", err);
        }

        Ok(unsafe { FromRawFd::from_raw_fd(readfd) })
    }

    /// Initialize `PrimarySelectionOffer` from the `Zwp` offer.
    pub(crate) fn from_zwp(offer: Main<ZwpPrimarySelectionOfferV1>) -> Self {
        let inner = Arc::new(Mutex::new(PrimarySelectionOfferInner::new()));
        let inner2 = inner.clone();

        offer.quick_assign(move |_, event, _| {
            use zwp_primary_selection_offer_v1::Event;
            let mut inner = inner2.lock().unwrap();
            match event {
                Event::Offer { mime_type } => {
                    inner.mime_types.push(mime_type);
                }
                _ => unreachable!(),
            }
        });

        Self { offer: PrimarySelectionOfferImpl::Zwp(offer.detach()), inner }
    }

    /// Initialize `PrimarySelectionOffer` from the `Gtk` offer.
    pub(crate) fn from_gtk(offer: Main<GtkPrimarySelectionOffer>) -> Self {
        let inner = Arc::new(Mutex::new(PrimarySelectionOfferInner::new()));
        let inner2 = inner.clone();

        offer.quick_assign(move |_, event, _| {
            use gtk_primary_selection_offer::Event;
            let mut inner = inner2.lock().unwrap();
            match event {
                Event::Offer { mime_type } => {
                    inner.mime_types.push(mime_type);
                }
                _ => unreachable!(),
            }
        });

        Self { offer: PrimarySelectionOfferImpl::Gtk(offer.detach()), inner }
    }
}

impl Drop for PrimarySelectionOffer {
    fn drop(&mut self) {
        match &self.offer {
            PrimarySelectionOfferImpl::Zwp(offer) => offer.destroy(),
            PrimarySelectionOfferImpl::Gtk(offer) => offer.destroy(),
        }
    }
}

/// Inner state for `PrimarySelectionOffer`.
#[derive(Debug, Default)]
struct PrimarySelectionOfferInner {
    mime_types: Vec<String>,
}

impl PrimarySelectionOfferInner {
    fn new() -> Self {
        Self::default()
    }
}

/// Possible supported primary selection offers.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PrimarySelectionOfferImpl {
    Zwp(ZwpPrimarySelectionOfferV1),
    Gtk(GtkPrimarySelectionOffer),
}
