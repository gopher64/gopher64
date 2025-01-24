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
const CHANGE_PAK: usize = 18;
pub const PROFILE_SIZE: usize = 19;

const X_AXIS_SHIFT: usize = 16;
const Y_AXIS_SHIFT: usize = 24;

const MAX_AXIS_VALUE: f64 = 85.0;

pub struct Controllers {
    pub rumble: bool,
    pub game_controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
    pub joystick: *mut sdl3_sys::joystick::SDL_Joystick,
}

fn bound_axis(x: &mut f64, y: &mut f64) {
    let radius = 95.0; // this is roughly the maxium diagonal distance of the controller

    // Calculate the distance from the origin (0, 0)
    let distance = f64::sqrt((*x) * (*x) + (*y) * (*y));

    // If the distance is greater than the radius, scale the coordinates
    if distance > radius {
        let scale_factor = radius / distance;
        *x *= scale_factor;
        *y *= scale_factor;
    }
}

fn set_axis_from_joystick(
    profile: &ui::config::InputProfile,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    if profile.joystick_axis[AXIS_LEFT].0 {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_LEFT].1)
        };
        if axis_position * profile.joystick_axis[AXIS_LEFT].2 > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.joystick_axis[AXIS_RIGHT].0 {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_RIGHT].1)
        };
        if axis_position * profile.joystick_axis[AXIS_RIGHT].2 > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.joystick_axis[AXIS_DOWN].0 {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_DOWN].1)
        };
        if axis_position * profile.joystick_axis[AXIS_DOWN].2 > 0 {
            y = (axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64).neg();
        }
    }
    if profile.joystick_axis[AXIS_UP].0 {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_UP].1)
        };
        if axis_position * profile.joystick_axis[AXIS_UP].2 > 0 {
            y = (axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64).neg();
        }
    }
    (x, y)
}

fn get_axis_from_i32(axis: i32) -> sdl3_sys::gamepad::SDL_GamepadAxis {
    match axis {
        0 => sdl3_sys::gamepad::SDL_GamepadAxis::LEFTX,
        1 => sdl3_sys::gamepad::SDL_GamepadAxis::LEFTY,
        2 => sdl3_sys::gamepad::SDL_GamepadAxis::RIGHTX,
        3 => sdl3_sys::gamepad::SDL_GamepadAxis::RIGHTY,
        4 => sdl3_sys::gamepad::SDL_GamepadAxis::LEFT_TRIGGER,
        5 => sdl3_sys::gamepad::SDL_GamepadAxis::RIGHT_TRIGGER,
        _ => panic!("Invalid axis"),
    }
}

fn get_button_from_i32(button: i32) -> sdl3_sys::gamepad::SDL_GamepadButton {
    match button {
        0 => sdl3_sys::gamepad::SDL_GamepadButton::SOUTH,
        1 => sdl3_sys::gamepad::SDL_GamepadButton::EAST,
        2 => sdl3_sys::gamepad::SDL_GamepadButton::WEST,
        3 => sdl3_sys::gamepad::SDL_GamepadButton::NORTH,
        4 => sdl3_sys::gamepad::SDL_GamepadButton::BACK,
        5 => sdl3_sys::gamepad::SDL_GamepadButton::GUIDE,
        6 => sdl3_sys::gamepad::SDL_GamepadButton::START,
        7 => sdl3_sys::gamepad::SDL_GamepadButton::LEFT_STICK,
        8 => sdl3_sys::gamepad::SDL_GamepadButton::RIGHT_STICK,
        9 => sdl3_sys::gamepad::SDL_GamepadButton::LEFT_SHOULDER,
        10 => sdl3_sys::gamepad::SDL_GamepadButton::RIGHT_SHOULDER,
        11 => sdl3_sys::gamepad::SDL_GamepadButton::DPAD_UP,
        12 => sdl3_sys::gamepad::SDL_GamepadButton::DPAD_DOWN,
        13 => sdl3_sys::gamepad::SDL_GamepadButton::DPAD_LEFT,
        14 => sdl3_sys::gamepad::SDL_GamepadButton::DPAD_RIGHT,
        _ => panic!("Invalid button"),
    }
}

fn set_axis_from_controller(
    profile: &ui::config::InputProfile,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    if profile.controller_axis[AXIS_LEFT].0 {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_LEFT].1),
            )
        };
        if axis_position * profile.controller_axis[AXIS_LEFT].2 > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.controller_axis[AXIS_RIGHT].0 {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_RIGHT].1),
            )
        };
        if axis_position * profile.controller_axis[AXIS_RIGHT].2 > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.controller_axis[AXIS_DOWN].0 {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_DOWN].1),
            )
        };
        if axis_position * profile.controller_axis[AXIS_DOWN].2 > 0 {
            y = (axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64).neg();
        }
    }
    if profile.controller_axis[AXIS_UP].0 {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_UP].1),
            )
        };
        if axis_position * profile.controller_axis[AXIS_UP].2 > 0 {
            y = (axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64).neg();
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
    if profile.keys[AXIS_LEFT].0
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_LEFT].1 as isize) }
    {
        x = -MAX_AXIS_VALUE
    }
    if profile.keys[AXIS_RIGHT].0
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_RIGHT].1 as isize) }
    {
        x = MAX_AXIS_VALUE
    }
    if profile.keys[AXIS_DOWN].0
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_DOWN].1 as isize) }
    {
        y = -MAX_AXIS_VALUE
    }
    if profile.keys[AXIS_UP].0
        && unsafe { *keyboard_state.offset(profile.keys[AXIS_UP].1 as isize) }
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
    if profile_joystick_button.0 {
        *keys |= (unsafe {
            sdl3_sys::joystick::SDL_GetJoystickButton(joystick, profile_joystick_button.1)
        } as u32)
            << i;
    }

    let profile_joystick_hat = profile.joystick_hat[i];
    if profile_joystick_hat.0
        && unsafe { sdl3_sys::joystick::SDL_GetJoystickHat(joystick, profile_joystick_hat.1) }
            == profile_joystick_hat.2
    {
        *keys |= 1 << i;
    }

    let profile_joystick_axis = profile.joystick_axis[i];
    if profile_joystick_axis.0 {
        let axis_position =
            unsafe { sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile_joystick_axis.1) };
        if axis_position * profile_joystick_axis.2 > 0
            && axis_position.saturating_abs() > i16::MAX / 2
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
    if profile_controller_button.0 {
        *keys |= (unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadButton(
                controller,
                get_button_from_i32(profile_controller_button.1),
            )
        } as u32)
            << i;
    }

    let profile_controller_axis = profile.controller_axis[i];
    if profile_controller_axis.0 {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile_controller_axis.1),
            )
        };
        if axis_position * profile_controller_axis.2 > 0
            && axis_position.saturating_abs() > i16::MAX / 2
        {
            *keys |= 1 << i;
        }
    }
}

pub fn set_rumble(ui: &mut ui::Ui, channel: usize, rumble: u8) {
    if !ui.controllers[channel].rumble {
        return;
    }
    let controller = ui.controllers[channel].game_controller;
    let joystick = ui.controllers[channel].joystick;
    if !controller.is_null() {
        unsafe {
            sdl3_sys::gamepad::SDL_RumbleGamepad(
                controller,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u32 * 60000,
            )
        };
    } else if !joystick.is_null() {
        unsafe {
            sdl3_sys::joystick::SDL_RumbleJoystick(
                joystick,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u16 * u16::MAX,
                (rumble & 1) as u32 * 60000,
            )
        };
    }
}

fn change_paks(
    profile: &ui::config::InputProfile,
    joystick: *mut sdl3_sys::joystick::SDL_Joystick,
    controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
    keyboard_state: *const bool,
) -> bool {
    let controller_button = profile.controller_buttons[CHANGE_PAK];
    let joystick_button = profile.joystick_buttons[CHANGE_PAK];
    let joystick_hat = profile.joystick_hat[CHANGE_PAK];
    let key = profile.keys[CHANGE_PAK];

    let mut pressed = false;
    if controller_button.0 && !controller.is_null() {
        pressed = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadButton(
                controller,
                get_button_from_i32(controller_button.1),
            )
        };
    } else if joystick_button.0 && !joystick.is_null() {
        pressed = unsafe { sdl3_sys::joystick::SDL_GetJoystickButton(joystick, joystick_button.1) };
    } else if joystick_hat.0 && !joystick.is_null() {
        pressed = unsafe { sdl3_sys::joystick::SDL_GetJoystickHat(joystick, joystick_hat.1) }
            == joystick_hat.2;
    } else if key.0 {
        pressed = unsafe { *keyboard_state.offset(key.1 as isize) };
    }
    pressed
}

pub fn get(ui: &mut ui::Ui, channel: usize) -> (u32, bool) {
    let profile_name = ui.config.input.input_profile_binding[channel].clone();
    let profile = ui.config.input.input_profiles.get(&profile_name).unwrap();
    let mut keys = 0;
    let controller = ui.controllers[channel].game_controller;
    let joystick = ui.controllers[channel].joystick;
    for i in 0..14 {
        if profile_name != "default" || channel == 0 {
            let profile_key = profile.keys[i];
            if profile_key.0 {
                keys |= (unsafe { *ui.keyboard_state.offset(profile_key.1 as isize) } as u32) << i;
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
        (x, y) = set_axis_from_keys(profile, ui.keyboard_state);
    }

    if !controller.is_null() {
        (x, y) = set_axis_from_controller(profile, controller)
    } else if !joystick.is_null() {
        (x, y) = set_axis_from_joystick(profile, joystick)
    }
    bound_axis(&mut x, &mut y);

    keys |= (x.round() as i8 as u8 as u32) << X_AXIS_SHIFT;
    keys |= (y.round() as i8 as u8 as u32) << Y_AXIS_SHIFT;

    (
        keys,
        change_paks(profile, joystick, controller, ui.keyboard_state),
    )
}

pub fn assign_controller(ui: &mut ui::Ui, controller: i32, port: usize) {
    let mut joystick_count = 0;
    let joysticks = unsafe { sdl3_sys::joystick::SDL_GetJoysticks(&mut joystick_count) };
    if !joysticks.is_null() {
        if controller < joystick_count {
            let guid = unsafe {
                sdl3_sys::joystick::SDL_GetJoystickGUIDForID(
                    *(joysticks.offset(controller as isize)),
                )
                .data
            };
            ui.config.input.controller_assignment[port - 1] = Some(guid);
        } else {
            println!("Invalid controller number")
        }
        unsafe { sdl3_sys::stdinc::SDL_free(joysticks as *mut std::ffi::c_void) };
    }
}

pub fn bind_input_profile(ui: &mut ui::Ui, profile: String, port: usize) {
    if ui.config.input.input_profiles.contains_key(&profile) {
        ui.config.input.input_profile_binding[port - 1] = profile;
    } else {
        println!("Invalid profile name")
    }
}

pub fn clear_bindings(ui: &mut ui::Ui) {
    for i in 0..4 {
        ui.config.input.controller_assignment[i] = None;
        ui.config.input.input_profile_binding[i] = "default".to_string();
    }
}

fn close_controllers(
    open_joysticks: Vec<*mut sdl3_sys::joystick::SDL_Joystick>,
    open_controllers: Vec<*mut sdl3_sys::gamepad::SDL_Gamepad>,
) {
    for joystick in open_joysticks {
        unsafe { sdl3_sys::joystick::SDL_CloseJoystick(joystick) }
    }
    for controller in open_controllers {
        unsafe { sdl3_sys::gamepad::SDL_CloseGamepad(controller) }
    }
}

pub fn configure_input_profile(ui: &mut ui::Ui, profile: String, dinput: bool) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_VIDEO);
    ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);

    if profile == "default" {
        println!("Profile name cannot be default");
        return;
    }
    if profile.is_empty() {
        println!("Profile name cannot be empty");
        return;
    }
    let mut open_joysticks: Vec<*mut sdl3_sys::joystick::SDL_Joystick> = Vec::new();
    let mut open_controllers: Vec<*mut sdl3_sys::gamepad::SDL_Gamepad> = Vec::new();

    let mut joystick_count = 0;
    let joysticks = unsafe { sdl3_sys::joystick::SDL_GetJoysticks(&mut joystick_count) };
    if !joysticks.is_null() {
        for offset in 0..joystick_count as isize {
            if !dinput {
                let controller =
                    unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(*joysticks.offset(offset)) };
                if !controller.is_null() {
                    open_controllers.push(controller);
                }
            } else {
                let joystick =
                    unsafe { sdl3_sys::joystick::SDL_OpenJoystick(*joysticks.offset(offset)) };
                if !joystick.is_null() {
                    open_joysticks.push(joystick);
                }
            }
        }
        unsafe { sdl3_sys::stdinc::SDL_free(joysticks as *mut std::ffi::c_void) };
    }

    let title = std::ffi::CString::new("configure input profile").unwrap();
    let mut window: *mut sdl3_sys::video::SDL_Window = std::ptr::null_mut();
    let mut renderer: *mut sdl3_sys::render::SDL_Renderer = std::ptr::null_mut();
    if !unsafe {
        sdl3_sys::render::SDL_CreateWindowAndRenderer(
            title.as_ptr(),
            640,
            480,
            0,
            &mut window,
            &mut renderer,
        )
    } {
        panic!("Could not create window and renderer")
    }
    if !unsafe { sdl3_sys::video::SDL_ShowWindow(window) } {
        panic!("Could not show window")
    }
    let font =
        rusttype::Font::try_from_bytes(include_bytes!("../../data/Roboto-Regular.ttf")).unwrap();

    let key_labels: [(&str, usize); PROFILE_SIZE] = [
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
        ("Change Pak", CHANGE_PAK),
    ];

    let mut new_keys = [(false, 0); PROFILE_SIZE];
    let mut new_joystick_buttons = [(false, 0i32); PROFILE_SIZE];
    let mut new_joystick_hat = [(false, 0i32, 0); PROFILE_SIZE];
    let mut new_joystick_axis = [(false, 0i32, 0); PROFILE_SIZE];
    let mut new_controller_buttons = [(false, 0i32); PROFILE_SIZE];
    let mut new_controller_axis = [(false, 0i32, 0); PROFILE_SIZE];

    let mut last_joystick_axis_result = (false, 0, 0);
    let mut last_controller_axis_result = (false, 0, 0);
    for (key, value) in key_labels.iter() {
        let mut event: sdl3_sys::events::SDL_Event = Default::default();
        while unsafe { sdl3_sys::events::SDL_PollEvent(&mut event) } {} // clear events

        ui::video::draw_text(
            format!("Select binding for: {key}").as_str(),
            renderer,
            &font,
        );

        let mut key_set = false;
        while !key_set {
            std::thread::sleep(std::time::Duration::from_millis(100));
            while unsafe { sdl3_sys::events::SDL_PollEvent(&mut event) } {
                let event_type = unsafe { event.r#type };
                if event_type == u32::from(sdl3_sys::events::SDL_EVENT_WINDOW_CLOSE_REQUESTED) {
                    close_controllers(open_joysticks, open_controllers);
                    return;
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_KEY_DOWN) {
                    new_keys[*value] = (true, i32::from(unsafe { event.key.scancode }));
                    key_set = true
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_GAMEPAD_BUTTON_DOWN) {
                    if !open_controllers.is_empty() {
                        new_controller_buttons[*value] =
                            (true, i32::from(unsafe { event.gbutton.button }));
                        key_set = true
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_GAMEPAD_AXIS_MOTION) {
                    let axis_value = unsafe { event.gaxis.value };
                    let axis = unsafe { event.gaxis.axis };
                    if !open_controllers.is_empty() && axis_value.saturating_abs() > i16::MAX / 2 {
                        let result = (true, axis as i32, axis_value / axis_value.saturating_abs());
                        if result != last_controller_axis_result {
                            new_controller_axis[*value] = result;
                            last_controller_axis_result = result;
                            key_set = true
                        }
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_JOYSTICK_BUTTON_DOWN)
                {
                    if !open_joysticks.is_empty() {
                        new_joystick_buttons[*value] =
                            (true, i32::from(unsafe { event.jbutton.button }));
                        key_set = true
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_JOYSTICK_HAT_MOTION) {
                    let state = unsafe { event.jhat.value };
                    let hat = unsafe { event.jhat.hat };
                    if !open_joysticks.is_empty() && state != sdl3_sys::joystick::SDL_HAT_CENTERED {
                        new_joystick_hat[*value] = (true, hat as i32, state);
                        key_set = true
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_JOYSTICK_AXIS_MOTION)
                {
                    let axis_value = unsafe { event.jaxis.value };
                    let axis = unsafe { event.jaxis.axis };
                    if !open_joysticks.is_empty() && axis_value.saturating_abs() > i16::MAX / 2 {
                        let result = (true, axis as i32, axis_value / axis_value.saturating_abs());
                        if result != last_joystick_axis_result {
                            new_joystick_axis[*value] = result;
                            last_joystick_axis_result = result;
                            key_set = true
                        }
                    }
                }
            }
        }
    }

    close_controllers(open_joysticks, open_controllers);

    unsafe {
        sdl3_sys::render::SDL_DestroyRenderer(renderer);
        sdl3_sys::video::SDL_DestroyWindow(window);
    }

    let new_profile = ui::config::InputProfile {
        keys: new_keys,
        controller_buttons: new_controller_buttons,
        controller_axis: new_controller_axis,
        joystick_buttons: new_joystick_buttons,
        joystick_hat: new_joystick_hat,
        joystick_axis: new_joystick_axis,
        dinput,
    };
    ui.config.input.input_profiles.insert(profile, new_profile);
}

pub fn get_default_profile() -> ui::config::InputProfile {
    let mut default_controller_buttons = [(false, 0); PROFILE_SIZE];
    let mut default_controller_axis = [(false, 0, 0); PROFILE_SIZE];
    let mut default_keys = [(false, 0); PROFILE_SIZE];
    default_keys[R_DPAD] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_D));
    default_keys[L_DPAD] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_A));
    default_keys[D_DPAD] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_S));
    default_keys[U_DPAD] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_W));
    default_keys[START_BUTTON] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_RETURN));
    default_keys[Z_TRIG] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_Z));
    default_keys[B_BUTTON] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_LCTRL));
    default_keys[A_BUTTON] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_LSHIFT));
    default_keys[R_CBUTTON] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_L));
    default_keys[L_CBUTTON] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_J));
    default_keys[D_CBUTTON] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_K));
    default_keys[U_CBUTTON] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_I));
    default_keys[R_TRIG] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_C));
    default_keys[L_TRIG] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_X));
    default_keys[AXIS_LEFT] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_LEFT));
    default_keys[AXIS_RIGHT] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_RIGHT));
    default_keys[AXIS_UP] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_UP));
    default_keys[AXIS_DOWN] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_DOWN));
    default_keys[CHANGE_PAK] = (true, i32::from(sdl3_sys::scancode::SDL_SCANCODE_COMMA));

    default_controller_buttons[R_DPAD] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_RIGHT),
    );
    default_controller_buttons[L_DPAD] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_LEFT),
    );
    default_controller_buttons[D_DPAD] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_DOWN),
    );
    default_controller_buttons[U_DPAD] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_UP),
    );
    default_controller_buttons[START_BUTTON] =
        (true, i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_START));
    default_controller_axis[Z_TRIG] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFT_TRIGGER),
        1,
    );
    default_controller_buttons[B_BUTTON] =
        (true, i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_WEST));
    default_controller_buttons[A_BUTTON] =
        (true, i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_SOUTH));
    default_controller_axis[R_CBUTTON] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTX),
        1,
    );
    default_controller_axis[L_CBUTTON] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTX),
        -1,
    );
    default_controller_axis[D_CBUTTON] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
        1,
    );
    default_controller_axis[U_CBUTTON] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
        -1,
    );
    default_controller_buttons[R_TRIG] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER),
    );
    default_controller_buttons[L_TRIG] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_LEFT_SHOULDER),
    );
    default_controller_axis[AXIS_LEFT] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX),
        -1,
    );
    default_controller_axis[AXIS_RIGHT] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX),
        1,
    );
    default_controller_axis[AXIS_UP] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
        -1,
    );
    default_controller_axis[AXIS_DOWN] = (
        true,
        i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
        1,
    );
    default_controller_buttons[CHANGE_PAK] =
        (true, i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_BACK));

    ui::config::InputProfile {
        keys: default_keys,
        controller_buttons: default_controller_buttons,
        controller_axis: default_controller_axis,
        joystick_buttons: Default::default(),
        joystick_hat: Default::default(),
        joystick_axis: Default::default(),
        dinput: false,
    }
}

pub fn init(ui: &mut ui::Ui) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);

    ui.keyboard_state = unsafe { sdl3_sys::keyboard::SDL_GetKeyboardState(std::ptr::null_mut()) };

    let mut taken = [false; 4];
    for i in 0..4 {
        let controller_assignment = &ui.config.input.controller_assignment[i];
        if controller_assignment.is_some() {
            let mut joystick_id = 0;
            let assigned_guid = controller_assignment.unwrap();

            let mut joystick_count = 0;
            let joysticks = unsafe { sdl3_sys::joystick::SDL_GetJoysticks(&mut joystick_count) };
            if !joysticks.is_null() {
                for offset in 0..joystick_count as isize {
                    let guid = unsafe {
                        sdl3_sys::joystick::SDL_GetJoystickGUIDForID(*(joysticks.offset(offset)))
                            .data
                    };
                    if guid == assigned_guid && !taken[offset as usize] {
                        joystick_id = unsafe { *(joysticks.offset(offset)) };
                        taken[offset as usize] = true;
                        break;
                    }
                }
                unsafe { sdl3_sys::stdinc::SDL_free(joysticks as *mut std::ffi::c_void) };
            }
            if joystick_id != 0 {
                let profile_name = ui.config.input.input_profile_binding[i].clone();
                let profile = ui.config.input.input_profiles.get(&profile_name).unwrap();

                if !profile.dinput {
                    let gamepad = unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(joystick_id) };
                    if gamepad.is_null() {
                        println!("could not connect gamepad: {}", joystick_id)
                    } else {
                        ui.controllers[i].game_controller = gamepad;
                        let properties =
                            unsafe { sdl3_sys::gamepad::SDL_GetGamepadProperties(gamepad) };
                        let rumble_prop =
                            std::ffi::CString::new("SDL_PROP_GAMEPAD_CAP_RUMBLE_BOOLEAN").unwrap();
                        ui.controllers[i].rumble = unsafe {
                            sdl3_sys::properties::SDL_GetBooleanProperty(
                                properties,
                                rumble_prop.as_ptr(),
                                false,
                            )
                        };
                    }
                } else {
                    let joystick = unsafe { sdl3_sys::joystick::SDL_OpenJoystick(joystick_id) };
                    if joystick.is_null() {
                        println!("could not connect joystick: {}", joystick_id)
                    } else {
                        ui.controllers[i].joystick = joystick;
                        let properties =
                            unsafe { sdl3_sys::joystick::SDL_GetJoystickProperties(joystick) };
                        let rumble_prop =
                            std::ffi::CString::new("SDL_PROP_JOYSTICK_CAP_RUMBLE_BOOLEAN").unwrap();
                        ui.controllers[i].rumble = unsafe {
                            sdl3_sys::properties::SDL_GetBooleanProperty(
                                properties,
                                rumble_prop.as_ptr(),
                                false,
                            )
                        };
                    }
                }
            } else {
                println!("Could not bind assigned controller");
            }
        }
    }
}

pub fn close(ui: &mut ui::Ui) {
    for controller in ui.controllers.iter_mut() {
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
