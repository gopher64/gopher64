//! Device-agnostic classification of raw inputs for the profile-binding screen.
//!
//! This is the seam between event sources and the binding state machine: both
//! platforms feed Slint key events through [`slint_key_to_scancode`]; desktop
//! decodes SDL gamepad/joystick events via [`decode_sdl`], Android decodes
//! JNI-forwarded key/axis events via [`decode_android`]. Both produce a
//! [`Decoded`] the wizard applies with one shared capture policy. No SDL
//! window is required — these functions only read event payloads.

use crate::ui::input::{Action, axis_sign};

/// One classified raw input during binding capture. `Key` carries an SDL
/// scancode id (what profiles store); the SDL arms mirror the binding types in
/// `ui::config`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CaptureEvent {
    Key(i32),
    GamepadButton(i32),
    GamepadAxis {
        id: i32,
        sign: i16,
    },
    JoyButton(i32),
    /// Never produced on Android: SDL's Android backend reports HAT_X/HAT_Y
    /// as DPAD *buttons* (`Android_OnHat`), so hats decode to `JoyButton`.
    #[cfg(not(target_os = "android"))]
    JoyHat {
        id: i32,
        direction: u8,
    },
    JoyAxis {
        id: i32,
        sign: i16,
        initial_state: i16,
    },
}

/// Slint logical key text (see i-slint-common `key_codes.rs`) → SDL scancode
/// id, the representation input profiles store. Shifted text ("A", "!") is not
/// reverse-mapped through a layout; letters are folded to lowercase, anything
/// else unmappable returns `None`.
pub(crate) fn slint_key_to_scancode(text: &str) -> Option<i32> {
    use sdl3_sys::scancode as sc;
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let scancode = match c.to_ascii_lowercase() {
        'a'..='z' => sdl3_sys::scancode::SDL_Scancode(
            i32::from(sc::SDL_SCANCODE_A) + (c.to_ascii_lowercase() as i32 - 'a' as i32),
        ),
        '1'..='9' => sdl3_sys::scancode::SDL_Scancode(
            i32::from(sc::SDL_SCANCODE_1) + (c as i32 - '1' as i32),
        ),
        '0' => sc::SDL_SCANCODE_0,
        ' ' => sc::SDL_SCANCODE_SPACE,
        '\n' | '\r' => sc::SDL_SCANCODE_RETURN,
        '\u{1b}' => sc::SDL_SCANCODE_ESCAPE,
        '\t' => sc::SDL_SCANCODE_TAB,
        '\u{8}' => sc::SDL_SCANCODE_BACKSPACE,
        '\u{7f}' => sc::SDL_SCANCODE_DELETE,
        // Slint private-use codepoints (macOS NSEvent convention).
        '\u{f700}' => sc::SDL_SCANCODE_UP,
        '\u{f701}' => sc::SDL_SCANCODE_DOWN,
        '\u{f702}' => sc::SDL_SCANCODE_LEFT,
        '\u{f703}' => sc::SDL_SCANCODE_RIGHT,
        // F1..=F12 are contiguous in both tables.
        c @ '\u{f704}'..='\u{f70f}' => {
            sdl3_sys::scancode::SDL_Scancode(i32::from(sc::SDL_SCANCODE_F1) + (c as i32 - 0xf704))
        }
        '\u{f727}' => sc::SDL_SCANCODE_INSERT,
        '\u{f729}' => sc::SDL_SCANCODE_HOME,
        '\u{f72b}' => sc::SDL_SCANCODE_END,
        '\u{f72c}' => sc::SDL_SCANCODE_PAGEUP,
        '\u{f72d}' => sc::SDL_SCANCODE_PAGEDOWN,
        '-' => sc::SDL_SCANCODE_MINUS,
        '=' => sc::SDL_SCANCODE_EQUALS,
        '[' => sc::SDL_SCANCODE_LEFTBRACKET,
        ']' => sc::SDL_SCANCODE_RIGHTBRACKET,
        '\\' => sc::SDL_SCANCODE_BACKSLASH,
        ';' => sc::SDL_SCANCODE_SEMICOLON,
        '\'' => sc::SDL_SCANCODE_APOSTROPHE,
        '`' => sc::SDL_SCANCODE_GRAVE,
        ',' => sc::SDL_SCANCODE_COMMA,
        '.' => sc::SDL_SCANCODE_PERIOD,
        '/' => sc::SDL_SCANCODE_SLASH,
        _ => return None,
    };
    Some(i32::from(scancode))
}

/// Gamepad axis capture gate, identical to `wait_capture`: past ¾ deflection
/// binds with `axis_sign(v, 0)` (gamepad axes rest at 0), below is ignored.
pub(crate) fn gamepad_axis_capture(id: i32, value: i16) -> Option<CaptureEvent> {
    let past = value.saturating_abs() > (i16::MAX as i32 * 3 / 4) as i16;
    past.then(|| CaptureEvent::GamepadAxis {
        id,
        sign: axis_sign(value, 0),
    })
}

/// Normalize a joystick axis resting value to `i16::MIN` / `0` / `i16::MAX`,
/// identical to `wait_capture` (triggers rest at an extreme; sticks at ~0).
/// `has_initial_state` false (SDL couldn't query it) assumes centered.
#[cfg(not(target_os = "android"))]
pub(crate) fn normalize_initial_state(initial_state: i16, has_initial_state: bool) -> i16 {
    if !has_initial_state {
        return 0;
    }
    if initial_state < i16::MIN / 2 {
        i16::MIN
    } else if initial_state > i16::MAX / 2 {
        i16::MAX
    } else {
        0
    }
}

/// Joystick axis capture gate, identical to `wait_capture`: deflection is
/// measured against the (normalized) resting state, past ¼ of full range binds
/// with `axis_sign(v, initial_state)`.
pub(crate) fn joy_axis_capture(id: i32, value: i16, initial_state: i16) -> Option<CaptureEvent> {
    let past = value.abs_diff(initial_state) > (u16::MAX / 4);
    past.then(|| CaptureEvent::JoyAxis {
        id,
        sign: axis_sign(value, initial_state),
        initial_state,
    })
}

/// Classify ONE raw SDL event into a bindable `CaptureEvent`, lifting the
/// per-device arms of `wait_capture`. `dinput` selects the joystick arms
/// (raw button/hat/axis) over the gamepad ones, mirroring how
/// `configure_input_profile` opens devices. Non-bindable events → `None`;
/// capture policy (Esc/East skip, axis neutral re-arm) stays with the caller.
#[cfg(not(target_os = "android"))]
pub(crate) fn classify_sdl_event(
    ev: &sdl3_sys::events::SDL_Event,
    dinput: bool,
) -> Option<CaptureEvent> {
    let et = ev.event_type();
    if et == sdl3_sys::events::SDL_EVENT_GAMEPAD_BUTTON_DOWN {
        if !dinput {
            return Some(CaptureEvent::GamepadButton(i32::from(unsafe {
                ev.gbutton.button
            })));
        }
    } else if et == sdl3_sys::events::SDL_EVENT_GAMEPAD_AXIS_MOTION {
        if !dinput {
            let (axis, value) = unsafe { (ev.gaxis.axis, ev.gaxis.value) };
            return gamepad_axis_capture(i32::from(axis), value);
        }
    } else if et == sdl3_sys::events::SDL_EVENT_JOYSTICK_BUTTON_DOWN {
        if dinput {
            return Some(CaptureEvent::JoyButton(i32::from(unsafe {
                ev.jbutton.button
            })));
        }
    } else if et == sdl3_sys::events::SDL_EVENT_JOYSTICK_HAT_MOTION {
        let (hat, direction) = unsafe { (ev.jhat.hat, ev.jhat.value) };
        if dinput && direction != sdl3_sys::joystick::SDL_HAT_CENTERED {
            return Some(CaptureEvent::JoyHat {
                id: i32::from(hat),
                direction,
            });
        }
    } else if et == sdl3_sys::events::SDL_EVENT_JOYSTICK_AXIS_MOTION && dinput {
        let (which, axis, value) = unsafe { (ev.jaxis.which, ev.jaxis.axis, ev.jaxis.value) };
        let mut initial_state = 0;
        let has_initial_state = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxisInitialState(
                sdl3_sys::joystick::SDL_GetJoystickFromID(which),
                i32::from(axis),
                &mut initial_state,
            )
        };
        let initial_state = normalize_initial_state(initial_state, has_initial_state);
        return joy_axis_capture(i32::from(axis), value, initial_state);
    }
    None
}

/// Decode ONE raw SDL event into a navigation `Action`, lifting the
/// key/gamepad/joystick arms of `wait_nav_action`. Touch and gamepad-stick
/// navigation stay with the UI layer (they need hit areas / axis state).
#[cfg(not(target_os = "android"))]
pub(crate) fn classify_sdl_nav(ev: &sdl3_sys::events::SDL_Event) -> Option<Action> {
    let et = ev.event_type();
    if et == sdl3_sys::events::SDL_EVENT_KEY_DOWN && !unsafe { ev.key.repeat } {
        let sc = unsafe { ev.key.scancode };
        if sc == sdl3_sys::scancode::SDL_SCANCODE_UP {
            return Some(Action::Up);
        } else if sc == sdl3_sys::scancode::SDL_SCANCODE_DOWN {
            return Some(Action::Down);
        } else if sc == sdl3_sys::scancode::SDL_SCANCODE_RETURN {
            return Some(Action::Confirm);
        } else if sc == sdl3_sys::scancode::SDL_SCANCODE_ESCAPE
            || sc == sdl3_sys::scancode::SDL_SCANCODE_AC_BACK
        {
            return Some(Action::Cancel);
        }
    } else if et == sdl3_sys::events::SDL_EVENT_GAMEPAD_BUTTON_DOWN {
        let b = i32::from(unsafe { ev.gbutton.button });
        if b == sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_UP.value() {
            return Some(Action::Up);
        } else if b == sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_DOWN.value() {
            return Some(Action::Down);
        } else if b == sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_SOUTH.value() {
            return Some(Action::Confirm);
        } else if b == sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_EAST.value() {
            return Some(Action::Cancel);
        }
    } else if et == sdl3_sys::events::SDL_EVENT_JOYSTICK_HAT_MOTION {
        let state = unsafe { ev.jhat.value };
        if state == sdl3_sys::joystick::SDL_HAT_UP {
            return Some(Action::Up);
        } else if state == sdl3_sys::joystick::SDL_HAT_DOWN {
            return Some(Action::Down);
        }
    } else if et == sdl3_sys::events::SDL_EVENT_JOYSTICK_BUTTON_DOWN
        && unsafe { ev.jbutton.button } == 0
    {
        return Some(Action::Confirm);
    }
    None
}

/// One platform event decoded for the wizard: at most one navigation intent,
/// at most one bindable capture, plus the flags the capture policy gates on.
/// `axis_stream` marks events on the bindable axis stream (the one the
/// axis-neutral latch applies to); `list_y` carries a left-stick Y sample for
/// edge-triggered list navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decoded {
    pub(crate) nav: Option<Action>,
    pub(crate) bind: Option<CaptureEvent>,
    pub(crate) axis_stream: bool,
    pub(crate) list_y: Option<i16>,
}

/// Decode ONE raw SDL event (desktop pump) into a [`Decoded`].
#[cfg(not(target_os = "android"))]
pub(crate) fn decode_sdl(ev: &sdl3_sys::events::SDL_Event, dinput: bool) -> Decoded {
    let et = ev.event_type();
    // The bindable axis stream mirrors how devices are opened: gamepad axes
    // normally, raw joystick axes for DirectInput.
    let axis_stream = if dinput {
        et == sdl3_sys::events::SDL_EVENT_JOYSTICK_AXIS_MOTION
    } else {
        et == sdl3_sys::events::SDL_EVENT_GAMEPAD_AXIS_MOTION
    };
    let list_y = if et == sdl3_sys::events::SDL_EVENT_GAMEPAD_AXIS_MOTION {
        let (axis, v) = unsafe { (ev.gaxis.axis, ev.gaxis.value) };
        (i32::from(axis) == sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY.value()).then_some(v)
    } else {
        None
    };
    Decoded {
        nav: classify_sdl_nav(ev),
        bind: classify_sdl_event(ev, dinput),
        axis_stream,
        list_y,
    }
}

// --- Android (JNI) decoding -------------------------------------------------
//
// Captured bindings must match what the game reports at play time. In-game,
// the ROM runs in `N64Activity` (an `SDLActivity`), so SDL's own Android
// backend produces the ids; the wizard (running windowless under
// `SlintActivity`, where SDL is NOT initialized) must therefore mirror SDL's
// Android mapping exactly. Sources (SDL 3.4, vendored):
//
// Buttons — `keycode_to_SDL` in `src/joystick/android/SDL_sysjoystick.c`.
// Android keycodes map to fixed SDL joystick button indices which EQUAL the
// `SDL_GamepadButton` values (SDL's Android auto-mapping is `b<N>:<button N>`):
//   KEYCODE_BUTTON_A(96)→0 SOUTH        KEYCODE_BUTTON_B(97)→1 EAST
//   KEYCODE_BUTTON_X(99)→2 WEST         KEYCODE_BUTTON_Y(100)→3 NORTH
//   KEYCODE_BACK(4)/BUTTON_SELECT(109)→4 BACK   KEYCODE_BUTTON_MODE(110)→5 GUIDE
//   KEYCODE_MENU(82)/BUTTON_START(108)→6 START
//   KEYCODE_BUTTON_THUMBL(106)→7 LEFT_STICK     KEYCODE_BUTTON_THUMBR(107)→8 RIGHT_STICK
//   KEYCODE_BUTTON_L1(102)→9 LEFT_SHOULDER      KEYCODE_BUTTON_R1(103)→10 RIGHT_SHOULDER
//   KEYCODE_DPAD_UP(19)→11  DOWN(20)→12  LEFT(21)→13  RIGHT(22)→14
//   KEYCODE_DPAD_CENTER(23)→0 SOUTH (SDL folds it into A)
//   KEYCODE_BUTTON_L2(104)→15 MISC1     KEYCODE_BUTTON_R2(105)→16
//   KEYCODE_BUTTON_C(98)→17             KEYCODE_BUTTON_Z(101)→18
//   KEYCODE_BUTTON_1..16(188..203)→20..35
// HAT_X/HAT_Y motion is translated by SDL into the DPAD buttons 11–14
// (`Android_OnHat`), never into SDL hat events — Kotlin therefore synthesizes
// DPAD keycodes on hat edges and `JoyHat` is never produced here.
//
// Axes — `SDLControllerManager.java` sorts the device's joystick-class motion
// ranges (RangeComparator: GAS/BRAKE swapped, Z re-keyed to sit just before
// RZ) and numbers them in order; SDL's auto-mapping then assigns
// leftx=a0, lefty=a1, rightx=a2, righty=a3, lefttrigger=a4, righttrigger=a5.
// For the standard Android gamepad profile (X, Y, Z, RZ, LTRIGGER|BRAKE,
// RTRIGGER|GAS) that yields the static table below. RISK (needs on-device
// verification): controllers whose axis set deviates from the standard
// profile (e.g. DS4 reporting triggers on RX/RY) get their in-game ids from
// SDL's curated controller database, which this static table cannot consult;
// such devices may capture axis ids that differ from in-game. RX/RY are
// deliberately not bindable for that reason.

/// One controller/keyboard input forwarded from `SlintActivity` over JNI.
/// Axis values arrive normalized to `-1..=1` exactly as
/// `SDLControllerManager.handleMotionEvent` normalizes them for the game;
/// `rest` is the sign (-1/0/+1) of the axis' resting position.
#[cfg(any(target_os = "android", test))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AndroidEvent {
    /// `KeyEvent` ACTION_DOWN, repeat 0. `source` is the raw
    /// `InputEvent.getSource()` bits.
    Key { source: i32, keycode: i32 },
    /// One changed `MotionEvent` axis. `axis` is the raw `MotionEvent.AXIS_*`.
    Axis { axis: i32, value: f32, rest: i8 },
}

/// `InputDevice.SOURCE_*` class bits that mark a controller.
#[cfg(any(target_os = "android", test))]
const SOURCE_GAMEPAD: i32 = 0x0000_0401;
#[cfg(any(target_os = "android", test))]
const SOURCE_JOYSTICK: i32 = 0x0100_0010;
#[cfg(any(target_os = "android", test))]
const SOURCE_DPAD: i32 = 0x0000_0201;

/// Whether `source` contains a controller class. Android `SOURCE_*` values
/// share class bits (e.g. GAMEPAD/DPAD/KEYBOARD all carry CLASS_BUTTON 0x1),
/// so containment MUST be `(source & S) == S`, never `!= 0` — a plain
/// keyboard (0x101) would otherwise match GAMEPAD (0x401).
#[cfg(any(target_os = "android", test))]
fn is_controller_source(source: i32) -> bool {
    (source & SOURCE_GAMEPAD) == SOURCE_GAMEPAD
        || (source & SOURCE_JOYSTICK) == SOURCE_JOYSTICK
        || (source & SOURCE_DPAD) == SOURCE_DPAD
}

/// `android.view.KeyEvent` keycodes (stable API constants).
#[cfg(any(target_os = "android", test))]
mod keycode {
    pub(super) const BACK: i32 = 4;
    pub(super) const DPAD_UP: i32 = 19;
    pub(super) const DPAD_DOWN: i32 = 20;
    pub(super) const DPAD_LEFT: i32 = 21;
    pub(super) const DPAD_RIGHT: i32 = 22;
    pub(super) const DPAD_CENTER: i32 = 23;
    pub(super) const A: i32 = 29; // ..= Z (54)
    pub(super) const Z: i32 = 54;
    pub(super) const KEY_0: i32 = 7; // ..= 9 (16)
    pub(super) const KEY_9: i32 = 16;
    pub(super) const SPACE: i32 = 62;
    pub(super) const ENTER: i32 = 66;
    pub(super) const ESCAPE: i32 = 111;
    pub(super) const MENU: i32 = 82;
    pub(super) const BUTTON_A: i32 = 96;
    pub(super) const BUTTON_Z: i32 = 101;
    pub(super) const BUTTON_L2: i32 = 104;
    pub(super) const BUTTON_R2: i32 = 105;
    pub(super) const BUTTON_THUMBL: i32 = 106;
    pub(super) const BUTTON_THUMBR: i32 = 107;
    pub(super) const BUTTON_START: i32 = 108;
    pub(super) const BUTTON_SELECT: i32 = 109;
    pub(super) const BUTTON_MODE: i32 = 110;
    pub(super) const BUTTON_1: i32 = 188; // ..= BUTTON_16 (203)
    pub(super) const BUTTON_16: i32 = 203;
}

/// `android.view.MotionEvent` axis ids (stable API constants).
#[cfg(any(target_os = "android", test))]
mod axis {
    pub(super) const X: i32 = 0;
    pub(super) const Y: i32 = 1;
    pub(super) const Z: i32 = 11;
    pub(super) const RZ: i32 = 14;
    pub(super) const LTRIGGER: i32 = 17;
    pub(super) const RTRIGGER: i32 = 18;
    pub(super) const GAS: i32 = 22;
    pub(super) const BRAKE: i32 = 23;
}

/// Android keycode → SDL button index (== `SDL_GamepadButton` value), the
/// `keycode_to_SDL` table from SDL's Android joystick backend.
#[cfg(any(target_os = "android", test))]
fn android_keycode_to_sdl_button(code: i32) -> Option<i32> {
    use keycode as k;
    Some(match code {
        k::BUTTON_A | k::DPAD_CENTER => 0, // SOUTH
        97 => 1,                           // BUTTON_B → EAST
        99 => 2,                           // BUTTON_X → WEST
        100 => 3,                          // BUTTON_Y → NORTH
        k::BACK | k::BUTTON_SELECT => 4,   // BACK
        k::BUTTON_MODE => 5,               // GUIDE
        k::MENU | k::BUTTON_START => 6,    // START
        k::BUTTON_THUMBL => 7,             // LEFT_STICK
        k::BUTTON_THUMBR => 8,             // RIGHT_STICK
        102 => 9,                          // BUTTON_L1 → LEFT_SHOULDER
        103 => 10,                         // BUTTON_R1 → RIGHT_SHOULDER
        k::DPAD_UP => 11,
        k::DPAD_DOWN => 12,
        k::DPAD_LEFT => 13,
        k::DPAD_RIGHT => 14,
        k::BUTTON_L2 => 15, // MISC1
        k::BUTTON_R2 => 16,
        98 => 17, // BUTTON_C
        k::BUTTON_Z => 18,
        c @ k::BUTTON_1..=k::BUTTON_16 => 20 + (c - k::BUTTON_1),
        _ => return None,
    })
}

/// Android keyboard keycode → SDL scancode id (the representation profiles
/// store), for binding a physical keyboard. Deliberately the same compact set
/// desktop's `slint_key_to_scancode` covers for game inputs.
#[cfg(any(target_os = "android", test))]
fn android_keycode_to_scancode(code: i32) -> Option<i32> {
    use keycode as k;
    use sdl3_sys::scancode as sc;
    let scancode = match code {
        c @ k::A..=k::Z => {
            sdl3_sys::scancode::SDL_Scancode(i32::from(sc::SDL_SCANCODE_A) + (c - k::A))
        }
        k::KEY_0 => sc::SDL_SCANCODE_0,
        c @ 8..=k::KEY_9 => {
            // KEYCODE_1(8)..KEYCODE_9(16)
            sdl3_sys::scancode::SDL_Scancode(i32::from(sc::SDL_SCANCODE_1) + (c - 8))
        }
        k::SPACE => sc::SDL_SCANCODE_SPACE,
        k::ENTER => sc::SDL_SCANCODE_RETURN,
        k::DPAD_UP => sc::SDL_SCANCODE_UP,
        k::DPAD_DOWN => sc::SDL_SCANCODE_DOWN,
        k::DPAD_LEFT => sc::SDL_SCANCODE_LEFT,
        k::DPAD_RIGHT => sc::SDL_SCANCODE_RIGHT,
        _ => return None,
    };
    Some(i32::from(scancode))
}

/// SDL button index → navigation `Action`, mirroring both desktop nav arms
/// (gamepad DPAD/SOUTH/EAST; the ids coincide for raw Android joysticks).
#[cfg(any(target_os = "android", test))]
fn android_button_nav(button: i32) -> Option<Action> {
    match button {
        11 => Some(Action::Up),     // DPAD_UP
        12 => Some(Action::Down),   // DPAD_DOWN
        0 => Some(Action::Confirm), // SOUTH
        1 => Some(Action::Cancel),  // EAST
        _ => None,
    }
}

/// Android `MotionEvent` axis → SDL axis index for the standard Android
/// gamepad profile (see the module comment's RISK note). The id doubles as
/// the `SDL_GamepadAxis` value in gamepad mode and the raw joystick axis
/// index in dinput mode.
#[cfg(any(target_os = "android", test))]
fn android_axis_to_sdl(a: i32) -> Option<i32> {
    match a {
        axis::X => Some(0),                      // LEFTX
        axis::Y => Some(1),                      // LEFTY
        axis::Z => Some(2),                      // RIGHTX
        axis::RZ => Some(3),                     // RIGHTY
        axis::LTRIGGER | axis::BRAKE => Some(4), // LEFT_TRIGGER
        axis::RTRIGGER | axis::GAS => Some(5),   // RIGHT_TRIGGER
        _ => None,
    }
}

/// Decode ONE JNI-forwarded Android input into a [`Decoded`].
///
/// System back (`KEYCODE_BACK`) is always `Cancel` and never bindable —
/// matching Android back semantics (and desktop's Esc). Controller-sourced
/// keys bind via the SDL button table; keyboard keys via the scancode table.
/// Axes bind through the same deflection gates as desktop
/// ([`gamepad_axis_capture`] / [`joy_axis_capture`]).
#[cfg(any(target_os = "android", test))]
pub(crate) fn decode_android(ev: &AndroidEvent, dinput: bool) -> Decoded {
    let mut d = Decoded {
        nav: None,
        bind: None,
        axis_stream: false,
        list_y: None,
    };
    match *ev {
        AndroidEvent::Key { source, keycode } => {
            if keycode == keycode::BACK {
                d.nav = Some(Action::Cancel);
            } else if is_controller_source(source) {
                if let Some(button) = android_keycode_to_sdl_button(keycode) {
                    d.nav = android_button_nav(button);
                    d.bind = Some(if dinput {
                        CaptureEvent::JoyButton(button)
                    } else {
                        CaptureEvent::GamepadButton(button)
                    });
                }
            } else {
                d.nav = match keycode {
                    keycode::ESCAPE => Some(Action::Cancel),
                    keycode::ENTER | keycode::DPAD_CENTER => Some(Action::Confirm),
                    keycode::DPAD_UP => Some(Action::Up),
                    keycode::DPAD_DOWN => Some(Action::Down),
                    _ => None,
                };
                d.bind = android_keycode_to_scancode(keycode).map(CaptureEvent::Key);
            }
        }
        AndroidEvent::Axis { axis, value, rest } => {
            let Some(id) = android_axis_to_sdl(axis) else {
                return d;
            };
            d.axis_stream = true;
            if dinput {
                // Raw joystick semantics: full -1..=1 range; triggers rest at
                // an extreme, like SDL's (normalized) initial state.
                let v = (value.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
                let initial_state = match rest {
                    r if r < 0 => i16::MIN,
                    r if r > 0 => i16::MAX,
                    _ => 0,
                };
                d.bind = joy_axis_capture(id, v, initial_state);
                if id == 1 {
                    d.list_y = Some(v);
                }
            } else {
                // SDL gamepad semantics: triggers (a4/a5) are re-ranged by
                // SDL's mapping to 0 (released) ..= 32767 (pulled); sticks
                // keep the full range resting at 0.
                let v = if id >= 4 {
                    (((value.clamp(-1.0, 1.0) + 1.0) / 2.0) * f32::from(i16::MAX)) as i16
                } else {
                    (value.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
                };
                d.bind = gamepad_axis_capture(id, v);
                if id == 1 {
                    d.list_y = Some(v);
                }
            }
        }
    }
    d
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::input::Action;

    // --- test event builders (writing a Copy union field is safe) ---

    fn key_event(
        scancode: sdl3_sys::scancode::SDL_Scancode,
        repeat: bool,
    ) -> sdl3_sys::events::SDL_Event {
        let mut ev: sdl3_sys::events::SDL_Event = Default::default();
        ev.key = sdl3_sys::events::SDL_KeyboardEvent {
            r#type: sdl3_sys::events::SDL_EVENT_KEY_DOWN,
            scancode,
            down: true,
            repeat,
            ..Default::default()
        };
        ev
    }

    fn gamepad_button_event(button: i32) -> sdl3_sys::events::SDL_Event {
        let mut ev: sdl3_sys::events::SDL_Event = Default::default();
        ev.gbutton = sdl3_sys::events::SDL_GamepadButtonEvent {
            r#type: sdl3_sys::events::SDL_EVENT_GAMEPAD_BUTTON_DOWN,
            button: button as u8,
            down: true,
            ..Default::default()
        };
        ev
    }

    fn gamepad_axis_event(axis: u8, value: i16) -> sdl3_sys::events::SDL_Event {
        let mut ev: sdl3_sys::events::SDL_Event = Default::default();
        ev.gaxis = sdl3_sys::events::SDL_GamepadAxisEvent {
            r#type: sdl3_sys::events::SDL_EVENT_GAMEPAD_AXIS_MOTION,
            axis,
            value,
            ..Default::default()
        };
        ev
    }

    fn joy_button_event(button: u8) -> sdl3_sys::events::SDL_Event {
        let mut ev: sdl3_sys::events::SDL_Event = Default::default();
        ev.jbutton = sdl3_sys::events::SDL_JoyButtonEvent {
            r#type: sdl3_sys::events::SDL_EVENT_JOYSTICK_BUTTON_DOWN,
            button,
            down: true,
            ..Default::default()
        };
        ev
    }

    fn joy_hat_event(hat: u8, value: u8) -> sdl3_sys::events::SDL_Event {
        let mut ev: sdl3_sys::events::SDL_Event = Default::default();
        ev.jhat = sdl3_sys::events::SDL_JoyHatEvent {
            r#type: sdl3_sys::events::SDL_EVENT_JOYSTICK_HAT_MOTION,
            hat,
            value,
            ..Default::default()
        };
        ev
    }

    fn scancode_id(sc: sdl3_sys::scancode::SDL_Scancode) -> i32 {
        i32::from(sc)
    }

    // --- slint_key_to_scancode ---

    #[test]
    fn key_map_letters_and_digits() {
        use sdl3_sys::scancode as sc;
        assert_eq!(
            slint_key_to_scancode("a"),
            Some(scancode_id(sc::SDL_SCANCODE_A))
        );
        assert_eq!(
            slint_key_to_scancode("z"),
            Some(scancode_id(sc::SDL_SCANCODE_Z))
        );
        // Shifted text still names the same physical key.
        assert_eq!(
            slint_key_to_scancode("A"),
            Some(scancode_id(sc::SDL_SCANCODE_A))
        );
        assert_eq!(
            slint_key_to_scancode("1"),
            Some(scancode_id(sc::SDL_SCANCODE_1))
        );
        assert_eq!(
            slint_key_to_scancode("9"),
            Some(scancode_id(sc::SDL_SCANCODE_9))
        );
        assert_eq!(
            slint_key_to_scancode("0"),
            Some(scancode_id(sc::SDL_SCANCODE_0))
        );
    }

    #[test]
    fn key_map_special_keys() {
        use sdl3_sys::scancode as sc;
        // Slint logical key texts: see i-slint-common key_codes.rs.
        assert_eq!(
            slint_key_to_scancode(" "),
            Some(scancode_id(sc::SDL_SCANCODE_SPACE))
        );
        assert_eq!(
            slint_key_to_scancode("\n"),
            Some(scancode_id(sc::SDL_SCANCODE_RETURN))
        );
        assert_eq!(
            slint_key_to_scancode("\r"),
            Some(scancode_id(sc::SDL_SCANCODE_RETURN))
        );
        assert_eq!(
            slint_key_to_scancode("\u{1b}"),
            Some(scancode_id(sc::SDL_SCANCODE_ESCAPE))
        );
        assert_eq!(
            slint_key_to_scancode("\t"),
            Some(scancode_id(sc::SDL_SCANCODE_TAB))
        );
        assert_eq!(
            slint_key_to_scancode("\u{8}"),
            Some(scancode_id(sc::SDL_SCANCODE_BACKSPACE))
        );
        assert_eq!(
            slint_key_to_scancode("\u{7f}"),
            Some(scancode_id(sc::SDL_SCANCODE_DELETE))
        );
        assert_eq!(
            slint_key_to_scancode("\u{f700}"),
            Some(scancode_id(sc::SDL_SCANCODE_UP))
        );
        assert_eq!(
            slint_key_to_scancode("\u{f701}"),
            Some(scancode_id(sc::SDL_SCANCODE_DOWN))
        );
        assert_eq!(
            slint_key_to_scancode("\u{f702}"),
            Some(scancode_id(sc::SDL_SCANCODE_LEFT))
        );
        assert_eq!(
            slint_key_to_scancode("\u{f703}"),
            Some(scancode_id(sc::SDL_SCANCODE_RIGHT))
        );
        assert_eq!(
            slint_key_to_scancode("\u{f704}"),
            Some(scancode_id(sc::SDL_SCANCODE_F1))
        );
        assert_eq!(
            slint_key_to_scancode("\u{f70f}"),
            Some(scancode_id(sc::SDL_SCANCODE_F12))
        );
        assert_eq!(
            slint_key_to_scancode(","),
            Some(scancode_id(sc::SDL_SCANCODE_COMMA))
        );
        assert_eq!(
            slint_key_to_scancode("/"),
            Some(scancode_id(sc::SDL_SCANCODE_SLASH))
        );
    }

    #[test]
    fn key_map_unmappable_returns_none() {
        assert_eq!(slint_key_to_scancode("\u{1}"), None);
        assert_eq!(slint_key_to_scancode(""), None);
        assert_eq!(slint_key_to_scancode("ab"), None);
        // Modifier-only logical keys (Shift) are not bindable scancodes here.
        assert_eq!(slint_key_to_scancode("\u{10}"), None);
    }

    // --- axis helpers (threshold + initial_state normalization) ---

    #[test]
    fn gamepad_axis_threshold() {
        // wait_capture gate: |v| > i16::MAX * 3 / 4 (= 24575).
        assert_eq!(
            gamepad_axis_capture(0, 24576),
            Some(CaptureEvent::GamepadAxis { id: 0, sign: 1 })
        );
        assert_eq!(
            gamepad_axis_capture(1, -24576),
            Some(CaptureEvent::GamepadAxis { id: 1, sign: -1 })
        );
        assert_eq!(gamepad_axis_capture(0, 24575), None);
        assert_eq!(gamepad_axis_capture(0, -24575), None);
        assert_eq!(gamepad_axis_capture(0, 0), None);
        // i16::MIN must not panic (saturating_abs).
        assert_eq!(
            gamepad_axis_capture(2, i16::MIN),
            Some(CaptureEvent::GamepadAxis { id: 2, sign: -1 })
        );
    }

    #[test]
    fn joy_axis_resting_at_min_reads_zero_without_panic() {
        // A trigger rests at i16::MIN; reading 0 is a half-pull past the gate.
        assert_eq!(
            joy_axis_capture(2, 0, i16::MIN),
            Some(CaptureEvent::JoyAxis {
                id: 2,
                sign: 1,
                initial_state: i16::MIN
            })
        );
        // Small deflection from the resting point stays below the gate.
        assert_eq!(joy_axis_capture(2, -30000, i16::MIN), None);
        // wait_capture gate: abs_diff > u16::MAX / 4 (= 16383).
        assert_eq!(joy_axis_capture(0, 16383, 0), None);
        assert_eq!(
            joy_axis_capture(0, 16384, 0),
            Some(CaptureEvent::JoyAxis {
                id: 0,
                sign: 1,
                initial_state: 0
            })
        );
        assert_eq!(
            joy_axis_capture(0, -16384, 0),
            Some(CaptureEvent::JoyAxis {
                id: 0,
                sign: -1,
                initial_state: 0
            })
        );
    }

    #[test]
    fn initial_state_normalizes_to_three_buckets() {
        // Identical to wait_capture: <MIN/2 → MIN, >MAX/2 → MAX, else 0.
        assert_eq!(normalize_initial_state(i16::MIN, true), i16::MIN);
        assert_eq!(normalize_initial_state(i16::MIN / 2 - 1, true), i16::MIN);
        assert_eq!(normalize_initial_state(i16::MIN / 2, true), 0);
        assert_eq!(normalize_initial_state(-1000, true), 0);
        assert_eq!(normalize_initial_state(0, true), 0);
        assert_eq!(normalize_initial_state(i16::MAX / 2, true), 0);
        assert_eq!(normalize_initial_state(i16::MAX / 2 + 1, true), i16::MAX);
        assert_eq!(normalize_initial_state(i16::MAX, true), i16::MAX);
        // No queryable initial state → assume centered.
        assert_eq!(normalize_initial_state(i16::MIN, false), 0);
    }

    // --- classify_sdl_event ---

    #[test]
    fn classify_gamepad_button_respects_dinput() {
        let south = sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_SOUTH.value();
        let ev = gamepad_button_event(south);
        assert_eq!(
            classify_sdl_event(&ev, false),
            Some(CaptureEvent::GamepadButton(south))
        );
        // dinput profiles bind the raw joystick arms, not the gamepad ones.
        assert_eq!(classify_sdl_event(&ev, true), None);
    }

    #[test]
    fn classify_gamepad_axis_event() {
        let ev = gamepad_axis_event(3, i16::MAX);
        assert_eq!(
            classify_sdl_event(&ev, false),
            Some(CaptureEvent::GamepadAxis { id: 3, sign: 1 })
        );
        let below = gamepad_axis_event(3, 1000);
        assert_eq!(classify_sdl_event(&below, false), None);
        assert_eq!(classify_sdl_event(&ev, true), None);
    }

    #[test]
    fn classify_joystick_button_and_hat() {
        let ev = joy_button_event(3);
        assert_eq!(
            classify_sdl_event(&ev, true),
            Some(CaptureEvent::JoyButton(3))
        );
        assert_eq!(classify_sdl_event(&ev, false), None);

        let hat = joy_hat_event(0, sdl3_sys::joystick::SDL_HAT_UP);
        assert_eq!(
            classify_sdl_event(&hat, true),
            Some(CaptureEvent::JoyHat {
                id: 0,
                direction: sdl3_sys::joystick::SDL_HAT_UP
            })
        );
        // Returning to center is not a bindable input.
        let centered = joy_hat_event(0, sdl3_sys::joystick::SDL_HAT_CENTERED);
        assert_eq!(classify_sdl_event(&centered, true), None);
        assert_eq!(classify_sdl_event(&hat, false), None);
    }

    // --- classify_sdl_nav ---

    #[test]
    fn classify_nav_keyboard() {
        use sdl3_sys::scancode as sc;
        assert_eq!(
            classify_sdl_nav(&key_event(sc::SDL_SCANCODE_UP, false)),
            Some(Action::Up)
        );
        assert_eq!(
            classify_sdl_nav(&key_event(sc::SDL_SCANCODE_DOWN, false)),
            Some(Action::Down)
        );
        assert_eq!(
            classify_sdl_nav(&key_event(sc::SDL_SCANCODE_RETURN, false)),
            Some(Action::Confirm)
        );
        assert_eq!(
            classify_sdl_nav(&key_event(sc::SDL_SCANCODE_ESCAPE, false)),
            Some(Action::Cancel)
        );
        assert_eq!(
            classify_sdl_nav(&key_event(sc::SDL_SCANCODE_AC_BACK, false)),
            Some(Action::Cancel)
        );
        // Key repeats never navigate.
        assert_eq!(
            classify_sdl_nav(&key_event(sc::SDL_SCANCODE_UP, true)),
            None
        );
        assert_eq!(
            classify_sdl_nav(&key_event(sc::SDL_SCANCODE_K, false)),
            None
        );
    }

    #[test]
    fn classify_nav_gamepad_and_joystick() {
        use sdl3_sys::gamepad as gp;
        assert_eq!(
            classify_sdl_nav(&gamepad_button_event(
                gp::SDL_GAMEPAD_BUTTON_DPAD_UP.value()
            )),
            Some(Action::Up)
        );
        assert_eq!(
            classify_sdl_nav(&gamepad_button_event(
                gp::SDL_GAMEPAD_BUTTON_DPAD_DOWN.value()
            )),
            Some(Action::Down)
        );
        assert_eq!(
            classify_sdl_nav(&gamepad_button_event(gp::SDL_GAMEPAD_BUTTON_SOUTH.value())),
            Some(Action::Confirm)
        );
        assert_eq!(
            classify_sdl_nav(&gamepad_button_event(gp::SDL_GAMEPAD_BUTTON_EAST.value())),
            Some(Action::Cancel)
        );
        assert_eq!(
            classify_sdl_nav(&gamepad_button_event(gp::SDL_GAMEPAD_BUTTON_NORTH.value())),
            None
        );
        assert_eq!(
            classify_sdl_nav(&joy_hat_event(0, sdl3_sys::joystick::SDL_HAT_UP)),
            Some(Action::Up)
        );
        assert_eq!(
            classify_sdl_nav(&joy_hat_event(0, sdl3_sys::joystick::SDL_HAT_DOWN)),
            Some(Action::Down)
        );
        assert_eq!(
            classify_sdl_nav(&joy_hat_event(0, sdl3_sys::joystick::SDL_HAT_CENTERED)),
            None
        );
        assert_eq!(
            classify_sdl_nav(&joy_button_event(0)),
            Some(Action::Confirm)
        );
        assert_eq!(classify_sdl_nav(&joy_button_event(1)), None);
    }

    // --- CaptureEvent variants produced by the UI layer (Slint keys) ---

    #[test]
    fn ui_layer_variants_construct_and_compare() {
        let key = CaptureEvent::Key(slint_key_to_scancode("a").unwrap());
        assert_eq!(
            key,
            CaptureEvent::Key(i32::from(sdl3_sys::scancode::SDL_SCANCODE_A))
        );
        assert_eq!(
            classify_sdl_nav(&key_event(sdl3_sys::scancode::SDL_SCANCODE_ESCAPE, false)),
            Some(Action::Cancel)
        );
    }

    // --- decode_android ---

    const GAMEPAD_SRC: i32 = super::SOURCE_GAMEPAD | super::SOURCE_JOYSTICK;
    const KEYBOARD_SRC: i32 = 0x0000_0101; // InputDevice.SOURCE_KEYBOARD

    fn android_key(source: i32, code: i32) -> AndroidEvent {
        AndroidEvent::Key {
            source,
            keycode: code,
        }
    }

    #[test]
    fn android_buttons_mirror_sdl_keycode_table() {
        // Spot-check keycode_to_SDL: A→SOUTH(0), B→EAST(1), L1→9, DPAD_UP→11,
        // BUTTON_1→20.
        for (code, button) in [(96, 0), (97, 1), (102, 9), (19, 11), (188, 20)] {
            let d = decode_android(&android_key(GAMEPAD_SRC, code), false);
            assert_eq!(d.bind, Some(CaptureEvent::GamepadButton(button)));
            // dinput binds the raw joystick arm with the SAME index.
            let d = decode_android(&android_key(GAMEPAD_SRC, code), true);
            assert_eq!(d.bind, Some(CaptureEvent::JoyButton(button)));
        }
    }

    #[test]
    fn android_nav_matches_desktop_gamepad_arm() {
        // DPAD up/down navigate, A confirms, B cancels — and stay bindable.
        let d = decode_android(&android_key(GAMEPAD_SRC, 19), false);
        assert_eq!(d.nav, Some(Action::Up));
        let d = decode_android(&android_key(GAMEPAD_SRC, 20), false);
        assert_eq!(d.nav, Some(Action::Down));
        let d = decode_android(&android_key(GAMEPAD_SRC, 96), false);
        assert_eq!(d.nav, Some(Action::Confirm));
        assert_eq!(d.bind, Some(CaptureEvent::GamepadButton(0)));
        let d = decode_android(&android_key(GAMEPAD_SRC, 97), false);
        assert_eq!(d.nav, Some(Action::Cancel));
    }

    #[test]
    fn android_system_back_cancels_and_never_binds() {
        for source in [GAMEPAD_SRC, KEYBOARD_SRC] {
            let d = decode_android(&android_key(source, 4), false);
            assert_eq!(d.nav, Some(Action::Cancel));
            assert_eq!(d.bind, None);
        }
        // BUTTON_SELECT (a real controller button) stays bindable as BACK(4).
        let d = decode_android(&android_key(GAMEPAD_SRC, 109), false);
        assert_eq!(d.bind, Some(CaptureEvent::GamepadButton(4)));
        assert_eq!(d.nav, None);
    }

    #[test]
    fn android_keyboard_binds_scancodes() {
        // KEYCODE_A(29) → SDL_SCANCODE_A; KEYCODE_1(8) → SDL_SCANCODE_1.
        let d = decode_android(&android_key(KEYBOARD_SRC, 29), false);
        assert_eq!(
            d.bind,
            Some(CaptureEvent::Key(i32::from(
                sdl3_sys::scancode::SDL_SCANCODE_A
            )))
        );
        let d = decode_android(&android_key(KEYBOARD_SRC, 8), false);
        assert_eq!(
            d.bind,
            Some(CaptureEvent::Key(i32::from(
                sdl3_sys::scancode::SDL_SCANCODE_1
            )))
        );
        // Enter both confirms (list nav) and binds (capture).
        let d = decode_android(&android_key(KEYBOARD_SRC, 66), false);
        assert_eq!(d.nav, Some(Action::Confirm));
        assert_eq!(
            d.bind,
            Some(CaptureEvent::Key(i32::from(
                sdl3_sys::scancode::SDL_SCANCODE_RETURN
            )))
        );
    }

    #[test]
    fn android_stick_axis_gates_like_desktop() {
        // Full left-stick deflection binds; half deflection does not.
        let full = AndroidEvent::Axis {
            axis: 0,
            value: -1.0,
            rest: 0,
        };
        let d = decode_android(&full, false);
        assert!(d.axis_stream);
        assert_eq!(d.bind, Some(CaptureEvent::GamepadAxis { id: 0, sign: -1 }));
        let half = AndroidEvent::Axis {
            axis: 0,
            value: 0.5,
            rest: 0,
        };
        let d = decode_android(&half, false);
        assert_eq!(d.bind, None);
        assert!(d.axis_stream);
        // AXIS_Y feeds edge-triggered list navigation.
        let y = AndroidEvent::Axis {
            axis: 1,
            value: 1.0,
            rest: 0,
        };
        let d = decode_android(&y, false);
        assert_eq!(d.list_y, Some(i16::MAX));
    }

    #[test]
    fn android_trigger_scaling_matches_sdl_gamepad_range() {
        // Java normalizes a resting trigger to -1.0; SDL's gamepad mapping
        // re-ranges that to 0 — must NOT bind.
        let rest = AndroidEvent::Axis {
            axis: 17, // LTRIGGER → a4
            value: -1.0,
            rest: -1,
        };
        let d = decode_android(&rest, false);
        assert_eq!(d.bind, None);
        // Fully pulled → +32767 → binds positive.
        let pulled = AndroidEvent::Axis {
            axis: 17,
            value: 1.0,
            rest: -1,
        };
        let d = decode_android(&pulled, false);
        assert_eq!(d.bind, Some(CaptureEvent::GamepadAxis { id: 4, sign: 1 }));
        // dinput keeps the raw range: rest is the initial state, a pull binds.
        let d = decode_android(&pulled, true);
        assert_eq!(
            d.bind,
            Some(CaptureEvent::JoyAxis {
                id: 4,
                sign: 1,
                initial_state: i16::MIN
            })
        );
        let d = decode_android(&rest, true);
        assert_eq!(d.bind, None);
        // BRAKE aliases LTRIGGER; GAS aliases RTRIGGER.
        let brake = AndroidEvent::Axis {
            axis: 23,
            value: 1.0,
            rest: -1,
        };
        assert_eq!(
            decode_android(&brake, false).bind,
            Some(CaptureEvent::GamepadAxis { id: 4, sign: 1 })
        );
    }

    #[test]
    fn android_unmapped_inputs_are_inert() {
        // RX (12) is deliberately unbindable (device-dependent meaning).
        let rx = AndroidEvent::Axis {
            axis: 12,
            value: 1.0,
            rest: 0,
        };
        let d = decode_android(&rx, false);
        assert_eq!(
            d,
            Decoded {
                nav: None,
                bind: None,
                axis_stream: false,
                list_y: None
            }
        );
        // Volume-style keycodes never reach a binding.
        let d = decode_android(&android_key(KEYBOARD_SRC, 24), false);
        assert_eq!(d.bind, None);
        assert_eq!(d.nav, None);
    }
}
