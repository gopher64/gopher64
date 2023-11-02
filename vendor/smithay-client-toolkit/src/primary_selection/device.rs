use std::sync::{Arc, Mutex};

use wayland_protocols::{
    misc::gtk_primary_selection::client::gtk_primary_selection_device::{
        self, GtkPrimarySelectionDevice,
    },
    unstable::primary_selection::v1::client::zwp_primary_selection_device_v1::{
        self, ZwpPrimarySelectionDeviceV1,
    },
};

use wayland_client::protocol::wl_seat::WlSeat;

use crate::primary_selection::offer::PrimarySelectionOfferImpl;
use crate::primary_selection::source::PrimarySelectionSourceImpl;

use super::PrimarySelectionDeviceManager;
use super::PrimarySelectionOffer;
use super::PrimarySelectionSource;

/// Handle to support primary selection on a given seat.
///
/// This type provides you with copy/paste actions. It is associated with a seat upon creation.
#[derive(Debug)]
pub struct PrimarySelectionDevice {
    device: PrimarySelectionDeviceImpl,
    inner: Arc<Mutex<PrimarySelectionDeviceInner>>,
}

/// Possible supported primary selection devices.
#[derive(Debug)]
enum PrimarySelectionDeviceImpl {
    Zwp(ZwpPrimarySelectionDeviceV1),
    Gtk(GtkPrimarySelectionDevice),
}

/// Inner state for `PrimarySelectionDevice`.
#[derive(Debug)]
struct PrimarySelectionDeviceInner {
    /// Current selection.
    selection: Option<PrimarySelectionOffer>,

    /// List of known offers.
    know_offers: Vec<PrimarySelectionOffer>,
}

impl PrimarySelectionDeviceInner {
    /// Provide a primary selection source as the new content for the primary selection.
    ///
    /// Correspond to traditional copy/paste behavior. Setting the source to `None` will clear
    /// the selection.
    fn set_selection(&mut self, offer: Option<PrimarySelectionOfferImpl>) {
        let offer = match offer {
            Some(offer) => offer,
            None => {
                // Drop the current offer if any.
                self.selection = None;
                return;
            }
        };

        if let Some(id) = self.know_offers.iter().position(|o| o.offer == offer) {
            self.selection = Some(self.know_offers.swap_remove(id));
        } else {
            panic!("Compositor set an unknown primary offer for a primary selection.")
        }
    }
}

impl Drop for PrimarySelectionDevice {
    fn drop(&mut self) {
        match self.device {
            PrimarySelectionDeviceImpl::Zwp(ref device) => device.destroy(),
            PrimarySelectionDeviceImpl::Gtk(ref device) => device.destroy(),
        }
    }
}

impl PrimarySelectionDevice {
    /// Create the `PrimarySelectionDevice` helper for this seat.
    pub fn init_for_seat(manager: &PrimarySelectionDeviceManager, seat: &WlSeat) -> Self {
        let inner = Arc::new(Mutex::new(PrimarySelectionDeviceInner {
            selection: None,
            know_offers: Vec::new(),
        }));

        let inner2 = inner.clone();

        let device = match manager {
            PrimarySelectionDeviceManager::Zwp(zwp_manager) => {
                let device = zwp_manager.get_device(seat);

                device.quick_assign(move |_, event, _| {
                    let mut inner = inner2.lock().unwrap();

                    use zwp_primary_selection_device_v1::Event;
                    match event {
                        Event::DataOffer { offer } => {
                            inner.know_offers.push(PrimarySelectionOffer::from_zwp(offer))
                        }
                        Event::Selection { id } => {
                            let id = id.map(PrimarySelectionOfferImpl::Zwp);
                            inner.set_selection(id);
                        }
                        _ => unreachable!(),
                    }
                });

                PrimarySelectionDeviceImpl::Zwp(device.detach())
            }
            PrimarySelectionDeviceManager::Gtk(gtk_manager) => {
                let device = gtk_manager.get_device(seat);

                device.quick_assign(move |_, event, _| {
                    let mut inner = inner2.lock().unwrap();

                    use gtk_primary_selection_device::Event;
                    match event {
                        Event::DataOffer { offer } => {
                            inner.know_offers.push(PrimarySelectionOffer::from_gtk(offer))
                        }
                        Event::Selection { id } => {
                            let id = id.map(PrimarySelectionOfferImpl::Gtk);
                            inner.set_selection(id);
                        }
                        _ => unreachable!(),
                    }
                });
                PrimarySelectionDeviceImpl::Gtk(device.detach())
            }
        };

        Self { device, inner }
    }

    /// Provide a primary selection source as the new content for the primary selection.
    ///
    /// Correspond to traditional copy/paste behavior. Setting the source to `None` will clear
    /// the selection.
    pub fn set_selection(&self, source: &Option<PrimarySelectionSource>, serial: u32) {
        match self.device {
            PrimarySelectionDeviceImpl::Zwp(ref device) => {
                let source = source.as_ref().map(|source| match source.source {
                    PrimarySelectionSourceImpl::Zwp(ref source) => source,
                    // We can't reach `Gtk` source in `Zwp`.
                    _ => unreachable!(),
                });
                device.set_selection(source, serial);
            }
            PrimarySelectionDeviceImpl::Gtk(ref device) => {
                let source = source.as_ref().map(|source| match source.source {
                    PrimarySelectionSourceImpl::Gtk(ref source) => source,
                    // We can't reach `Zwp` source in `Gtk`.
                    _ => unreachable!(),
                });
                device.set_selection(source, serial);
            }
        }
    }

    /// Access the `PrimarySelectionOffer` currently associated with the primary selection buffer.
    pub fn with_selection<F: FnOnce(Option<&PrimarySelectionOffer>) -> T, T>(&self, f: F) -> T {
        let inner = self.inner.lock().unwrap();
        f(inner.selection.as_ref())
    }
}
