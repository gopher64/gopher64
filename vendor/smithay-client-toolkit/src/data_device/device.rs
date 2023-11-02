use wayland_client::{
    protocol::{wl_data_device, wl_data_device_manager, wl_data_offer, wl_seat, wl_surface},
    DispatchData, Main,
};

use std::sync::{Arc, Mutex};

use super::{DataOffer, DataSource, DndAction};

#[derive(Debug)]
struct Inner {
    selection: Option<DataOffer>,
    current_dnd: Option<DataOffer>,
    known_offers: Vec<DataOffer>,
}

impl Inner {
    fn new_offer(&mut self, offer: Main<wl_data_offer::WlDataOffer>) {
        self.known_offers.push(DataOffer::new(offer));
    }

    fn set_selection(&mut self, offer: Option<wl_data_offer::WlDataOffer>) {
        if let Some(offer) = offer {
            if let Some(id) = self.known_offers.iter().position(|o| o.offer == offer) {
                self.selection = Some(self.known_offers.swap_remove(id));
            } else {
                panic!("Compositor set an unknown data_offer for selection.");
            }
        } else {
            // drop the current offer if any
            self.selection = None;
        }
    }

    fn set_dnd(&mut self, offer: Option<wl_data_offer::WlDataOffer>) {
        if let Some(offer) = offer {
            if let Some(id) = self.known_offers.iter().position(|o| o.offer == offer) {
                self.current_dnd = Some(self.known_offers.swap_remove(id));
            } else {
                panic!("Compositor set an unknown data_offer for selection.");
            }
        } else {
            // drop the current offer if any
            self.current_dnd = None;
        }
    }
}

/// Handle to support data exchange on a given seat
///
/// This type provides you with functionality to send and receive
/// data through drag'n'drop or copy/paste actions. It is associated
/// with a seat upon creation.
#[derive(Debug)]
pub struct DataDevice {
    device: wl_data_device::WlDataDevice,
    inner: Arc<Mutex<Inner>>,
}

/// Possible events generated during a drag'n'drop session
#[derive(Debug)]
pub enum DndEvent<'a> {
    /// A new drag'n'drop entered your surfaces
    Enter {
        /// The associated data offer
        ///
        /// Is None if it is an internal drag'n'drop you started with
        /// no source. See `DataDevice::start_drag` for details.
        offer: Option<&'a DataOffer>,
        /// A serial associated with the entry of this dnd
        serial: u32,
        /// The entered surface
        surface: wl_surface::WlSurface,
        /// horizontal location on the surface
        x: f64,
        /// vertical location on the surface
        y: f64,
    },
    /// The drag'n'drop offer moved on the surface
    Motion {
        /// The associated data offer
        ///
        /// Is None if it is an internal drag'n'drop you started with
        /// no source. See `DataDevice::start_drag` for details.
        offer: Option<&'a DataOffer>,
        /// The time of this motion
        time: u32,
        /// new horizontal location
        x: f64,
        /// new vertical location
        y: f64,
    },
    /// The drag'n'drop offer left your surface
    Leave,
    /// The drag'n'drop was dropped on your surface
    Drop {
        /// The associated data offer
        ///
        /// Is None if it is an internal drag'n'drop you started with
        /// no source. See `DataDevice::start_drag` for details.
        offer: Option<&'a DataOffer>,
    },
}

fn data_device_implem<F>(
    event: wl_data_device::Event,
    inner: &mut Inner,
    implem: &mut F,
    ddata: DispatchData,
) where
    for<'a> F: FnMut(DndEvent<'a>, DispatchData),
{
    use self::wl_data_device::Event;

    match event {
        Event::DataOffer { id } => inner.new_offer(id),
        Event::Enter { serial, surface, x, y, id } => {
            inner.set_dnd(id);
            implem(
                DndEvent::Enter { serial, surface, x, y, offer: inner.current_dnd.as_ref() },
                ddata,
            );
        }
        Event::Motion { time, x, y } => {
            implem(DndEvent::Motion { x, y, time, offer: inner.current_dnd.as_ref() }, ddata);
        }
        Event::Leave => implem(DndEvent::Leave, ddata),
        Event::Drop => {
            implem(DndEvent::Drop { offer: inner.current_dnd.as_ref() }, ddata);
        }
        Event::Selection { id } => inner.set_selection(id),
        _ => unreachable!(),
    }
}

impl DataDevice {
    /// Create the DataDevice helper for this seat.
    ///
    /// You need to provide an implementation that will handle drag'n'drop
    /// events.
    pub fn init_for_seat<F>(
        manager: &wl_data_device_manager::WlDataDeviceManager,
        seat: &wl_seat::WlSeat,
        mut callback: F,
    ) -> DataDevice
    where
        for<'a> F: FnMut(DndEvent<'a>, DispatchData) + 'static,
    {
        let inner = Arc::new(Mutex::new(Inner {
            selection: None,
            current_dnd: None,
            known_offers: Vec::new(),
        }));

        let inner2 = inner.clone();
        let device = manager.get_data_device(seat);
        device.quick_assign(move |_, evt, ddata| {
            let mut inner = inner2.lock().unwrap();
            data_device_implem(evt, &mut *inner, &mut callback, ddata);
        });

        DataDevice { device: device.detach(), inner }
    }

    /// Start a drag'n'drop offer
    ///
    /// You need to specify the origin surface, as well a serial associated
    /// to an implicit grab on this surface (for example received by a pointer click).
    ///
    /// An optional `DataSource` can be provided. If it is `None`, this drag'n'drop will
    /// be considered as internal to your application, and other applications will not be
    /// notified of it. You are then responsible for acting accordingly on drop.
    ///
    /// You also need to specify which possible drag'n'drop actions are associated to this
    /// drag (copy, move, or ask), the final action will be chosen by the target and/or
    /// compositor.
    ///
    /// You can finally provide a surface that will be used as an icon associated with
    /// this drag'n'drop for user visibility.
    pub fn start_drag(
        &self,
        origin: &wl_surface::WlSurface,
        source: Option<DataSource>,
        actions: DndAction,
        icon: Option<&wl_surface::WlSurface>,
        serial: u32,
    ) {
        if let Some(source) = source {
            source.source.set_actions(actions);
            self.device.start_drag(Some(&source.source), origin, icon, serial);
        } else {
            self.device.start_drag(None, origin, icon, serial);
        }
    }

    /// Provide a data source as the new content for the selection
    ///
    /// Correspond to traditional copy/paste behavior. Setting the
    /// source to `None` will clear the selection.
    pub fn set_selection(&self, source: &Option<DataSource>, serial: u32) {
        self.device.set_selection(source.as_ref().map(|s| &s.source), serial);
    }

    /// Access the `DataOffer` currently associated with the selection buffer
    pub fn with_selection<F, T>(&self, f: F) -> T
    where
        F: FnOnce(Option<&DataOffer>) -> T,
    {
        let inner = self.inner.lock().unwrap();
        f(inner.selection.as_ref())
    }

    /// Access the `DataOffer` currently associated with current DnD
    pub fn with_dnd<F, T>(&self, f: F) -> T
    where
        F: FnOnce(Option<&DataOffer>) -> T,
    {
        let inner = self.inner.lock().unwrap();
        f(inner.current_dnd.as_ref())
    }
}

impl Drop for DataDevice {
    fn drop(&mut self) {
        self.device.release();
    }
}
