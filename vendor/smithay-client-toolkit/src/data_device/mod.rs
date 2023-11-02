//! Helpers to handle data device related actions

use std::{cell::RefCell, fmt, rc::Rc};

use wayland_client::{
    protocol::{wl_data_device_manager, wl_registry, wl_seat},
    Attached, DispatchData,
};

pub use wayland_client::protocol::wl_data_device_manager::DndAction;

use crate::MissingGlobal;

mod device;
mod offer;
mod source;

pub use self::device::{DataDevice, DndEvent};
pub use self::offer::{DataOffer, ReadPipe};
pub use self::source::{DataSource, DataSourceEvent, WritePipe};

type DDCallback = dyn FnMut(wl_seat::WlSeat, DndEvent, DispatchData);

enum DDInner {
    Ready {
        mgr: Attached<wl_data_device_manager::WlDataDeviceManager>,
        devices: Vec<(wl_seat::WlSeat, DataDevice)>,
        callback: Rc<RefCell<Box<DDCallback>>>,
    },
    Pending {
        seats: Vec<wl_seat::WlSeat>,
    },
}

impl DDInner {
    fn init_dd_mgr(&mut self, mgr: Attached<wl_data_device_manager::WlDataDeviceManager>) {
        let seats = if let DDInner::Pending { seats } = self {
            ::std::mem::take(seats)
        } else {
            log::warn!("Ignoring second wl_data_device_manager.");
            return;
        };

        let mut devices = Vec::new();

        let callback = Rc::new(RefCell::new(Box::new(|_, _: DndEvent, _: DispatchData| {})
            as Box<dyn FnMut(_, DndEvent, DispatchData)>));

        for seat in seats {
            let cb = callback.clone();
            let my_seat = seat.clone();
            let device = DataDevice::init_for_seat(&mgr, &seat, move |event, dispatch_data| {
                (cb.borrow_mut())(my_seat.clone(), event, dispatch_data);
            });
            devices.push((seat.clone(), device));
        }

        *self = DDInner::Ready { mgr, devices, callback };
    }

    // A potential new seat is seen
    //
    // should do nothing if the seat is already known
    fn new_seat(&mut self, seat: &wl_seat::WlSeat) {
        match self {
            DDInner::Ready { mgr, devices, callback } => {
                if devices.iter().any(|(s, _)| s == seat) {
                    // the seat already exists, nothing to do
                    return;
                }
                let cb = callback.clone();
                let my_seat = seat.clone();
                let device = DataDevice::init_for_seat(mgr, seat, move |event, dispatch_data| {
                    (cb.borrow_mut())(my_seat.clone(), event, dispatch_data);
                });
                devices.push((seat.clone(), device));
            }
            DDInner::Pending { seats } => {
                seats.push(seat.clone());
            }
        }
    }

    fn remove_seat(&mut self, seat: &wl_seat::WlSeat) {
        match self {
            DDInner::Ready { devices, .. } => devices.retain(|(s, _)| s != seat),
            DDInner::Pending { seats } => seats.retain(|s| s != seat),
        }
    }

    fn get_mgr(&self) -> Option<Attached<wl_data_device_manager::WlDataDeviceManager>> {
        match self {
            DDInner::Ready { mgr, .. } => Some(mgr.clone()),
            DDInner::Pending { .. } => None,
        }
    }

    fn set_callback<F: FnMut(wl_seat::WlSeat, DndEvent, DispatchData) + 'static>(
        &mut self,
        cb: F,
    ) -> Result<(), MissingGlobal> {
        match self {
            DDInner::Ready { callback, .. } => {
                *(callback.borrow_mut()) = Box::new(cb);
                Ok(())
            }
            DDInner::Pending { .. } => Err(MissingGlobal),
        }
    }

    fn with_device<F: FnOnce(&DataDevice)>(
        &self,
        seat: &wl_seat::WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        match self {
            DDInner::Pending { .. } => Err(MissingGlobal),
            DDInner::Ready { devices, .. } => {
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

impl fmt::Debug for DDInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready { mgr, devices, .. } => f
                .debug_struct("Ready")
                .field("mgr", mgr)
                .field("devices", devices)
                .field("callback", &"Fn() -> { ... }")
                .finish(),
            Self::Pending { seats } => f.debug_struct("Pending").field("seats", seats).finish(),
        }
    }
}

/// A handler for data devices
///
/// It provides automatic tracking of data device for each available seat,
/// allowing you to manipulate selection clipboard and drag&drop manipulations.
///
/// It is automatically included in the [`default_environment!`](../macro.default_environment.html).
#[derive(Debug)]
pub struct DataDeviceHandler {
    inner: Rc<RefCell<DDInner>>,
    _listener: crate::seat::SeatListener,
}

impl DataDeviceHandler {
    /// Initialize a data device handler
    ///
    /// It needs access to a seat handler in order to track
    /// the creation and removal of seats.
    pub fn init<S>(seat_handler: &mut S) -> DataDeviceHandler
    where
        S: crate::seat::SeatHandling,
    {
        let inner = Rc::new(RefCell::new(DDInner::Pending { seats: Vec::new() }));

        let seat_inner = inner.clone();
        let listener = seat_handler.listen(move |seat, seat_data, _| {
            if seat_data.defunct {
                seat_inner.borrow_mut().remove_seat(&seat);
            } else {
                seat_inner.borrow_mut().new_seat(&seat)
            }
        });

        DataDeviceHandler { inner, _listener: listener }
    }
}

impl crate::environment::GlobalHandler<wl_data_device_manager::WlDataDeviceManager>
    for DataDeviceHandler
{
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        // data device manager is supported until version 3
        let version = std::cmp::min(version, 3);
        let ddmgr = registry.bind::<wl_data_device_manager::WlDataDeviceManager>(version, id);
        self.inner.borrow_mut().init_dd_mgr((*ddmgr).clone());
    }
    fn get(&self) -> Option<Attached<wl_data_device_manager::WlDataDeviceManager>> {
        self.inner.borrow().get_mgr()
    }
}

/// An interface trait to forward the data device handler capability
///
/// You need to implement this trait for your environment struct, by
/// delegating it to its `DataDeviceHandler` field in order to get the
/// associated methods on your [`Environment`](../environment/struct.environment.html).
pub trait DataDeviceHandling {
    /// Set the global drag'n'drop callback
    ///
    /// Returns an error if the `wl_data_device_manager` global is missing.
    fn set_callback<F: FnMut(wl_seat::WlSeat, DndEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<(), MissingGlobal>;

    /// Access the data device associated with a seat
    ///
    /// Returns an error if the seat is not found (for example if it has since been removed by
    /// the server) or if the `wl_data_device_manager` global is missing.
    fn with_device<F: FnOnce(&DataDevice)>(
        &self,
        seat: &wl_seat::WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal>;
}

impl DataDeviceHandling for DataDeviceHandler {
    fn set_callback<F: FnMut(wl_seat::WlSeat, DndEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<(), MissingGlobal> {
        self.inner.borrow_mut().set_callback(callback)
    }

    fn with_device<F: FnOnce(&DataDevice)>(
        &self,
        seat: &wl_seat::WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        self.inner.borrow().with_device(seat, f)
    }
}

impl<E> crate::environment::Environment<E>
where
    E: crate::environment::GlobalHandler<wl_data_device_manager::WlDataDeviceManager>,
{
    /// Create a new data source
    ///
    /// This data source is the basic object for offering content to other clients,
    /// be it for clipboard selection or as drag'n'drop content.
    ///
    /// Once this source is created, you will need to give it to a
    /// [`DataDevice`](../data_device/struct.DataDevice.html)
    /// to start interaction.
    pub fn new_data_source<F>(&self, mime_types: Vec<String>, callback: F) -> DataSource
    where
        F: FnMut(DataSourceEvent, DispatchData) + 'static,
    {
        let ddmgr = self.require_global::<wl_data_device_manager::WlDataDeviceManager>();
        DataSource::new(&ddmgr, mime_types, callback)
    }
}

impl<E> crate::environment::Environment<E>
where
    E: DataDeviceHandling,
{
    /// Set the data device callback
    ///
    /// This callback will be invoked whenever some drag'n'drop action is done onto one of
    /// your surfaces.
    ///
    /// You should set it before entering your main loop, to ensure you will not miss any events.
    ///
    /// Returns an error if the compositor did not advertise a data device capability.
    pub fn set_data_device_callback<F: FnMut(wl_seat::WlSeat, DndEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<(), MissingGlobal> {
        self.with_inner(|inner| inner.set_callback(callback))
    }

    /// Access the data device associated with a seat
    ///
    /// Returns an error if the seat is not found (for example if it has since been removed by
    /// the server) or if the `wl_data_device_manager` global is missing.
    pub fn with_data_device<F: FnOnce(&DataDevice)>(
        &self,
        seat: &wl_seat::WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        self.with_inner(|inner| inner.with_device(seat, f))
    }
}
