use std::ops::Neg;

use crate::ui;

const R_DPAD: usize = 0;
const L_DPAD: usize = 1;
const D_DPAD: usize = 2;
const U_DPAD: usize = 3;
const START_BUTTON: usize = 4;
const Z_TRIG: usize = 5;
const B_BUTTON: usize = 6;
const A_BUTTON: usize = 7;
const R_CBUTTON: usize = 8;
const L_CBUTTON: usize = 9;
const D_CBUTTON: usize = 10;
const U_CBUTTON: usize = 11;
const R_TRIG: usize = 12;
const L_TRIG: usize = 13;
const AXIS_LEFT: usize = 14;
const AXIS_RIGHT: usize = 15;
const AXIS_UP: usize = 16;
const AXIS_DOWN: usize = 17;
const HOTKEY: usize = 18;
pub const PROFILE_SIZE: usize = 19;

const X_AXIS_SHIFT: usize = 16;
const Y_AXIS_SHIFT: usize = 24;

const MAX_AXIS_VALUE: f64 = 85.0;

pub const DEADZONE_DEFAULT: i32 = 5;

pub const UNKNOWN_CONTROLLER_NAME: &str = "Unknown controller";

#[derive(Default)]
pub struct Controllers {
    pub rumble: bool,
    pub game_controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
    pub joystick: *mut sdl3_sys::joystick::SDL_Joystick,
    pub guid: sdl3_sys::guid::SDL_GUID,
    pub last_key_state: u32,
}

#[derive(Default, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputData {
    pub data: u32,
    pub pak_change_pressed: bool,
}

fn bound_axis(x: &mut f64, y: &mut f64) {
    let radius = f64::sqrt(70.0 * 70.0 + 70.0 * 70.0); // this is roughly the maximum diagonal distance of the controller

    // Calculate the distance from the origin (0, 0)
    let distance = f64::sqrt((*x) * (*x) + (*y) * (*y));

    // If the distance is greater than the radius, scale the coordinates
    if distance > radius {
        let scale_factor = radius / distance;
        *x *= scale_factor;
        *y *= scale_factor;
    }
}

fn apply_deadzone(x: &mut f64, y: &mut f64, deadzone: i32) {
    let axis_deadzone = MAX_AXIS_VALUE * (deadzone as f64 / 100.0);

    // Calculate the distance from the origin (0, 0)
    let distance = f64::sqrt((*x) * (*x) + (*y) * (*y));

    if distance <= axis_deadzone {
        *x = 0.0;
        *y = 0.0;
        return;
    }

    let new_distance =
        (distance - axis_deadzone) * MAX_AXIS_VALUE / (MAX_AXIS_VALUE - axis_deadzone);
    *x = *x / distance * new_distance;
    *y = *y / distance * new_distance;
}

fn normalize_axis_position(axis_position: i16) -> f64 {
    axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64
}

fn set_axis_from_joystick(
    profile: &ui::config::InputProfile,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    if profile.joystick_axis[AXIS_LEFT].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_LEFT].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_LEFT].axis as isize > 0 {
            x = normalize_axis_position(axis_position);
        }
    }
    if profile.joystick_axis[AXIS_RIGHT].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_RIGHT].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_RIGHT].axis as isize > 0 {
            x = normalize_axis_position(axis_position);
        }
    }
    if profile.joystick_axis[AXIS_DOWN].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_DOWN].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_DOWN].axis as isize > 0 {
            y = normalize_axis_position(axis_position).neg();
        }
    }
    if profile.joystick_axis[AXIS_UP].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_UP].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_UP].axis as isize > 0 {
            y = normalize_axis_position(axis_position).neg();
        }
    }
    (x, y)
}

fn set_axis_from_controller(
    profile: &ui::config::InputProfile,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    if profile.controller_axis[AXIS_LEFT].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                sdl3_sys::gamepad::SDL_GamepadAxis(profile.controller_axis[AXIS_LEFT].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_LEFT].axis as isize > 0 {
            x = normalize_axis_position(axis_position);
        }
    }
    if profile.controller_axis[AXIS_RIGHT].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                sdl3_sys::gamepad::SDL_GamepadAxis(profile.controller_axis[AXIS_RIGHT].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_RIGHT].axis as isize > 0 {
            x = normalize_axis_position(axis_position);
        }
    }
    if profile.controller_axis[AXIS_DOWN].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                sdl3_sys::gamepad::SDL_GamepadAxis(profile.controller_axis[AXIS_DOWN].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_DOWN].axis as isize > 0 {
            y = normalize_axis_position(axis_position).neg();
        }
    }
    if profile.controller_axis[AXIS_UP].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                sdl3_sys::gamepad::SDL_GamepadAxis(profile.controller_axis[AXIS_UP].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_UP].axis as isize > 0 {
            y = normalize_axis_position(axis_position).neg();
        }
    }
    (x, y)
}

fn set_axis_from_keys(
    profile: &ui::config::InputProfile,
    keyboard_state: *const bool,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    if profile.keys[AXIS_LEFT].enabled
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_LEFT].id as isize) }
    {
        x = -MAX_AXIS_VALUE
    }
    if profile.keys[AXIS_RIGHT].enabled
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_RIGHT].id as isize) }
    {
        x = MAX_AXIS_VALUE
    }
    if profile.keys[AXIS_DOWN].enabled
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_DOWN].id as isize) }
    {
        y = -MAX_AXIS_VALUE
    }
    if profile.keys[AXIS_UP].enabled
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_UP].id as isize) }
    {
        y = MAX_AXIS_VALUE
    }
    (x, y)
}

fn set_buttons_from_joystick(
    profile: &ui::config::InputProfile,
    i: usize,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
    keys: &mut u32,
) {
    let profile_joystick_button = profile.joystick_buttons[i];
    if profile_joystick_button.enabled {
        *keys |= (unsafe {
            sdl3_sys::joystick::SDL_GetJoystickButton(joystick, profile_joystick_button.id)
        } as u32)
            << i;
    }

    let profile_joystick_hat = profile.joystick_hat[i];
    if profile_joystick_hat.enabled
        && (unsafe { sdl3_sys::joystick::SDL_GetJoystickHat(joystick, profile_joystick_hat.id) }
            & profile_joystick_hat.direction)
            != 0
    {
        *keys |= 1 << i;
    }

    let profile_joystick_axis = profile.joystick_axis[i];
    if profile_joystick_axis.enabled {
        let axis_position =
            unsafe { sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile_joystick_axis.id) };
        if (axis_position as isize * profile_joystick_axis.axis as isize > 0
            || profile_joystick_axis.initial_state != 0)
            && axis_position.abs_diff(profile_joystick_axis.initial_state) > (u16::MAX / 4)
        {
            *keys |= 1 << i;
        }
    }
}

fn set_buttons_from_controller(
    profile: &ui::config::InputProfile,
    i: usize,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
    keys: &mut u32,
) {
    let profile_controller_button = profile.controller_buttons[i];
    if profile_controller_button.enabled {
        *keys |= (unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadButton(
                controller,
                sdl3_sys::gamepad::SDL_GamepadButton(profile_controller_button.id),
            )
        } as u32)
            << i;
    }

    let profile_controller_axis = profile.controller_axis[i];
    if profile_controller_axis.enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                sdl3_sys::gamepad::SDL_GamepadAxis(profile_controller_axis.id),
            )
        };
        if axis_position as isize * profile_controller_axis.axis as isize > 0
            && axis_position.saturating_abs() > i16::MAX / 2
        {
            *keys |= 1 << i;
        }
    }
}

pub fn set_rumble(ui: &ui::Ui, channel: usize, rumble: u8) {
    if !ui.input.controllers[channel].rumble {
        return;
    }
    let controller = ui.input.controllers[channel].game_controller;
    let joystick = ui.input.controllers[channel].joystick;
    if !controller.is_null() {
        unsafe {
            sdl3_sys::gamepad::SDL_RumbleGamepad(
                controller,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u32 * sdl3_sys::haptic::SDL_HAPTIC_INFINITY,
            )
        };
    } else if !joystick.is_null() {
        unsafe {
            sdl3_sys::joystick::SDL_RumbleJoystick(
                joystick,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u32 * sdl3_sys::haptic::SDL_HAPTIC_INFINITY,
            )
        };
    }
}

fn hotkey_pressed(
    profile: &ui::config::InputProfile,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
) -> bool {
    let controller_button = profile.controller_buttons[HOTKEY];
    let controller_axis = profile.controller_axis[HOTKEY];
    let joystick_button = profile.joystick_buttons[HOTKEY];
    let joystick_hat = profile.joystick_hat[HOTKEY];
    let joystick_axis = profile.joystick_axis[HOTKEY];

    let mut pressed = false;
    if controller_button.enabled && !controller.is_null() {
        pressed = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadButton(
                controller,
                sdl3_sys::gamepad::SDL_GamepadButton(controller_button.id),
            )
        };
    } else if controller_axis.enabled && !controller.is_null() {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                sdl3_sys::gamepad::SDL_GamepadAxis(controller_axis.id),
            )
        };
        pressed = axis_position as isize * controller_axis.axis as isize > 0
            && axis_position.saturating_abs() > i16::MAX / 2
    } else if joystick_button.enabled && !joystick.is_null() {
        pressed =
            unsafe { sdl3_sys::joystick::SDL_GetJoystickButton(joystick, joystick_button.id) };
    } else if joystick_hat.enabled && !joystick.is_null() {
        pressed = (unsafe { sdl3_sys::joystick::SDL_GetJoystickHat(joystick, joystick_hat.id) }
            & joystick_hat.direction)
            != 0;
    } else if joystick_axis.enabled && !joystick.is_null() {
        let axis_position =
            unsafe { sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, joystick_axis.id) };
        pressed = (axis_position as isize * joystick_axis.axis as isize > 0
            || joystick_axis.initial_state != 0)
            && axis_position.abs_diff(joystick_axis.initial_state) > (u16::MAX / 4)
    }
    pressed
}

pub fn get_controller_names() -> Vec<String> {
    #[cfg(target_os = "android")]
    return ui::android::get_controller_names();

    #[cfg(not(target_os = "android"))]
    {
        let mut controllers: Vec<String> = vec![];

        for joystick in get_joysticks().iter() {
            let name = unsafe { sdl3_sys::joystick::SDL_GetJoystickNameForID(*joystick) };
            controllers.push(if name.is_null() {
                UNKNOWN_CONTROLLER_NAME.to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap() }.to_string()
            });
        }
        controllers.insert(0, "None".into());

        controllers
    }
}

#[cfg(feature = "gui")]
pub fn get_controller_paths() -> Vec<String> {
    #[cfg(target_os = "android")]
    return ui::android::get_controller_paths();

    #[cfg(not(target_os = "android"))]
    {
        let mut controller_paths: Vec<String> = vec![];

        for joystick in get_joysticks().iter() {
            let path = unsafe { sdl3_sys::joystick::SDL_GetJoystickPathForID(*joystick) };
            controller_paths.push(if path.is_null() {
                String::new()
            } else {
                unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() }.to_string()
            });
        }
        controller_paths.insert(0, String::new());

        controller_paths
    }
}

fn handle_joystick_events(ui: &mut ui::Ui) {
    let joystick_event = unsafe { ui::video::get_joystick_event() };
    if joystick_event.joystick_id != 0 {
        let joystick_id = sdl3_sys::joystick::SDL_JoystickID(joystick_event.joystick_id);
        for (i, controller) in ui.input.controllers.iter_mut().enumerate() {
            if joystick_event.connected {
                if let Some(profile) = ui
                    .config
                    .input
                    .input_profiles
                    .get(&ui.config.input.input_profile_binding[i])
                {
                    if profile.dinput {
                        if controller.joystick.is_null()
                            && controller.guid
                                == unsafe {
                                    sdl3_sys::joystick::SDL_GetJoystickGUIDForID(joystick_id)
                                }
                        {
                            controller.joystick =
                                unsafe { sdl3_sys::joystick::SDL_OpenJoystick(joystick_id) };
                        }
                    } else {
                        if controller.game_controller.is_null()
                            && controller.guid
                                == unsafe {
                                    sdl3_sys::gamepad::SDL_GetGamepadGUIDForID(joystick_id)
                                }
                        {
                            controller.game_controller =
                                unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(joystick_id) };
                        }
                    }
                }
            } else {
                if !controller.joystick.is_null()
                    && controller.joystick
                        == unsafe { sdl3_sys::joystick::SDL_GetJoystickFromID(joystick_id) }
                {
                    unsafe { sdl3_sys::joystick::SDL_CloseJoystick(controller.joystick) };
                    controller.joystick = std::ptr::null_mut();
                } else if !controller.game_controller.is_null()
                    && controller.game_controller
                        == unsafe { sdl3_sys::gamepad::SDL_GetGamepadFromID(joystick_id) }
                {
                    unsafe { sdl3_sys::gamepad::SDL_CloseGamepad(controller.game_controller) };
                    controller.game_controller = std::ptr::null_mut();
                }
            }
        }
    }
}

fn handle_hotkeys(keys: u32, last_key_state: u32) {
    if keys & (1 << L_TRIG) != 0 && last_key_state & (1 << L_TRIG) == 0 {
        unsafe {
            sdl3_sys::events::SDL_PushEvent(&mut sdl3_sys::events::SDL_Event {
                user: sdl3_sys::events::SDL_UserEvent {
                    r#type: u32::from(sdl3_sys::events::SDL_EVENT_USER),
                    code: 1, //save state
                    ..Default::default()
                },
            })
        };
    }
    if keys & (1 << R_TRIG) != 0 && last_key_state & (1 << R_TRIG) == 0 {
        unsafe {
            sdl3_sys::events::SDL_PushEvent(&mut sdl3_sys::events::SDL_Event {
                user: sdl3_sys::events::SDL_UserEvent {
                    r#type: u32::from(sdl3_sys::events::SDL_EVENT_USER),
                    code: 2, //load state
                    ..Default::default()
                },
            })
        };
    }
    if keys & (1 << START_BUTTON) != 0 && last_key_state & (1 << START_BUTTON) == 0 {
        unsafe {
            sdl3_sys::events::SDL_PushEvent(&mut sdl3_sys::events::SDL_Event {
                user: sdl3_sys::events::SDL_UserEvent {
                    r#type: u32::from(sdl3_sys::events::SDL_EVENT_USER),
                    code: 3, //exit game
                    ..Default::default()
                },
            })
        };
    }
    if keys & (1 << Z_TRIG) != 0 && last_key_state & (1 << Z_TRIG) == 0 {
        unsafe {
            sdl3_sys::events::SDL_PushEvent(&mut sdl3_sys::events::SDL_Event {
                user: sdl3_sys::events::SDL_UserEvent {
                    r#type: u32::from(sdl3_sys::events::SDL_EVENT_USER),
                    code: 4, //fast forward
                    ..Default::default()
                },
            })
        };
    }
    if keys & (1 << L_CBUTTON) != 0 && last_key_state & (1 << L_CBUTTON) == 0 {
        unsafe {
            sdl3_sys::events::SDL_PushEvent(&mut sdl3_sys::events::SDL_Event {
                user: sdl3_sys::events::SDL_UserEvent {
                    r#type: u32::from(sdl3_sys::events::SDL_EVENT_USER),
                    code: 5, //load rewind
                    ..Default::default()
                },
            })
        };
    }
}

pub fn get(ui: &mut ui::Ui, channel: usize, vi_counter: u64) -> InputData {
    handle_joystick_events(ui);

    if ui.input.last_polled != vi_counter {
        ui.input.last_polled = vi_counter;
        unsafe { sdl3_sys::joystick::SDL_UpdateJoysticks() };
    }

    let profile_name = &ui.config.input.input_profile_binding[channel];
    let Some(profile) = ui.config.input.input_profiles.get(profile_name) else {
        eprintln!("Invalid profile name: {profile_name}");
        return InputData {
            data: 0,
            pak_change_pressed: false,
        };
    };
    let mut keys = 0;
    let controller = ui.input.controllers[channel].game_controller;
    let joystick = ui.input.controllers[channel].joystick;

    let alt_pressed = unsafe {
        // ignore key presses if ALT is pressed
        *ui.input
            .keyboard_state
            .offset(i32::from(sdl3_sys::scancode::SDL_SCANCODE_LALT) as isize)
            || *ui
                .input
                .keyboard_state
                .offset(i32::from(sdl3_sys::scancode::SDL_SCANCODE_RALT) as isize)
    };

    for i in 0..14 {
        if profile_name != "default" || channel == 0 {
            let profile_key = profile.keys[i];
            if profile_key.enabled && !alt_pressed {
                keys |= (unsafe { *ui.input.keyboard_state.offset(profile_key.id as isize) }
                    as u32)
                    << i;
            }
        }

        if !controller.is_null() {
            set_buttons_from_controller(profile, i, controller, &mut keys);
        } else if !joystick.is_null() {
            set_buttons_from_joystick(profile, i, joystick, &mut keys);
        }
    }

    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;

    if profile_name != "default" || channel == 0 {
        (x, y) = set_axis_from_keys(profile, ui.input.keyboard_state);
    }

    if !controller.is_null() {
        (x, y) = set_axis_from_controller(profile, controller);
        apply_deadzone(&mut x, &mut y, profile.deadzone);
    } else if !joystick.is_null() {
        (x, y) = set_axis_from_joystick(profile, joystick);
        apply_deadzone(&mut x, &mut y, profile.deadzone);
    }
    bound_axis(&mut x, &mut y);

    keys |= (x.round() as i8 as u8 as u32) << X_AXIS_SHIFT;
    keys |= (y.round() as i8 as u8 as u32) << Y_AXIS_SHIFT;

    let last_key_state = ui.input.controllers[channel].last_key_state;
    ui.input.controllers[channel].last_key_state = keys;

    if hotkey_pressed(profile, joystick, controller) {
        handle_hotkeys(keys, last_key_state);
        InputData {
            data: 0,
            pak_change_pressed: keys & (1 << B_BUTTON) != 0,
        }
    } else {
        let key = profile.keys[HOTKEY];
        InputData {
            data: keys,
            pak_change_pressed: key.enabled
                && unsafe { *ui.input.keyboard_state.offset(key.id as isize) },
        }
    }
}

pub fn assign_controller(config: &mut ui::config::Config, controller: i32, port: usize) {
    let joysticks = get_joysticks();
    if controller < 0 {
        config.input.controller_assignment[port - 1] = None;
    } else if controller < joysticks.len() as i32 {
        let path =
            unsafe { sdl3_sys::joystick::SDL_GetJoystickPathForID(joysticks[controller as usize]) };
        if !path.is_null() {
            config.input.controller_assignment[port - 1] =
                Some(unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap().to_string() });
        } else {
            eprintln!("Invalid controller path for controller {controller}");
        }
    } else {
        eprintln!("Invalid controller number")
    }
}

pub fn bind_input_profile(config: &mut ui::config::Config, profile: String, port: usize) {
    if config.input.input_profiles.contains_key(&profile) {
        config.input.input_profile_binding[port - 1] = profile;
    } else {
        eprintln!("Invalid profile name")
    }
}

pub fn clear_bindings(config: &mut ui::config::Config) {
    for i in 0..4 {
        config.input.controller_assignment[i] = None;
        config.input.input_profile_binding[i] = "default".to_string();
    }
}

#[cfg(feature = "gui")]
fn clear_profile_input(
    value: usize,
    keys: &mut [ui::config::InputKeyButton; PROFILE_SIZE],
    controller_buttons: &mut [ui::config::InputKeyButton; PROFILE_SIZE],
    controller_axis: &mut [ui::config::InputControllerAxis; PROFILE_SIZE],
    joystick_buttons: &mut [ui::config::InputKeyButton; PROFILE_SIZE],
    joystick_hat: &mut [ui::config::InputJoystickHat; PROFILE_SIZE],
    joystick_axis: &mut [ui::config::InputControllerAxis; PROFILE_SIZE],
) {
    keys[value].enabled = false;
    controller_buttons[value].enabled = false;
    controller_axis[value].enabled = false;
    joystick_buttons[value].enabled = false;
    joystick_hat[value].enabled = false;
    joystick_axis[value].enabled = false;
}

#[cfg(feature = "gui")]
fn binding_label(
    value: usize,
    keys: &[ui::config::InputKeyButton; PROFILE_SIZE],
    controller_buttons: &[ui::config::InputKeyButton; PROFILE_SIZE],
    controller_axis: &[ui::config::InputControllerAxis; PROFILE_SIZE],
    joystick_buttons: &[ui::config::InputKeyButton; PROFILE_SIZE],
    joystick_hat: &[ui::config::InputJoystickHat; PROFILE_SIZE],
    joystick_axis: &[ui::config::InputControllerAxis; PROFILE_SIZE],
) -> String {
    if keys[value].enabled {
        format!("Key {}", keys[value].id)
    } else if controller_buttons[value].enabled {
        format!("Button {}", controller_buttons[value].id)
    } else if controller_axis[value].enabled {
        let sign = if controller_axis[value].axis >= 0 {
            "+"
        } else {
            "-"
        };
        format!("Axis {}{sign}", controller_axis[value].id)
    } else if joystick_buttons[value].enabled {
        format!("Joystick Button {}", joystick_buttons[value].id)
    } else if joystick_hat[value].enabled {
        format!("Hat {}", joystick_hat[value].id)
    } else if joystick_axis[value].enabled {
        let sign = if joystick_axis[value].axis >= 0 {
            "+"
        } else {
            "-"
        };
        format!("Joystick Axis {}{sign}", joystick_axis[value].id)
    } else {
        "(unset)".to_string()
    }
}

/// Display labels for the review list / capture prompt, in wizard order.
#[cfg(feature = "gui")]
pub(crate) const KEY_LABELS: [(&str, usize); PROFILE_SIZE] = [
    ("A", A_BUTTON),
    ("B", B_BUTTON),
    ("Start", START_BUTTON),
    ("D Up", U_DPAD),
    ("D Down", D_DPAD),
    ("D Left", L_DPAD),
    ("D Right", R_DPAD),
    ("C Up", U_CBUTTON),
    ("C Down", D_CBUTTON),
    ("C Left", L_CBUTTON),
    ("C Right", R_CBUTTON),
    ("L", L_TRIG),
    ("R", R_TRIG),
    ("Z", Z_TRIG),
    ("Control Stick Up", AXIS_UP),
    ("Control Stick Down", AXIS_DOWN),
    ("Control Stick Left", AXIS_LEFT),
    ("Control Stick Right", AXIS_RIGHT),
    ("Hotkey Activator", HOTKEY),
];

/// The "Save & Exit" row sits one past the last input in the review list.
#[cfg(feature = "gui")]
pub(crate) const SAVE_ROW: usize = PROFILE_SIZE;

#[cfg(feature = "gui")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Screen {
    /// Reviewing the input list (navigable).
    List,
    /// Listening for an input to bind.
    Capture,
}

/// Device-agnostic navigation intents. Raw keyboard/gamepad/joystick/touch
/// events all decode to these before reaching `advance`.
#[cfg(feature = "gui")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
    Up,
    Down,
    /// Enter / gamepad South / joystick button 0 / tap a row.
    Confirm,
    /// Esc / gamepad East / on-screen ✕. Skips in capture, quits in list.
    Cancel,
    /// Close request from the wizard shell (e.g. window close). Honors the
    /// dirty-quit guard like `Cancel`, but never merely skips a capture.
    Quit,
    /// A raw input was successfully captured (caller performed the binding).
    Bound,
}

#[cfg(feature = "gui")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MenuState {
    pub(crate) screen: Screen,
    /// True during the guided flow (auto-advance through every input).
    pub(crate) guided: bool,
    /// List: row `0..=SAVE_ROW`. Capture: the input row `0..PROFILE_SIZE` being bound.
    pub(crate) selected: usize,
    /// A binding has changed since the window opened.
    pub(crate) dirty: bool,
    /// A dirty quit has been requested once; a second quit discards.
    pub(crate) quit_armed: bool,
}

#[cfg(feature = "gui")]
impl MenuState {
    pub(crate) fn entry(existing_profile: bool) -> Self {
        MenuState {
            // New/empty profile → jump straight into the guided flow;
            // editing an existing profile → start on the review list.
            screen: if existing_profile {
                Screen::List
            } else {
                Screen::Capture
            },
            guided: !existing_profile,
            selected: 0,
            dirty: false,
            quit_armed: false,
        }
    }
}

#[cfg(feature = "gui")]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct Transition {
    /// Close the window.
    pub(crate) exit: bool,
    /// Persist the profile (only true alongside `exit`).
    pub(crate) save: bool,
    /// Begin a fresh capture listen (caller debounces first).
    pub(crate) begin_capture: bool,
}

/// Pure state transition. `action` is the decoded intent; returns side-effect
/// signals for the platform shell to act on.
#[cfg(feature = "gui")]
pub(crate) fn advance(state: &mut MenuState, action: Action) -> Transition {
    let mut t = Transition::default();
    // Any non-quit-like action disarms a pending dirty-quit.
    if action != Action::Quit && action != Action::Cancel {
        state.quit_armed = false;
    }
    match state.screen {
        Screen::List => match action {
            Action::Up => state.selected = (state.selected + SAVE_ROW) % (SAVE_ROW + 1),
            Action::Down => state.selected = (state.selected + 1) % (SAVE_ROW + 1),
            Action::Confirm => {
                if state.selected == SAVE_ROW {
                    t.save = true;
                    t.exit = true;
                } else {
                    state.screen = Screen::Capture;
                    state.guided = false;
                    t.begin_capture = true;
                }
            }
            // Esc/East in the list quits (with the dirty guard).
            Action::Cancel | Action::Quit => {
                if state.dirty && !state.quit_armed {
                    state.quit_armed = true;
                } else {
                    t.exit = true;
                }
            }
            Action::Bound => {}
        },
        Screen::Capture => match action {
            Action::Bound | Action::Cancel => {
                if action == Action::Bound {
                    state.dirty = true;
                }
                if state.guided {
                    if state.selected + 1 >= PROFILE_SIZE {
                        state.screen = Screen::List;
                        state.guided = false;
                        state.selected = 0;
                    } else {
                        state.selected += 1;
                        t.begin_capture = true;
                    }
                } else {
                    state.screen = Screen::List;
                }
            }
            Action::Quit => {
                if state.dirty && !state.quit_armed {
                    state.quit_armed = true;
                    state.screen = Screen::List;
                } else {
                    t.exit = true;
                }
            }
            Action::Up | Action::Down | Action::Confirm => {}
        },
    }
    t
}

/// Glow location for an input as fractions of the 852×480 logical canvas:
/// `(center_x, center_y, radius)` where radius is a fraction of canvas width.
/// `None` for inputs with no controller-art location (the hotkey activator).
#[cfg(feature = "gui")]
pub(crate) fn glow_center(input: usize) -> Option<(f32, f32, f32)> {
    let spot = match input {
        A_BUTTON => (0.715, 0.595, 0.045),
        B_BUTTON => (0.655, 0.495, 0.045),
        START_BUTTON => (0.505, 0.515, 0.040),
        U_DPAD => (0.215, 0.355, 0.030),
        D_DPAD => (0.215, 0.525, 0.030),
        L_DPAD => (0.165, 0.445, 0.030),
        R_DPAD => (0.265, 0.445, 0.030),
        U_CBUTTON => (0.800, 0.320, 0.028),
        D_CBUTTON => (0.800, 0.500, 0.028),
        L_CBUTTON => (0.755, 0.415, 0.028),
        R_CBUTTON => (0.845, 0.415, 0.028),
        L_TRIG => (0.220, 0.110, 0.048),
        R_TRIG => (0.780, 0.110, 0.048),
        Z_TRIG => (0.500, 0.630, 0.042),
        AXIS_UP | AXIS_DOWN | AXIS_LEFT | AXIS_RIGHT => (0.475, 0.755, 0.060),
        _ => return None, // HOTKEY and any future input
    };
    Some(spot)
}

/// The six per-input binding arrays, bundled so the capture helpers stay within
/// clippy's argument budget. Field-disjoint borrows keep `clear_profile_input`
/// and `binding_label` callable on individual arrays.
#[cfg(feature = "gui")]
pub(crate) struct Bindings {
    keys: [ui::config::InputKeyButton; PROFILE_SIZE],
    controller_buttons: [ui::config::InputKeyButton; PROFILE_SIZE],
    controller_axis: [ui::config::InputControllerAxis; PROFILE_SIZE],
    joystick_buttons: [ui::config::InputKeyButton; PROFILE_SIZE],
    joystick_hat: [ui::config::InputJoystickHat; PROFILE_SIZE],
    joystick_axis: [ui::config::InputControllerAxis; PROFILE_SIZE],
}

#[cfg(feature = "gui")]
impl Bindings {
    pub(crate) fn empty() -> Self {
        Bindings {
            keys: [ui::config::InputKeyButton {
                enabled: false,
                id: 0,
            }; PROFILE_SIZE],
            controller_buttons: [ui::config::InputKeyButton {
                enabled: false,
                id: 0,
            }; PROFILE_SIZE],
            controller_axis: [ui::config::InputControllerAxis {
                enabled: false,
                id: 0,
                axis: 0,
                initial_state: 0,
            }; PROFILE_SIZE],
            joystick_buttons: [ui::config::InputKeyButton {
                enabled: false,
                id: 0,
            }; PROFILE_SIZE],
            joystick_hat: [ui::config::InputJoystickHat {
                enabled: false,
                id: 0,
                direction: 0,
            }; PROFILE_SIZE],
            joystick_axis: [ui::config::InputControllerAxis {
                enabled: false,
                id: 0,
                axis: 0,
                initial_state: 0,
            }; PROFILE_SIZE],
        }
    }

    /// Bindings preloaded from an existing profile, so editing only
    /// re-captures the inputs the user picks.
    pub(crate) fn from_profile(profile: &ui::config::InputProfile) -> Self {
        Bindings {
            keys: profile.keys,
            controller_buttons: profile.controller_buttons,
            controller_axis: profile.controller_axis,
            joystick_buttons: profile.joystick_buttons,
            joystick_hat: profile.joystick_hat,
            joystick_axis: profile.joystick_axis,
        }
    }

    /// Assemble the persisted profile, exactly as the old SDL window saved it.
    pub(crate) fn to_profile(&self, dinput: bool, deadzone: i32) -> ui::config::InputProfile {
        ui::config::InputProfile {
            keys: self.keys,
            controller_buttons: self.controller_buttons,
            controller_axis: self.controller_axis,
            joystick_buttons: self.joystick_buttons,
            joystick_hat: self.joystick_hat,
            joystick_axis: self.joystick_axis,
            dinput,
            deadzone,
        }
    }

    /// Store one captured input for `value`: clear every per-device slot, then
    /// set the matching array — the same clear+set the old `wait_capture` did.
    pub(crate) fn bind(&mut self, value: usize, ev: ui::input_capture::CaptureEvent) {
        use ui::input_capture::CaptureEvent;
        clear_profile_input(
            value,
            &mut self.keys,
            &mut self.controller_buttons,
            &mut self.controller_axis,
            &mut self.joystick_buttons,
            &mut self.joystick_hat,
            &mut self.joystick_axis,
        );
        match ev {
            CaptureEvent::Key(id) => {
                self.keys[value] = ui::config::InputKeyButton { enabled: true, id };
            }
            CaptureEvent::GamepadButton(id) => {
                self.controller_buttons[value] = ui::config::InputKeyButton { enabled: true, id };
            }
            CaptureEvent::GamepadAxis { id, sign } => {
                self.controller_axis[value] = ui::config::InputControllerAxis {
                    enabled: true,
                    id,
                    axis: sign,
                    initial_state: 0,
                };
            }
            CaptureEvent::JoyButton(id) => {
                self.joystick_buttons[value] = ui::config::InputKeyButton { enabled: true, id };
            }
            #[cfg(not(target_os = "android"))]
            CaptureEvent::JoyHat { id, direction } => {
                self.joystick_hat[value] = ui::config::InputJoystickHat {
                    enabled: true,
                    id,
                    direction,
                };
            }
            CaptureEvent::JoyAxis {
                id,
                sign,
                initial_state,
            } => {
                self.joystick_axis[value] = ui::config::InputControllerAxis {
                    enabled: true,
                    id,
                    axis: sign,
                    initial_state,
                };
            }
        }
    }

    /// Human-readable binding for the review list ("Key 4", "(unset)", ...).
    pub(crate) fn label(&self, value: usize) -> String {
        binding_label(
            value,
            &self.keys,
            &self.controller_buttons,
            &self.controller_axis,
            &self.joystick_buttons,
            &self.joystick_hat,
            &self.joystick_axis,
        )
    }

    /// Whether any device slot is bound for `value`.
    pub(crate) fn is_bound(&self, value: usize) -> bool {
        self.keys[value].enabled
            || self.controller_buttons[value].enabled
            || self.controller_axis[value].enabled
            || self.joystick_buttons[value].enabled
            || self.joystick_hat[value].enabled
            || self.joystick_axis[value].enabled
    }
}

/// Sign (+1 / -1) of an axis deflection relative to its resting state.
/// Avoids the 0/0 panic of `v / v.abs()` and never returns 0 (a 0 sign would
/// dead-bind the input, since in-game uses `axis_position * axis > 0`).
#[cfg(feature = "gui")]
pub(crate) fn axis_sign(axis_value: i16, initial_state: i16) -> i16 {
    if axis_value >= initial_state { 1 } else { -1 }
}

pub fn get_default_profile() -> ui::config::InputProfile {
    let mut default_controller_buttons = [ui::config::InputKeyButton {
        enabled: false,
        id: 0,
    }; PROFILE_SIZE];
    let mut default_controller_axis = [ui::config::InputControllerAxis {
        enabled: false,
        id: 0,
        axis: 0,
        initial_state: 0,
    }; PROFILE_SIZE];
    let mut default_keys = [ui::config::InputKeyButton {
        enabled: false,
        id: 0,
    }; PROFILE_SIZE];
    default_keys[R_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_D),
    };
    default_keys[L_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_A),
    };
    default_keys[D_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_S),
    };
    default_keys[U_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_W),
    };
    default_keys[START_BUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_RETURN),
    };
    default_keys[Z_TRIG] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_Z),
    };
    default_keys[B_BUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_LCTRL),
    };
    default_keys[A_BUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_LSHIFT),
    };
    default_keys[R_CBUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_L),
    };
    default_keys[L_CBUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_J),
    };
    default_keys[D_CBUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_K),
    };
    default_keys[U_CBUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_I),
    };
    default_keys[R_TRIG] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_C),
    };
    default_keys[L_TRIG] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_X),
    };
    default_keys[AXIS_LEFT] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_LEFT),
    };
    default_keys[AXIS_RIGHT] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_RIGHT),
    };
    default_keys[AXIS_UP] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_UP),
    };
    default_keys[AXIS_DOWN] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_DOWN),
    };
    default_keys[HOTKEY] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_COMMA),
    };

    default_controller_buttons[R_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_RIGHT),
    };
    default_controller_buttons[L_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_LEFT),
    };
    default_controller_buttons[D_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_DOWN),
    };
    default_controller_buttons[U_DPAD] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_UP),
    };
    default_controller_buttons[START_BUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_START),
    };
    default_controller_axis[Z_TRIG] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFT_TRIGGER),
        axis: 1,
        initial_state: 0,
    };
    default_controller_buttons[B_BUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_WEST),
    };
    default_controller_buttons[A_BUTTON] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_SOUTH),
    };
    default_controller_axis[R_CBUTTON] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTX),
        axis: 1,
        initial_state: 0,
    };
    default_controller_axis[L_CBUTTON] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTX),
        axis: -1,
        initial_state: 0,
    };
    default_controller_axis[D_CBUTTON] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
        axis: 1,
        initial_state: 0,
    };
    default_controller_axis[U_CBUTTON] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
        axis: -1,
        initial_state: 0,
    };
    default_controller_buttons[R_TRIG] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER),
    };
    default_controller_buttons[L_TRIG] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_LEFT_SHOULDER),
    };
    default_controller_axis[AXIS_LEFT] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX),
        axis: -1,
        initial_state: 0,
    };
    default_controller_axis[AXIS_RIGHT] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX),
        axis: 1,
        initial_state: 0,
    };
    default_controller_axis[AXIS_UP] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
        axis: -1,
        initial_state: 0,
    };
    default_controller_axis[AXIS_DOWN] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
        axis: 1,
        initial_state: 0,
    };
    default_controller_buttons[HOTKEY] = ui::config::InputKeyButton {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_BACK),
    };

    ui::config::InputProfile {
        keys: default_keys,
        controller_buttons: default_controller_buttons,
        controller_axis: default_controller_axis,
        joystick_buttons: [ui::config::InputKeyButton {
            enabled: false,
            id: 0,
        }; PROFILE_SIZE],
        joystick_hat: [ui::config::InputJoystickHat {
            enabled: false,
            id: 0,
            direction: 0,
        }; PROFILE_SIZE],
        joystick_axis: [ui::config::InputControllerAxis {
            enabled: false,
            id: 0,
            axis: 0,
            initial_state: 0,
        }; PROFILE_SIZE],
        dinput: false,
        deadzone: DEADZONE_DEFAULT,
    }
}

pub(crate) fn get_joysticks() -> Vec<sdl3_sys::joystick::SDL_JoystickID> {
    unsafe { sdl3_sys::events::SDL_PumpEvents() };
    let mut num_joysticks = 0;
    let sdl_joysticks = unsafe { sdl3_sys::joystick::SDL_GetJoysticks(&mut num_joysticks) };
    if !sdl_joysticks.is_null() {
        let parts =
            unsafe { std::slice::from_raw_parts(sdl_joysticks, num_joysticks as usize) }.to_vec();
        unsafe { sdl3_sys::stdinc::SDL_free(sdl_joysticks as *mut std::ffi::c_void) };
        parts
    } else {
        eprintln!("Could not get joysticks");
        vec![]
    }
}

pub fn init(ui: &mut ui::Ui) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);

    ui.input.keyboard_state =
        unsafe { sdl3_sys::keyboard::SDL_GetKeyboardState(std::ptr::null_mut()) };
    if ui.input.keyboard_state.is_null() {
        panic!("Could not get keyboard state");
    }

    for i in 0..4 {
        if let Some(controller_assignment) = &ui.config.input.controller_assignment[i]
            && ui.config.input.controller_enabled[i]
        {
            let mut joystick_id = sdl3_sys::everything::SDL_JoystickID(0);

            for joystick in get_joysticks().iter() {
                let path = if cfg!(target_os = "android") {
                    let name = if let name =
                        unsafe { sdl3_sys::joystick::SDL_GetJoystickNameForID(*joystick) }
                        && !name.is_null()
                    {
                        unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap() }.to_string()
                    } else {
                        UNKNOWN_CONTROLLER_NAME.to_string()
                    };

                    let vendor_id =
                        unsafe { sdl3_sys::joystick::SDL_GetJoystickVendorForID(*joystick) };
                    let product_id =
                        unsafe { sdl3_sys::joystick::SDL_GetJoystickProductForID(*joystick) };
                    Some(format!("{}:{}:{}", name, vendor_id, product_id))
                } else {
                    let path = unsafe { sdl3_sys::joystick::SDL_GetJoystickPathForID(*joystick) };
                    if !path.is_null() {
                        Some(
                            unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() }.to_string(),
                        )
                    } else {
                        None
                    }
                };
                if let Some(path) = path
                    && path == *controller_assignment
                    && unsafe { sdl3_sys::joystick::SDL_GetJoystickFromID(*joystick) }.is_null()
                    && unsafe { sdl3_sys::gamepad::SDL_GetGamepadFromID(*joystick) }.is_null()
                {
                    joystick_id = *joystick;
                    break;
                }
            }

            if joystick_id != 0
                && let Some(profile) = ui
                    .config
                    .input
                    .input_profiles
                    .get(&ui.config.input.input_profile_binding[i])
            {
                if !profile.dinput {
                    let gamepad = unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(joystick_id) };
                    if gamepad.is_null() {
                        eprintln!("could not connect gamepad: {}", u32::from(joystick_id))
                    } else {
                        ui.input.controllers[i].game_controller = gamepad;
                        ui.input.controllers[i].guid =
                            unsafe { sdl3_sys::gamepad::SDL_GetGamepadGUIDForID(joystick_id) };
                        let properties =
                            unsafe { sdl3_sys::gamepad::SDL_GetGamepadProperties(gamepad) };
                        if properties == 0 {
                            eprintln!("could not get gamepad properties");
                        }
                        ui.input.controllers[i].rumble = unsafe {
                            sdl3_sys::properties::SDL_GetBooleanProperty(
                                properties,
                                sdl3_sys::gamepad::SDL_PROP_GAMEPAD_CAP_RUMBLE_BOOLEAN,
                                false,
                            )
                        };
                    }
                } else {
                    let joystick = unsafe { sdl3_sys::joystick::SDL_OpenJoystick(joystick_id) };
                    if joystick.is_null() {
                        eprintln!("could not connect joystick: {}", u32::from(joystick_id))
                    } else {
                        ui.input.controllers[i].joystick = joystick;
                        ui.input.controllers[i].guid =
                            unsafe { sdl3_sys::joystick::SDL_GetJoystickGUIDForID(joystick_id) };
                        let properties =
                            unsafe { sdl3_sys::joystick::SDL_GetJoystickProperties(joystick) };
                        if properties == 0 {
                            eprintln!("could not get joystick properties");
                        }
                        ui.input.controllers[i].rumble = unsafe {
                            sdl3_sys::properties::SDL_GetBooleanProperty(
                                properties,
                                sdl3_sys::joystick::SDL_PROP_JOYSTICK_CAP_RUMBLE_BOOLEAN,
                                false,
                            )
                        };
                    }
                }
            } else {
                eprintln!("Could not bind assigned controller");
            }
        }
    }
}

pub fn close(ui: &mut ui::Ui) {
    for controller in ui.input.controllers.iter_mut() {
        if !controller.joystick.is_null() {
            unsafe { sdl3_sys::joystick::SDL_CloseJoystick(controller.joystick) }
            controller.joystick = std::ptr::null_mut();
        }
        if !controller.game_controller.is_null() {
            unsafe { sdl3_sys::gamepad::SDL_CloseGamepad(controller.game_controller) }
            controller.game_controller = std::ptr::null_mut();
        }
    }
}

#[cfg(all(test, feature = "gui"))]
mod menu_tests {
    use super::*;

    #[test]
    fn entry_new_profile_starts_guided_capture() {
        let s = MenuState::entry(false);
        assert_eq!(s.screen, Screen::Capture);
        assert!(s.guided);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn entry_existing_profile_starts_list() {
        let s = MenuState::entry(true);
        assert_eq!(s.screen, Screen::List);
        assert!(!s.guided);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn list_up_wraps_to_save_row() {
        let mut s = MenuState::entry(true);
        advance(&mut s, Action::Up);
        assert_eq!(s.selected, SAVE_ROW);
    }

    #[test]
    fn list_down_from_save_row_wraps_to_zero() {
        let mut s = MenuState::entry(true);
        s.selected = SAVE_ROW;
        advance(&mut s, Action::Down);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn list_confirm_on_input_row_enters_single_capture() {
        let mut s = MenuState::entry(true);
        s.selected = 3;
        let t = advance(&mut s, Action::Confirm);
        assert_eq!(s.screen, Screen::Capture);
        assert!(!s.guided);
        assert_eq!(s.selected, 3);
        assert!(t.begin_capture);
        assert!(!t.save && !t.exit);
    }

    #[test]
    fn list_confirm_on_save_row_saves_and_exits() {
        let mut s = MenuState::entry(true);
        s.selected = SAVE_ROW;
        let t = advance(&mut s, Action::Confirm);
        assert!(t.save && t.exit);
    }

    #[test]
    fn single_capture_bound_returns_to_list_same_row() {
        let mut s = MenuState::entry(true);
        s.selected = 5;
        advance(&mut s, Action::Confirm); // -> capture
        let t = advance(&mut s, Action::Bound);
        assert_eq!(s.screen, Screen::List);
        assert_eq!(s.selected, 5);
        assert!(s.dirty);
        assert!(!t.begin_capture);
    }

    #[test]
    fn guided_bound_advances_and_re_captures() {
        let mut s = MenuState::entry(false); // guided, selected 0
        let t = advance(&mut s, Action::Bound);
        assert_eq!(s.screen, Screen::Capture);
        assert!(s.guided);
        assert_eq!(s.selected, 1);
        assert!(t.begin_capture);
        assert!(s.dirty);
    }

    #[test]
    fn guided_skip_advances_without_dirty() {
        let mut s = MenuState::entry(false);
        let t = advance(&mut s, Action::Cancel);
        assert_eq!(s.selected, 1);
        assert!(s.screen == Screen::Capture && s.guided);
        assert!(t.begin_capture);
        assert!(!s.dirty);
    }

    #[test]
    fn guided_past_last_input_drops_to_list() {
        let mut s = MenuState::entry(false);
        s.selected = PROFILE_SIZE - 1; // last input
        advance(&mut s, Action::Bound);
        assert_eq!(s.screen, Screen::List);
        assert!(!s.guided);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn clean_quit_exits_immediately() {
        let mut s = MenuState::entry(true);
        let t = advance(&mut s, Action::Quit);
        assert!(t.exit);
    }

    #[test]
    fn dirty_quit_arms_then_discards_on_second() {
        let mut s = MenuState::entry(true);
        s.dirty = true;
        let t1 = advance(&mut s, Action::Quit);
        assert!(!t1.exit);
        assert!(s.quit_armed);
        let t2 = advance(&mut s, Action::Quit);
        assert!(t2.exit);
    }

    #[test]
    fn any_action_disarms_quit() {
        let mut s = MenuState::entry(true);
        s.dirty = true;
        advance(&mut s, Action::Quit); // arm
        advance(&mut s, Action::Down); // disarm
        assert!(!s.quit_armed);
        let t = advance(&mut s, Action::Quit); // arms again, does not exit
        assert!(!t.exit);
        assert!(s.quit_armed);
    }

    #[test]
    fn every_pad_input_has_an_in_bounds_glow() {
        for input in 0..PROFILE_SIZE {
            if input == HOTKEY {
                assert!(glow_center(input).is_none(), "hotkey has no pad location");
                continue;
            }
            let (cx, cy, r) = glow_center(input).expect("pad input must glow");
            assert!((0.0..=1.0).contains(&cx) && (0.0..=1.0).contains(&cy));
            assert!(r > 0.0 && r < 0.2);
            // rect stays within the canvas
            assert!(cx - r >= 0.0 && cx + r <= 1.0);
        }
    }

    #[test]
    fn list_cancel_when_dirty_arms_then_exits() {
        let mut s = MenuState::entry(true);
        s.dirty = true;
        let t1 = advance(&mut s, Action::Cancel);
        assert!(!t1.exit);
        assert!(s.quit_armed);
        let t2 = advance(&mut s, Action::Cancel);
        assert!(t2.exit);
    }

    #[test]
    fn single_capture_cancel_returns_to_list_without_dirty() {
        let mut s = MenuState::entry(true);
        s.selected = 6;
        advance(&mut s, Action::Confirm); // -> single capture
        let t = advance(&mut s, Action::Cancel);
        assert_eq!(s.screen, Screen::List);
        assert_eq!(s.selected, 6);
        assert!(!s.dirty);
        assert!(!t.begin_capture);
    }

    #[test]
    fn axis_sign_never_zero_and_matches_direction() {
        assert_eq!(axis_sign(0, 0), 1); // the old 0/0 panic case
        assert_eq!(axis_sign(30000, 0), 1);
        assert_eq!(axis_sign(-30000, 0), -1);
        assert_eq!(axis_sign(0, i16::MIN), 1); // MIN-resting axis reaching 0 → positive
        assert_eq!(axis_sign(0, i16::MAX), -1); // MAX-resting axis reaching 0 → negative
        assert_ne!(axis_sign(0, 0), 0);
    }
}
