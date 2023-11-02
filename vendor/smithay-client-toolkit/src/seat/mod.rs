//! Types for automatically handling seats
//!
//! This modules provides a `SeatHandler` for use with the
//! [`environment!`](../macro.environment.html) macro. It is automatically inserted
//! in the [`default_environment!`](../macro.default_environment.html).
//!
//! This handler tracks the capability of the seats declared by the compositor,
//! and gives you the possibility to register callbacks that will be invoked whenever
//! a new seat is created of the state of a seat changes, via the
//! [`Environment::listen_for_seats`](../environment/struct.Environment.html) method.
//!
//! **Note:** if you don't use the [`default_environment!`](../macro.default_environment.html),
//! you'll need to implement the [`SeatHandling`](trait.SeatHandling.hmtl) on your
//! environment struct to access the added methods on
//! [`Environment`](../environment/struct.Environment.html).

use std::{
    cell::RefCell,
    fmt::{self, Debug, Formatter},
    rc::{Rc, Weak},
    sync::Mutex,
};

use bitflags::bitflags;

use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, DispatchData, Main,
};

pub mod keyboard;
pub mod pointer;

type SeatCallback = dyn FnMut(Attached<wl_seat::WlSeat>, &SeatData, DispatchData) + 'static;

/// The metadata associated with a seat
#[derive(Clone)]
pub struct SeatData {
    /// The name of this seat
    ///
    /// It can be used as an identifier for the seat
    pub name: String,
    /// Whether this seat has a pointer available
    pub has_pointer: bool,
    /// Whether this seat has a keyboard available
    pub has_keyboard: bool,
    /// Whether this seat has a touchscreen available
    pub has_touch: bool,
    /// Whether this seat has been removed from the registry
    ///
    /// Once a seat is removed, you will no longer receive any
    /// event on any of its associated devices (pointer, keyboard or touch).
    ///
    /// You can thus cleanup all your state associated with this seat.
    pub defunct: bool,

    /// State of readiness of the data.
    state: SeatDataState,
}

bitflags! {
    struct SeatDataState: u8 {
        const NEW              = 0b00000000;
        const GOT_NAME         = 0b00000001;
        const GOT_CAPABILITIES = 0b00000010;
        const READY            = Self::GOT_NAME.bits | Self::GOT_CAPABILITIES.bits;
    }
}

impl Debug for SeatData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeatData")
            .field("name", &self.name)
            .field("has_pointer", &self.has_pointer)
            .field("has_keyboard", &self.has_keyboard)
            .field("has_touch", &self.has_touch)
            .field("defunct", &self.defunct)
            .finish()
    }
}

impl SeatData {
    fn new() -> SeatData {
        SeatData {
            name: String::new(),
            has_pointer: false,
            has_keyboard: false,
            has_touch: false,
            defunct: false,
            state: SeatDataState::NEW,
        }
    }
}

/// A simple handler for seats
///
/// This handler will manage seats and track their capabilities.
///
/// You can register callbacks using the [`SeatHandling::listen`](trait.SeatHandling.html)
/// to be notified whenever a seat is created, destroyed, or its capabilities change.
pub struct SeatHandler {
    seats: Vec<(u32, Attached<wl_seat::WlSeat>)>,
    listeners: Rc<RefCell<Vec<Weak<RefCell<SeatCallback>>>>>,
}

impl SeatHandler {
    /// Create a new SeatHandler
    pub fn new() -> SeatHandler {
        SeatHandler { seats: Vec::new(), listeners: Rc::new(RefCell::new(Vec::new())) }
    }
}

impl fmt::Debug for SeatHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeatHandler")
            .field("seats", &self.seats)
            .field("listeners", &"Fn(..) -> { ... }")
            .finish()
    }
}

/// A handle to an seat listener callback
///
/// Dropping it disables the associated callback and frees the closure.
pub struct SeatListener {
    _cb: Rc<RefCell<SeatCallback>>,
}

impl crate::environment::MultiGlobalHandler<wl_seat::WlSeat> for SeatHandler {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        // Seat is supported up to version 6
        let version = std::cmp::min(version, 6);
        let seat = registry.bind::<wl_seat::WlSeat>(version, id);
        seat.as_ref().user_data().set_threadsafe(|| Mutex::new(SeatData::new()));
        let cb_listeners = self.listeners.clone();
        seat.quick_assign(move |seat, event, ddata| {
            process_seat_event(seat, event, &cb_listeners, ddata)
        });
        self.seats.push((id, (*seat).clone()));
    }
    fn removed(&mut self, id: u32, mut ddata: DispatchData) {
        let mut listeners = self.listeners.borrow_mut();
        self.seats.retain(|&(i, ref seat)| {
            if i != id {
                true
            } else {
                // This data must be `Mutex<SeatData>` if this seat is in our vec
                let data = seat.as_ref().user_data().get::<Mutex<SeatData>>().unwrap();
                let mut guard = data.lock().unwrap();
                guard.defunct = true;
                // notify the listeners that the seat is dead
                listeners.retain(|lst| {
                    if let Some(cb) = Weak::upgrade(lst) {
                        (cb.borrow_mut())(seat.clone(), &*guard, ddata.reborrow());
                        true
                    } else {
                        false
                    }
                });
                false
            }
        });
    }
    fn get_all(&self) -> Vec<Attached<wl_seat::WlSeat>> {
        self.seats.iter().map(|(_, s)| s.clone()).collect()
    }
}

impl fmt::Debug for SeatListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeatListener").field("_cb", &"Fn(..) -> { ... }").finish()
    }
}

fn process_seat_event(
    seat: Main<wl_seat::WlSeat>,
    event: wl_seat::Event,
    listeners: &RefCell<Vec<Weak<RefCell<SeatCallback>>>>,
    mut ddata: DispatchData,
) {
    let new_data = {
        let data = seat.as_ref().user_data().get::<Mutex<SeatData>>().unwrap();
        let mut guard = data.lock().unwrap();
        match event {
            wl_seat::Event::Name { name } => {
                guard.state.set(SeatDataState::GOT_NAME, true);
                guard.name = name;
            }
            wl_seat::Event::Capabilities { capabilities } => {
                guard.state.set(SeatDataState::GOT_CAPABILITIES, true);
                guard.has_pointer = capabilities.contains(wl_seat::Capability::Pointer);
                guard.has_keyboard = capabilities.contains(wl_seat::Capability::Keyboard);
                guard.has_touch = capabilities.contains(wl_seat::Capability::Touch);
            }
            _ => unreachable!(),
        }
        guard.clone()
    };

    if new_data.state.contains(SeatDataState::READY) {
        listeners.borrow_mut().retain(|lst| {
            if let Some(cb) = Weak::upgrade(lst) {
                (cb.borrow_mut())((*seat).clone(), &new_data, ddata.reborrow());
                true
            } else {
                false
            }
        });
    }
}

/// Get the copy of the data associated with this seat
///
/// If the provided `WlSeat` has not yet been initialized or is not managed by SCTK, `None` is returned.
///
/// If the seat has been removed by the compositor, the `defunct` field of the `SeatData`
/// will be set to `true`. This handler will not automatically detroy the output by calling its
/// `release` method, to avoid interfering with your logic.
pub fn clone_seat_data(seat: &wl_seat::WlSeat) -> Option<SeatData> {
    if let Some(udata_mutex) = seat.as_ref().user_data().get::<Mutex<SeatData>>() {
        let udata = udata_mutex.lock().unwrap();
        Some(udata.clone())
    } else {
        None
    }
}

/// Access the data associated with this seat
///
/// The provided closure is given the [`SeatData`](struct.SeatData.html) as argument,
/// and its return value is returned from this function.
///
/// If the provided `WlSeat` has not yet been initialized or is not managed by SCTK, `None` is returned.
///
/// If the seat has been removed by the compositor, the `defunct` field of the `SeatData`
/// will be set to `true`. This handler will not automatically detroy the output by calling its
/// `release` method, to avoid interfering with your logic.
pub fn with_seat_data<T, F: FnOnce(&SeatData) -> T>(seat: &wl_seat::WlSeat, f: F) -> Option<T> {
    if let Some(udata_mutex) = seat.as_ref().user_data().get::<Mutex<SeatData>>() {
        let udata = udata_mutex.lock().unwrap();
        Some(f(&*udata))
    } else {
        None
    }
}

/// Trait representing the SeatHandler functions
///
/// Implementing this trait on your inner environment struct used with the
/// [`environment!`](../macro.environment.html) by delegating it to its
/// [`SeatHandler`](struct.SeatHandler.html) field will make available the seat-associated
/// method on your [`Environment`](../environment/struct.Environment.html).
pub trait SeatHandling {
    /// Insert a listener for seat events
    fn listen<F: FnMut(Attached<wl_seat::WlSeat>, &SeatData, DispatchData) + 'static>(
        &mut self,
        f: F,
    ) -> SeatListener;
}

impl SeatHandling for SeatHandler {
    fn listen<F: FnMut(Attached<wl_seat::WlSeat>, &SeatData, DispatchData) + 'static>(
        &mut self,
        f: F,
    ) -> SeatListener {
        let rc = Rc::new(RefCell::new(f)) as Rc<_>;
        self.listeners.borrow_mut().push(Rc::downgrade(&rc));
        SeatListener { _cb: rc }
    }
}

impl<E: SeatHandling> crate::environment::Environment<E> {
    /// Insert a new listener for seats
    ///
    /// The provided closure will be invoked whenever a `wl_seat` is made available,
    /// removed, or see its capabilities changed.
    ///
    /// Note that if seats already exist when this callback is setup, it'll not be invoked on them.
    /// For you to be notified of them as well, you need to first process them manually by calling
    /// `.get_all_seats()`.
    ///
    /// The returned [`SeatListener`](../seat/struct.SeatListener.hmtl) keeps your callback alive,
    /// dropping it will disable it.
    #[must_use = "the returned SeatListener keeps your callback alive, dropping it will disable it"]
    pub fn listen_for_seats<
        F: FnMut(Attached<wl_seat::WlSeat>, &SeatData, DispatchData) + 'static,
    >(
        &self,
        f: F,
    ) -> SeatListener {
        self.with_inner(move |inner| SeatHandling::listen(inner, f))
    }
}

impl<E: crate::environment::MultiGlobalHandler<wl_seat::WlSeat>>
    crate::environment::Environment<E>
{
    /// Shorthand method to retrieve the list of seats
    pub fn get_all_seats(&self) -> Vec<Attached<wl_seat::WlSeat>> {
        self.get_all_globals::<wl_seat::WlSeat>().into_iter().collect()
    }
}
