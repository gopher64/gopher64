//! Helpers to handle primary selection related actions.
//!
//! If you're not using [`default_environment!`](../macro.default_environment.html) you should
//! call `get_primary_selection_manager` to bind proper primary selection manager.

use std::{cell::RefCell, rc::Rc};

use wayland_protocols::misc::gtk_primary_selection::client::gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager;
use wayland_protocols::unstable::primary_selection::v1::client::zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1;

use wayland_client::{
    protocol::{wl_registry::WlRegistry, wl_seat::WlSeat},
    Attached, DispatchData,
};

use crate::lazy_global::LazyGlobal;
use crate::seat::{SeatHandling, SeatListener};
use crate::{environment::GlobalHandler, MissingGlobal};

mod device;
mod offer;
mod source;

pub use self::device::PrimarySelectionDevice;
pub use self::offer::PrimarySelectionOffer;
pub use self::source::{PrimarySelectionSource, PrimarySelectionSourceEvent};

/// A handler for primary selection.
///
/// It provides automatic tracking of primary selection device for each available seat,
/// allowing you to manipulate the primary selection clipboard.
///
/// It's automatically included in the [`default_environment!`](../macro.default_environment.html).
#[derive(Debug)]
pub struct PrimarySelectionHandler {
    inner: Rc<RefCell<PrimarySelectionDeviceManagerInner>>,
    _listener: SeatListener,
}

/// Possible supported primary selection protocols
#[derive(Debug)]
pub enum PrimarySelectionDeviceManager {
    /// The current standard `primary_selection` protocol.
    Zwp(Attached<ZwpPrimarySelectionDeviceManagerV1>),
    /// The old `gtk_primary_selection` protocol, which is still used by GTK.
    Gtk(Attached<GtkPrimarySelectionDeviceManager>),
}

impl PrimarySelectionHandler {
    /// Initialize a primary selection handler.
    ///
    /// In requires the access to the seat handler in order to track the creation and removal of
    /// seats.
    pub fn init<S: SeatHandling>(seat_handler: &mut S) -> Self {
        let inner = Rc::new(RefCell::new(PrimarySelectionDeviceManagerInner {
            registry: None,
            zwp_mgr: LazyGlobal::Unknown,
            gtk_mgr: LazyGlobal::Unknown,
            state: PrimarySelectionDeviceManagerInitState::Pending { seats: Vec::new() },
        }));

        // Listen for a new seat events to add new primary selection devices on the fly.
        let seat_inner = inner.clone();
        let listener = seat_handler.listen(move |seat, seat_data, _| {
            if seat_data.defunct {
                seat_inner.borrow_mut().remove_seat(&seat);
            } else {
                seat_inner.borrow_mut().new_seat(&seat);
            }
        });

        Self { inner, _listener: listener }
    }
}

/// An interface trait to forward the primary selection device handler capability.
///
/// You need to implement this trait for your environment struct, by delegating it
/// to its `PrimarySelectionHandler` field in order to get the associated methods
/// on your [`Environment`](../environment/struct.environment.html).
pub trait PrimarySelectionHandling {
    /// Access the primary selection associated with a seat.
    ///
    /// Returns an error if the seat is not found (for example if it has since been removed by
    /// the server) or if the `zwp_primary_selection_device_manager_v1` or
    /// `gtk_primary_selection_device_manager` globals are missing.
    fn with_primary_selection<F: FnOnce(&PrimarySelectionDevice)>(
        &self,
        seat: &WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal>;

    /// Get the best available primary selection device manager protocol.
    ///
    /// Returns `None` if no primary selection device manager was advertised.
    fn get_primary_selection_manager(&self) -> Option<PrimarySelectionDeviceManager>;
}

impl<E: PrimarySelectionHandling> crate::environment::Environment<E> {
    /// Get the best available primary selection device manager protocol.
    ///
    /// Returns `None` if no primary selection device manager was advertised.
    pub fn get_primary_selection_manager(&self) -> Option<PrimarySelectionDeviceManager> {
        self.with_inner(|manager| manager.get_primary_selection_manager())
    }

    /// Access the primary selection associated with a seat.
    ///
    /// Returns an error if the seat is not found (for example if it has since been removed by
    /// the server) of if the `zwp_primary_selection_device_manager_v1` or
    /// `gtk_primary_selection_device_manager` globals are missing.
    pub fn with_primary_selection<F: FnOnce(&PrimarySelectionDevice)>(
        &self,
        seat: &WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        self.with_inner(|inner| inner.with_primary_selection(seat, f))
    }

    /// Create a new primary selection source.
    ///
    /// This primary selection source is the basic object for offering primary selection clipboard
    /// to other clients.
    ///
    /// Once this source is created, you will need to give it to a
    /// [`PrimarySelectionDevice`](../primary_selection/struct.PrimarySelectionDevice.html)
    /// to start interaction.
    pub fn new_primary_selection_source<F>(
        &self,
        mime_types: Vec<String>,
        callback: F,
    ) -> PrimarySelectionSource
    where
        F: FnMut(PrimarySelectionSourceEvent, DispatchData) + 'static,
    {
        let manager = match self.get_primary_selection_manager() {
            Some(manager) => manager,
            None => panic!("[SCTK] primary selection was required"),
        };

        PrimarySelectionSource::new(&manager, mime_types, callback)
    }
}

impl PrimarySelectionHandling for PrimarySelectionHandler {
    /// Get the best available primary selection device manager protocol.
    ///
    /// Returns `None` if no primary selection device manager was advertised.
    fn get_primary_selection_manager(&self) -> Option<PrimarySelectionDeviceManager> {
        GlobalHandler::<ZwpPrimarySelectionDeviceManagerV1>::get(self)
            .map(PrimarySelectionDeviceManager::Zwp)
            .or_else(|| {
                GlobalHandler::<GtkPrimarySelectionDeviceManager>::get(self)
                    .map(PrimarySelectionDeviceManager::Gtk)
            })
    }

    /// Access the primary selection associated with a seat.
    ///
    /// Returns an error if the seat is not found (for example if it has since been removed by
    /// the server) of if the `zwp_primary_selection_device_manager_v1` or
    /// `gtk_primary_selection_device_manager` globals are missing.
    fn with_primary_selection<F: FnOnce(&PrimarySelectionDevice)>(
        &self,
        seat: &WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        self.inner.borrow().with_primary_selection(seat, f)
    }
}

/// Initialization phase of `PrimarySelectionDeviceManagerInner`.
#[derive(Debug)]
enum PrimarySelectionDeviceManagerInitState {
    Ready { manager: PrimarySelectionDeviceManager, devices: Vec<(WlSeat, PrimarySelectionDevice)> },
    Pending { seats: Vec<WlSeat> },
}

/// Inner mutable state for `PrimarySelectionHandler`.
#[derive(Debug)]
struct PrimarySelectionDeviceManagerInner {
    registry: Option<Attached<WlRegistry>>,
    zwp_mgr: LazyGlobal<ZwpPrimarySelectionDeviceManagerV1>,
    gtk_mgr: LazyGlobal<GtkPrimarySelectionDeviceManager>,
    pub state: PrimarySelectionDeviceManagerInitState,
}

impl PrimarySelectionDeviceManagerInner {
    /// Initialize `PrimarySelectionDeviceManager` and setup `PrimarySelectionDevice` for a
    /// registered seats, if we have pending `PrimarySelectionDeviceManager`.
    fn init_selection_manager(&mut self, manager: PrimarySelectionDeviceManager) {
        let seats =
            if let PrimarySelectionDeviceManagerInitState::Pending { seats } = &mut self.state {
                std::mem::take(seats)
            } else {
                log::warn!("Ignoring second primary selection manager.");
                return;
            };

        let mut devices = Vec::new();

        // Create primary selection devices for each seat.
        for seat in seats {
            let device = PrimarySelectionDevice::init_for_seat(&manager, &seat);
            devices.push((seat.clone(), device));
        }

        // Mark the state as `Ready`, so we can use our primary selection manager.
        self.state = PrimarySelectionDeviceManagerInitState::Ready { devices, manager }
    }

    /// Handle addition of a new seat.
    fn new_seat(&mut self, seat: &WlSeat) {
        match &mut self.state {
            PrimarySelectionDeviceManagerInitState::Ready { devices, manager } => {
                if devices.iter().any(|(s, _)| s == seat) {
                    // The seat already exists, nothing to do
                    return;
                }

                // Initialize primary selection device for a new seat.
                let device = PrimarySelectionDevice::init_for_seat(manager, seat);

                devices.push((seat.clone(), device));
            }
            PrimarySelectionDeviceManagerInitState::Pending { seats } => {
                seats.push(seat.clone());
            }
        }
    }

    /// Handle removal of a seat.
    fn remove_seat(&mut self, seat: &WlSeat) {
        match &mut self.state {
            PrimarySelectionDeviceManagerInitState::Ready { devices, .. } => {
                devices.retain(|(s, _)| s != seat)
            }
            PrimarySelectionDeviceManagerInitState::Pending { seats } => {
                seats.retain(|s| s != seat)
            }
        }
    }

    /// Access the primary selection associated with a seat.
    ///
    /// Returns an error if the seat is not found (for example if it has since been removed by
    /// the server) of if the `zwp_primary_selection_device_manager_v1` or
    /// `gtk_primary_selection_device_manager` globals are missing.
    fn with_primary_selection<F: FnOnce(&PrimarySelectionDevice)>(
        &self,
        seat: &WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        match &self.state {
            PrimarySelectionDeviceManagerInitState::Pending { .. } => Err(MissingGlobal),
            PrimarySelectionDeviceManagerInitState::Ready { devices, .. } => {
                for (s, device) in devices {
                    if s == seat {
                        f(device);
                        return Ok(());
                    }
                }

                Err(MissingGlobal)
            }
        }
    }
}

impl GlobalHandler<ZwpPrimarySelectionDeviceManagerV1> for PrimarySelectionHandler {
    fn created(&mut self, registry: Attached<WlRegistry>, id: u32, version: u32, _: DispatchData) {
        let mut inner = self.inner.borrow_mut();
        if inner.registry.is_none() {
            inner.registry = Some(registry);
        }

        if let LazyGlobal::Unknown = inner.zwp_mgr {
            // Mark global as seen.
            inner.zwp_mgr = LazyGlobal::Seen { id, version };
        } else {
            log::warn!(
                "Compositor advertised zwp_primary_selection_device_manager_v1 multiple\
                times, ignoring."
            )
        }
    }

    fn get(&self) -> Option<Attached<ZwpPrimarySelectionDeviceManagerV1>> {
        let mut inner = self.inner.borrow_mut();
        match inner.zwp_mgr {
            LazyGlobal::Bound(ref mgr) => Some(mgr.clone()),
            LazyGlobal::Unknown => None,
            LazyGlobal::Seen { id, version } => {
                // Registry cannot be `None` if we've seen the global.
                let registry = inner.registry.as_ref().unwrap();

                // Bind zwp primary selection.
                let version = std::cmp::min(1, version);
                let mgr = registry.bind::<ZwpPrimarySelectionDeviceManagerV1>(version, id);
                let manager = PrimarySelectionDeviceManager::Zwp((*mgr).clone());

                // Init zwp selection manager.
                inner.init_selection_manager(manager);

                inner.zwp_mgr = LazyGlobal::Bound((*mgr).clone());
                Some((*mgr).clone())
            }
        }
    }
}

impl GlobalHandler<GtkPrimarySelectionDeviceManager> for PrimarySelectionHandler {
    fn created(&mut self, registry: Attached<WlRegistry>, id: u32, version: u32, _: DispatchData) {
        let mut inner = self.inner.borrow_mut();
        if inner.registry.is_none() {
            inner.registry = Some(registry);
        }
        if let LazyGlobal::Unknown = inner.gtk_mgr {
            // Mark global as seen.
            inner.gtk_mgr = LazyGlobal::Seen { id, version };
        } else {
            log::warn!(
                "Compositor advertised gtk_primary_selection_device_manager multiple times,\
                ignoring."
            )
        }
    }

    fn get(&self) -> Option<Attached<GtkPrimarySelectionDeviceManager>> {
        let mut inner = self.inner.borrow_mut();
        match inner.gtk_mgr {
            LazyGlobal::Bound(ref mgr) => Some(mgr.clone()),
            LazyGlobal::Unknown => None,
            LazyGlobal::Seen { id, version } => {
                // Registry cannot be `None` if we've seen the global.
                let registry = inner.registry.as_ref().unwrap();

                // Bind gtk primary selection.
                let version = std::cmp::min(1, version);
                let mgr = registry.bind::<GtkPrimarySelectionDeviceManager>(version, id);
                let manager = PrimarySelectionDeviceManager::Gtk((*mgr).clone());

                // Init gtk selection manager.
                inner.init_selection_manager(manager);

                inner.gtk_mgr = LazyGlobal::Bound((*mgr).clone());
                Some((*mgr).clone())
            }
        }
    }
}
