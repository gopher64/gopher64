use std::ops::Neg;

use crate::ui;

const X_AXIS_SHIFT: usize = 16;
const Y_AXIS_SHIFT: usize = 24;

const MAX_AXIS_VALUE: f64 = 85.0;

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

fn set_axis(
    profile: &ui::config::InputProfile,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
    keyboard_state: *const bool,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let axes = [
        ui::input_profile::AXIS_LEFT,
        ui::input_profile::AXIS_RIGHT,
        ui::input_profile::AXIS_DOWN,
        ui::input_profile::AXIS_UP,
    ];
    let mut has_deadzone = false;
    for axis in axes {
        for input in profile.inputs[axis].iter() {
            if !controller.is_null()
                && let Some(ui::config::InputItem::ControllerAxis(controller_axis)) = input
            {
                let axis_position = unsafe {
                    sdl3_sys::gamepad::SDL_GetGamepadAxis(
                        controller,
                        sdl3_sys::gamepad::SDL_GamepadAxis(controller_axis.id),
                    )
                };
                if axis_position as isize * controller_axis.axis as isize > 0 {
                    let axis_value = if axis == ui::input_profile::AXIS_LEFT
                        || axis == ui::input_profile::AXIS_RIGHT
                    {
                        &mut x
                    } else {
                        &mut y
                    };
                    *axis_value = normalize_axis_position(axis_position);
                    has_deadzone = true;
                }
            } else if !joystick.is_null()
                && let Some(ui::config::InputItem::JoystickAxis(joystick_axis)) = input
            {
                let axis_position =
                    unsafe { sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, joystick_axis.id) };
                if axis_position as isize * joystick_axis.axis as isize > 0 {
                    let axis_value = if axis == ui::input_profile::AXIS_LEFT
                        || axis == ui::input_profile::AXIS_RIGHT
                    {
                        &mut x
                    } else {
                        &mut y
                    };
                    *axis_value = normalize_axis_position(axis_position);
                    has_deadzone = true;
                }
            } else if let Some(ui::config::InputItem::Key(key)) = input
                && unsafe { *keyboard_state.offset(key.id as isize) }
            {
                match axis {
                    ui::input_profile::AXIS_LEFT => x = -MAX_AXIS_VALUE,
                    ui::input_profile::AXIS_RIGHT => x = MAX_AXIS_VALUE,
                    ui::input_profile::AXIS_DOWN => y = MAX_AXIS_VALUE,
                    ui::input_profile::AXIS_UP => y = -MAX_AXIS_VALUE,
                    _ => unreachable!(),
                }
            }
        }
    }

    y = y.neg();
    if has_deadzone {
        apply_deadzone(&mut x, &mut y, profile.deadzone);
    }
    (x, y)
}

fn is_controller_button_pressed(
    input: &Option<ui::config::InputItem>,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
) -> bool {
    if !controller.is_null() {
        if let Some(ui::config::InputItem::ControllerButton(controller_button)) = input
            && unsafe {
                sdl3_sys::gamepad::SDL_GetGamepadButton(
                    controller,
                    sdl3_sys::gamepad::SDL_GamepadButton(controller_button.id),
                )
            }
        {
            return true;
        }
        if let Some(ui::config::InputItem::ControllerAxis(controller_axis)) = input {
            let axis_position = unsafe {
                sdl3_sys::gamepad::SDL_GetGamepadAxis(
                    controller,
                    sdl3_sys::gamepad::SDL_GamepadAxis(controller_axis.id),
                )
            };
            if axis_position as isize * controller_axis.axis as isize > 0
                && axis_position.saturating_abs() > i16::MAX / 2
            {
                return true;
            }
        }
    } else if !joystick.is_null() {
        if let Some(ui::config::InputItem::JoystickButton(joystick_button)) = input
            && unsafe { sdl3_sys::joystick::SDL_GetJoystickButton(joystick, joystick_button.id) }
        {
            return true;
        }
        if let Some(ui::config::InputItem::JoystickHat(joystick_hat)) = input
            && (unsafe { sdl3_sys::joystick::SDL_GetJoystickHat(joystick, joystick_hat.id) }
                & joystick_hat.direction)
                != 0
        {
            return true;
        }
        if let Some(ui::config::InputItem::JoystickAxis(joystick_axis)) = input {
            let axis_position =
                unsafe { sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, joystick_axis.id) };
            if (axis_position as isize * joystick_axis.axis as isize > 0
                || joystick_axis.initial_state != 0)
                && axis_position.abs_diff(joystick_axis.initial_state) > (u16::MAX / 4)
            {
                return true;
            }
        }
    }
    false
}

fn set_buttons(
    profile: &ui::config::InputProfile,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
    keyboard_state: *const bool,
    alt_pressed: bool,
) -> u32 {
    let mut keys = 0;
    for i in 0..14 {
        for input in profile.inputs[i].iter() {
            if let Some(ui::config::InputItem::Key(key)) = input
                && !alt_pressed
                && unsafe { *keyboard_state.offset(key.id as isize) }
            {
                keys |= 1 << i;
            }
            if is_controller_button_pressed(input, joystick, controller) {
                keys |= 1 << i;
            }
        }
    }
    keys
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
    for input in profile.inputs[ui::input_profile::HOTKEY].iter() {
        if is_controller_button_pressed(input, joystick, controller) {
            return true;
        }
    }
    false
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
    if keys & (1 << ui::input_profile::L_TRIG) != 0
        && last_key_state & (1 << ui::input_profile::L_TRIG) == 0
    {
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
    if keys & (1 << ui::input_profile::R_TRIG) != 0
        && last_key_state & (1 << ui::input_profile::R_TRIG) == 0
    {
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
    if keys & (1 << ui::input_profile::START_BUTTON) != 0
        && last_key_state & (1 << ui::input_profile::START_BUTTON) == 0
    {
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
    if keys & (1 << ui::input_profile::Z_TRIG) != 0
        && last_key_state & (1 << ui::input_profile::Z_TRIG) == 0
    {
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
    if keys & (1 << ui::input_profile::L_CBUTTON) != 0
        && last_key_state & (1 << ui::input_profile::L_CBUTTON) == 0
    {
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

pub fn get(ui: &mut ui::Ui, channel: usize) -> InputData {
    handle_joystick_events(ui);

    let profile_name = &ui.config.input.input_profile_binding[channel];
    let Some(profile) = ui.config.input.input_profiles.get(profile_name) else {
        eprintln!("Invalid profile name: {profile_name}");
        return InputData {
            data: 0,
            pak_change_pressed: false,
        };
    };
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

    let mut keys = set_buttons(
        profile,
        joystick,
        controller,
        ui.input.keyboard_state,
        alt_pressed,
    );

    let (mut x, mut y) = set_axis(profile, joystick, controller, ui.input.keyboard_state);
    bound_axis(&mut x, &mut y);

    keys |= (x.round() as i8 as u8 as u32) << X_AXIS_SHIFT;
    keys |= (y.round() as i8 as u8 as u32) << Y_AXIS_SHIFT;

    let last_key_state = ui.input.controllers[channel].last_key_state;
    ui.input.controllers[channel].last_key_state = keys;

    if hotkey_pressed(profile, joystick, controller) {
        handle_hotkeys(keys, last_key_state);
        InputData {
            data: 0,
            pak_change_pressed: keys & (1 << ui::input_profile::B_BUTTON) != 0,
        }
    } else {
        let mut pak_change_pressed = false;
        for input in profile.inputs[ui::input_profile::HOTKEY].iter() {
            if let Some(ui::config::InputItem::Key(key)) = input
                && unsafe { *ui.input.keyboard_state.offset(key.id as isize) }
            {
                pak_change_pressed = true;
            }
        }
        InputData {
            data: keys,
            pak_change_pressed,
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

pub fn get_joysticks() -> Vec<sdl3_sys::joystick::SDL_JoystickID> {
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
