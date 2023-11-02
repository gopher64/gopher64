//! Utilities for keymap interpretation of keyboard input
//!
//! This module provides an implementation for `wl_keyboard`
//! objects using `libxkbcommon` to interpret the keyboard input
//! given the user keymap.
//!
//! The entry point of this module is the [`map_keyboard`](fn.map_keyboard.html)
//! function which, given a `wl_seat` and a callback, setup keymap interpretation
//! and key repetition for the `wl_keyboard` of this seat.
//!
//! Key repetition relies on an event source, that needs to be inserted in your
//! calloop event loop. Not doing so will prevent key repetition to work
//! (but the rest of the functionnality will not be affected).

#[cfg(feature = "calloop")]
use calloop::{timer::Timer, RegistrationToken};
#[cfg(feature = "calloop")]
use std::num::NonZeroU32;
#[cfg(feature = "calloop")]
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    convert::TryInto,
    fs::File,
    os::unix::io::{FromRawFd, RawFd},
    rc::Rc,
    time::Instant,
};
pub use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_client::{
    protocol::{wl_keyboard, wl_seat, wl_surface},
    Attached,
};

#[rustfmt::skip]
mod ffi;
mod state;
#[rustfmt::skip]
pub mod keysyms;

use self::state::KbState;
pub use self::state::{ModifiersState, RMLVO};

#[cfg(feature = "calloop")]
const MICROS_IN_SECOND: u32 = 1000000;

/// Possible kinds of key repetition
#[derive(Debug)]
pub enum RepeatKind {
    /// keys will be repeated at a set rate and delay
    Fixed {
        /// The number of repetitions per second that should occur.
        rate: u32,
        /// delay (in milliseconds) between a key press and the start of repetition
        delay: u32,
    },
    /// keys will be repeated at a rate and delay set by the wayland server
    System,
}

#[derive(Debug)]
/// An error that occurred while trying to initialize a mapped keyboard
pub enum Error {
    /// libxkbcommon is not available
    XKBNotFound,
    /// Provided RMLVO specified a keymap that would not be loaded
    BadNames,
    /// The provided seat does not have the keyboard capability
    NoKeyboard,
    /// Failed to init timers for repetition
    TimerError(std::io::Error),
}

/// Events received from a mapped keyboard
#[derive(Debug)]
pub enum Event<'a> {
    /// The keyboard focus has entered a surface
    Enter {
        /// serial number of the event
        serial: u32,
        /// surface that was entered
        surface: wl_surface::WlSurface,
        /// raw values of the currently pressed keys
        rawkeys: &'a [u32],
        /// interpreted symbols of the currently pressed keys
        keysyms: &'a [u32],
    },
    /// The keyboard focus has left a surface
    Leave {
        /// serial number of the event
        serial: u32,
        /// surface that was left
        surface: wl_surface::WlSurface,
    },
    /// The key modifiers have changed state
    Modifiers {
        /// current state of the modifiers
        modifiers: ModifiersState,
    },
    /// A key event occurred
    Key {
        /// serial number of the event
        serial: u32,
        /// time at which the keypress occurred
        time: u32,
        /// raw value of the key
        rawkey: u32,
        /// interpreted symbol of the key
        keysym: u32,
        /// new state of the key
        state: KeyState,
        /// utf8 interpretation of the entered text
        ///
        /// will always be `None` on key release events
        utf8: Option<String>,
    },
    /// A key repetition event
    Repeat {
        /// time at which the repetition occured
        time: u32,
        /// raw value of the key
        rawkey: u32,
        /// interpreted symbol of the key
        keysym: u32,
        /// utf8 interpretation of the entered text
        utf8: Option<String>,
    },
}

/// Implement a keyboard for keymap translation with key repetition
///
/// This requires you to provide a callback to receive the events after they
/// have been interpreted with the keymap.
///
/// The keymap will be loaded from the provided RMLVO rules, or from the compositor
/// provided keymap if `None`.
///
/// Returns an error if xkbcommon could not be initialized, the RMLVO specification
/// contained invalid values, or if the provided seat does not have keyboard capability.
///
/// **Note:** This adapter does not handle key repetition. See `map_keyboard_repeat` for that.
pub fn map_keyboard<F>(
    seat: &Attached<wl_seat::WlSeat>,
    rmlvo: Option<RMLVO>,
    callback: F,
) -> Result<wl_keyboard::WlKeyboard, Error>
where
    F: FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>) + 'static,
{
    let has_kbd = super::with_seat_data(seat, |data| data.has_keyboard).unwrap_or(false);
    let keyboard = if has_kbd {
        seat.get_keyboard()
    } else {
        return Err(Error::NoKeyboard);
    };

    let state = Rc::new(RefCell::new(rmlvo.map(KbState::from_rmlvo).unwrap_or_else(KbState::new)?));

    let callback = Rc::new(RefCell::new(callback)) as Rc<RefCell<_>>;

    // prepare the handler
    let mut kbd_handler = KbdHandler {
        callback,
        state,
        #[cfg(feature = "calloop")]
        repeat: None,
    };

    keyboard.quick_assign(move |keyboard, event, data| {
        kbd_handler.event(keyboard.detach(), event, data)
    });

    Ok(keyboard.detach())
}

/// Implement a keyboard for keymap translation with key repetition
///
/// This requires you to provide a callback to receive the events after they
/// have been interpreted with the keymap.
///
/// The keymap will be loaded from the provided RMLVO rules, or from the compositor
/// provided keymap if `None`.
///
/// Returns an error if xkbcommon could not be initialized, the RMLVO specification
/// contained invalid values, or if the provided seat does not have keyboard capability.
///
/// **Note:** The keyboard repetition handling requires the `calloop` cargo feature.
#[cfg(feature = "calloop")]
pub fn map_keyboard_repeat<F, Data: 'static>(
    loop_handle: calloop::LoopHandle<'static, Data>,
    seat: &Attached<wl_seat::WlSeat>,
    rmlvo: Option<RMLVO>,
    repeatkind: RepeatKind,
    callback: F,
) -> Result<wl_keyboard::WlKeyboard, Error>
where
    F: FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>) + 'static,
{
    let has_kbd = super::with_seat_data(seat, |data| data.has_keyboard).unwrap_or(false);
    let keyboard = if has_kbd {
        seat.get_keyboard()
    } else {
        return Err(Error::NoKeyboard);
    };

    let state = Rc::new(RefCell::new(rmlvo.map(KbState::from_rmlvo).unwrap_or_else(KbState::new)?));

    let callback = Rc::new(RefCell::new(callback)) as Rc<RefCell<_>>;

    let repeat = match repeatkind {
        RepeatKind::System => RepeatDetails { locked: false, gap: None, delay: 200 },
        RepeatKind::Fixed { rate, delay } => {
            let gap = rate_to_gap(rate as i32);
            RepeatDetails { locked: true, gap, delay }
        }
    };

    // Prepare the repetition handling.
    let mut handler = KbdHandler {
        callback: callback.clone(),
        state,
        repeat: Some(KbdRepeat {
            start_timer: {
                let my_loop_handle = loop_handle.clone();
                Box::new(move |source| {
                    let my_callback = callback.clone();
                    my_loop_handle
                        .insert_source(source, move |event, kbd, ddata| {
                            (my_callback.borrow_mut())(
                                event,
                                kbd.clone(),
                                wayland_client::DispatchData::wrap(ddata),
                            )
                        })
                        .unwrap()
                })
            },
            stop_timer: Box::new(move |token| loop_handle.remove(token)),
            current_repeat: Rc::new(RefCell::new(None)),
            current_timer: Cell::new(None),
            details: repeat,
        }),
    };

    keyboard
        .quick_assign(move |keyboard, event, data| handler.event(keyboard.detach(), event, data));

    Ok(keyboard.detach())
}

#[cfg(feature = "calloop")]
fn rate_to_gap(rate: i32) -> Option<NonZeroU32> {
    if rate <= 0 {
        None
    } else if MICROS_IN_SECOND < rate as u32 {
        NonZeroU32::new(1)
    } else {
        NonZeroU32::new(MICROS_IN_SECOND / rate as u32)
    }
}

/*
 * Classic handling
 */

type KbdCallback = dyn FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>);

#[cfg(feature = "calloop")]
struct RepeatDetails {
    locked: bool,
    /// Gap between key presses in microseconds.
    ///
    /// If the `gap` is `None`, it means that repeat is disabled.
    gap: Option<NonZeroU32>,
    /// Delay before starting key repeat in milliseconds.
    delay: u32,
}

struct KbdHandler {
    state: Rc<RefCell<KbState>>,
    callback: Rc<RefCell<KbdCallback>>,
    #[cfg(feature = "calloop")]
    repeat: Option<KbdRepeat>,
}

#[cfg(feature = "calloop")]
struct KbdRepeat {
    start_timer: Box<dyn Fn(RepeatSource) -> RegistrationToken>,
    stop_timer: Box<dyn Fn(RegistrationToken)>,
    current_timer: Cell<Option<RegistrationToken>>,
    current_repeat: Rc<RefCell<Option<RepeatData>>>,
    details: RepeatDetails,
}

#[cfg(feature = "calloop")]
impl KbdRepeat {
    fn start_repeat(
        &self,
        key: u32,
        keyboard: wl_keyboard::WlKeyboard,
        time: u32,
        state: Rc<RefCell<KbState>>,
    ) {
        // Start a new repetition, overwriting the previous ones
        if let Some(timer) = self.current_timer.replace(None) {
            (self.stop_timer)(timer);
        }

        // Handle disabled repeat rate.
        let gap = match self.details.gap {
            Some(gap) => Duration::from_micros(gap.get() as u64),
            None => return,
        };

        let now = Instant::now();
        *self.current_repeat.borrow_mut() = Some(RepeatData {
            keyboard,
            keycode: key,
            gap,
            start_protocol_time: time,
            start_instant: now,
        });
        let token = (self.start_timer)(RepeatSource {
            timer: Timer::from_deadline(now + Duration::from_millis(self.details.delay as u64)),
            current_repeat: self.current_repeat.clone(),
            state,
        });
        self.current_timer.set(Some(token));
    }

    fn stop_repeat(&self, key: u32) {
        // only cancel if the released key is the currently repeating key
        let mut guard = self.current_repeat.borrow_mut();
        let stop = (*guard).as_ref().map(|d| d.keycode == key).unwrap_or(false);
        if stop {
            if let Some(timer) = self.current_timer.replace(None) {
                (self.stop_timer)(timer);
            }
            *guard = None;
        }
    }

    fn stop_all_repeat(&self) {
        if let Some(timer) = self.current_timer.replace(None) {
            (self.stop_timer)(timer);
        }
        *self.current_repeat.borrow_mut() = None;
    }
}

#[cfg(feature = "calloop")]
impl Drop for KbdRepeat {
    fn drop(&mut self) {
        self.stop_all_repeat();
    }
}

impl KbdHandler {
    fn event(
        &mut self,
        kbd: wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        dispatch_data: wayland_client::DispatchData,
    ) {
        use wl_keyboard::Event;

        match event {
            Event::Keymap { format, fd, size } => self.keymap(kbd, format, fd, size),
            Event::Enter { serial, surface, keys } => {
                self.enter(kbd, serial, surface, keys, dispatch_data)
            }
            Event::Leave { serial, surface } => self.leave(kbd, serial, surface, dispatch_data),
            Event::Key { serial, time, key, state } => {
                self.key(kbd, serial, time, key, state, dispatch_data)
            }
            Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
                self.modifiers(kbd, mods_depressed, mods_latched, mods_locked, group, dispatch_data)
            }
            Event::RepeatInfo { rate, delay } => self.repeat_info(kbd, rate, delay),
            _ => {}
        }
    }

    fn keymap(
        &mut self,
        _: wl_keyboard::WlKeyboard,
        format: wl_keyboard::KeymapFormat,
        fd: RawFd,
        size: u32,
    ) {
        let fd = unsafe { File::from_raw_fd(fd) };
        let mut state = self.state.borrow_mut();
        if state.locked() {
            // state is locked, ignore keymap updates
            return;
        }
        if state.ready() {
            // new keymap, we first deinit to free resources
            unsafe {
                state.de_init();
            }
        }
        match format {
            wl_keyboard::KeymapFormat::XkbV1 => unsafe {
                state.init_with_fd(fd, size as usize);
            },
            wl_keyboard::KeymapFormat::NoKeymap => {
                // TODO: how to handle this (hopefully never occuring) case?
            }
            _ => unreachable!(),
        }
    }

    fn enter(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        surface: wl_surface::WlSurface,
        keys: Vec<u8>,
        dispatch_data: wayland_client::DispatchData,
    ) {
        let mut state = self.state.borrow_mut();
        let rawkeys = keys
            .chunks_exact(4)
            .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
            .collect::<Vec<_>>();
        let keys: Vec<u32> = rawkeys.iter().map(|k| state.get_one_sym_raw(*k)).collect();
        (self.callback.borrow_mut())(
            Event::Enter { serial, surface, rawkeys: &rawkeys, keysyms: &keys },
            object,
            dispatch_data,
        );
    }

    fn leave(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        surface: wl_surface::WlSurface,
        dispatch_data: wayland_client::DispatchData,
    ) {
        #[cfg(feature = "calloop")]
        {
            if let Some(ref mut repeat) = self.repeat {
                repeat.stop_all_repeat();
            }
        }
        (self.callback.borrow_mut())(Event::Leave { serial, surface }, object, dispatch_data);
    }

    #[cfg_attr(not(feature = "calloop"), allow(unused_variables))]
    fn key(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        time: u32,
        key: u32,
        key_state: wl_keyboard::KeyState,
        dispatch_data: wayland_client::DispatchData,
    ) {
        let (sym, utf8, repeats) = {
            let mut state = self.state.borrow_mut();
            // Get the values to generate a key event
            let sym = state.get_one_sym_raw(key);
            let utf8 = if key_state == wl_keyboard::KeyState::Pressed {
                match state.compose_feed(sym) {
                    Some(ffi::xkb_compose_feed_result::XKB_COMPOSE_FEED_ACCEPTED) => {
                        if let Some(status) = state.compose_status() {
                            match status {
                                ffi::xkb_compose_status::XKB_COMPOSE_COMPOSED => {
                                    state.compose_get_utf8()
                                }
                                ffi::xkb_compose_status::XKB_COMPOSE_NOTHING => {
                                    state.get_utf8_raw(key)
                                }
                                _ => None,
                            }
                        } else {
                            state.get_utf8_raw(key)
                        }
                    }
                    Some(_) => {
                        // XKB_COMPOSE_FEED_IGNORED
                        None
                    }
                    None => {
                        // XKB COMPOSE is not initialized
                        state.get_utf8_raw(key)
                    }
                }
            } else {
                None
            };
            let repeats = unsafe { state.key_repeats(key + 8) };
            (sym, utf8, repeats)
        };

        #[cfg(feature = "calloop")]
        {
            if let Some(ref mut repeat_handle) = self.repeat {
                if repeats {
                    if key_state == wl_keyboard::KeyState::Pressed {
                        repeat_handle.start_repeat(key, object.clone(), time, self.state.clone());
                    } else {
                        repeat_handle.stop_repeat(key);
                    }
                }
            }
        }

        (self.callback.borrow_mut())(
            Event::Key { serial, time, rawkey: key, keysym: sym, state: key_state, utf8 },
            object,
            dispatch_data,
        );
    }

    fn modifiers(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
        dispatch_data: wayland_client::DispatchData,
    ) {
        {
            let mut state = self.state.borrow_mut();
            state.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
            (self.callback.borrow_mut())(
                Event::Modifiers { modifiers: state.mods_state() },
                object,
                dispatch_data,
            );
        }
    }

    #[cfg_attr(not(feature = "calloop"), allow(unused_variables))]
    fn repeat_info(&mut self, _: wl_keyboard::WlKeyboard, rate: i32, delay: i32) {
        #[cfg(feature = "calloop")]
        {
            if let Some(ref mut repeat_handle) = self.repeat {
                if !repeat_handle.details.locked {
                    repeat_handle.details.gap = rate_to_gap(rate);
                    repeat_handle.details.delay = delay as u32;
                }
            }
        }
    }
}

/*
 * Repeat handling
 */

#[derive(Debug)]
#[cfg(feature = "calloop")]
struct RepeatData {
    keyboard: wl_keyboard::WlKeyboard,
    keycode: u32,
    /// Gap between key presses
    gap: Duration,
    start_protocol_time: u32,
    start_instant: Instant,
}

/// An event source managing the key repetition of a keyboard
///
/// It is given to you from [`map_keyboard`](fn.map_keyboard.html), and you need to
/// insert it in your calloop event loop if you want to have functionning key repetition.
///
/// If don't want key repetition you can just drop it.
///
/// This source will not directly generate calloop events, and the callback provided to
/// `EventLoopHandle::insert_source()` will be ignored. Instead it triggers the
/// callback you provided to [`map_keyboard`](fn.map_keyboard.html).
#[cfg(feature = "calloop")]
#[derive(Debug)]
pub struct RepeatSource {
    timer: calloop::timer::Timer,
    state: Rc<RefCell<KbState>>,
    current_repeat: Rc<RefCell<Option<RepeatData>>>,
}

#[cfg(feature = "calloop")]
impl calloop::EventSource for RepeatSource {
    type Event = Event<'static>;
    type Metadata = wl_keyboard::WlKeyboard;
    type Error = <calloop::timer::Timer as calloop::EventSource>::Error;
    type Ret = ();

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<calloop::PostAction>
    where
        F: FnMut(Event<'static>, &mut wl_keyboard::WlKeyboard),
    {
        let current_repeat = &self.current_repeat;
        let state = &self.state;
        self.timer.process_events(readiness, token, |last_trigger, &mut ()| {
            if let Some(ref mut data) = *current_repeat.borrow_mut() {
                // there is something to repeat
                let mut state = state.borrow_mut();
                let keysym = state.get_one_sym_raw(data.keycode);
                let utf8 = state.get_utf8_raw(data.keycode);
                // Notify the callback.
                callback(
                    Event::Repeat {
                        time: data.start_protocol_time
                            + (last_trigger - data.start_instant).as_millis() as u32,
                        rawkey: data.keycode,
                        keysym,
                        utf8,
                    },
                    &mut data.keyboard,
                );
                // Schedule the next timeout.
                calloop::timer::TimeoutAction::ToInstant(last_trigger + data.gap)
            } else {
                calloop::timer::TimeoutAction::Drop
            }
        })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.timer.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.timer.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.timer.unregister(poll)
    }
}
