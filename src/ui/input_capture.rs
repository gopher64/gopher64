//! Device-agnostic classification of raw inputs for the profile-binding screen.
//!
//! This is the seam between event sources and the binding state machine:
//! desktop feeds Slint key events through [`slint_key_to_scancode`] and SDL
//! gamepad/joystick events through [`classify_sdl_event`]/[`classify_sdl_nav`];
//! Android (JNI) can construct [`CaptureEvent`]s directly. No SDL window is
//! required — these functions only read event payloads.

use crate::ui::input::{Action, axis_sign};

/// One classified raw input during binding capture. `Key` carries an SDL
/// scancode id (what profiles store); the SDL arms mirror the binding types in
/// `ui::config`; `Nav` carries a decoded navigation intent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CaptureEvent {
    Key(i32),
    GamepadButton(i32),
    GamepadAxis {
        id: i32,
        sign: i16,
    },
    JoyButton(i32),
    JoyHat {
        id: i32,
        direction: u8,
    },
    JoyAxis {
        id: i32,
        sign: i16,
        initial_state: i16,
    },
    Nav(Action),
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

    // --- CaptureEvent variants produced by the UI layer (Slint keys, nav) ---

    #[test]
    fn ui_layer_variants_construct_and_compare() {
        let key = CaptureEvent::Key(slint_key_to_scancode("a").unwrap());
        assert_eq!(
            key,
            CaptureEvent::Key(i32::from(sdl3_sys::scancode::SDL_SCANCODE_A))
        );
        let nav = CaptureEvent::Nav(
            classify_sdl_nav(&key_event(sdl3_sys::scancode::SDL_SCANCODE_ESCAPE, false)).unwrap(),
        );
        assert_eq!(nav, CaptureEvent::Nav(Action::Cancel));
    }
}
