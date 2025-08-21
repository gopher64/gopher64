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

pub const DEADZONE_DEFAULT: i32 = 5;

pub struct Controllers {
    pub rumble: bool,
    pub game_controller: *mut sdl3_sys::gamepad::SDL_Gamepad,
    pub joystick: *mut sdl3_sys::joystick::SDL_Joystick,
}

pub struct InputData {
    pub data: u32,
    pub pak_change_pressed: bool,
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
    if profile.joystick_axis[AXIS_LEFT].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_LEFT].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_LEFT].axis as isize > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.joystick_axis[AXIS_RIGHT].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_RIGHT].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_RIGHT].axis as isize > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.joystick_axis[AXIS_DOWN].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_DOWN].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_DOWN].axis as isize > 0 {
            y = (axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64).neg();
        }
    }
    if profile.joystick_axis[AXIS_UP].enabled {
        let axis_position = unsafe {
            sdl3_sys::joystick::SDL_GetJoystickAxis(joystick, profile.joystick_axis[AXIS_UP].id)
        };
        if axis_position as isize * profile.joystick_axis[AXIS_UP].axis as isize > 0 {
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
    if profile.controller_axis[AXIS_LEFT].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_LEFT].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_LEFT].axis as isize > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.controller_axis[AXIS_RIGHT].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_RIGHT].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_RIGHT].axis as isize > 0 {
            x = axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;
        }
    }
    if profile.controller_axis[AXIS_DOWN].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_DOWN].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_DOWN].axis as isize > 0 {
            y = (axis_position as f64 * MAX_AXIS_VALUE / i16::MAX as f64).neg();
        }
    }
    if profile.controller_axis[AXIS_UP].enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile.controller_axis[AXIS_UP].id),
            )
        };
        if axis_position as isize * profile.controller_axis[AXIS_UP].axis as isize > 0 {
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
        if axis_position as isize * profile_joystick_axis.axis as isize > 0
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
    if profile_controller_button.enabled {
        *keys |= (unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadButton(
                controller,
                get_button_from_i32(profile_controller_button.id),
            )
        } as u32)
            << i;
    }

    let profile_controller_axis = profile.controller_axis[i];
    if profile_controller_axis.enabled {
        let axis_position = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadAxis(
                controller,
                get_axis_from_i32(profile_controller_axis.id),
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
    if controller_button.enabled && !controller.is_null() {
        pressed = unsafe {
            sdl3_sys::gamepad::SDL_GetGamepadButton(
                controller,
                get_button_from_i32(controller_button.id),
            )
        };
    } else if joystick_button.enabled && !joystick.is_null() {
        pressed =
            unsafe { sdl3_sys::joystick::SDL_GetJoystickButton(joystick, joystick_button.id) };
    } else if joystick_hat.enabled && !joystick.is_null() {
        pressed = (unsafe { sdl3_sys::joystick::SDL_GetJoystickHat(joystick, joystick_hat.id) }
            & joystick_hat.direction)
            != 0;
    } else if key.enabled {
        pressed = unsafe { *keyboard_state.offset(key.id as isize) };
    }
    pressed
}

pub fn get_controller_names(game_ui: &ui::Ui) -> Vec<String> {
    let mut controllers: Vec<String> = vec![];

    for joystick in game_ui.input.joysticks.iter() {
        let name = unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::joystick::SDL_GetJoystickNameForID(*joystick))
        };
        controllers.push(name.to_string_lossy().to_string());
    }

    controllers
}

pub fn get_controller_paths(game_ui: &ui::Ui) -> Vec<Option<String>> {
    let mut controller_paths: Vec<Option<String>> = vec![];

    for joystick in game_ui.input.joysticks.iter() {
        let path = unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::joystick::SDL_GetJoystickPathForID(*joystick))
                .to_string_lossy()
                .to_string()
        };
        controller_paths.push(Some(path));
    }

    controller_paths
}

pub fn get(ui: &ui::Ui, channel: usize) -> InputData {
    unsafe { sdl3_sys::events::SDL_PumpEvents() };

    let profile_name = ui.config.input.input_profile_binding[channel].clone();
    let profile = ui.config.input.input_profiles.get(&profile_name).unwrap();
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
        (x, y) = set_axis_from_controller(profile, controller)
    } else if !joystick.is_null() {
        (x, y) = set_axis_from_joystick(profile, joystick)
    }
    bound_axis(&mut x, &mut y);

    keys |= (x.round() as i8 as u8 as u32) << X_AXIS_SHIFT;
    keys |= (y.round() as i8 as u8 as u32) << Y_AXIS_SHIFT;

    InputData {
        data: keys,
        pak_change_pressed: change_paks(profile, joystick, controller, ui.input.keyboard_state),
    }
}

pub fn assign_controller(ui: &mut ui::Ui, controller: i32, port: usize) {
    if controller < ui.input.joysticks.len() as i32 {
        let path = unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::joystick::SDL_GetJoystickPathForID(
                ui.input.joysticks[controller as usize],
            ))
            .to_string_lossy()
            .to_string()
        };
        ui.config.input.controller_assignment[port - 1] = Some(path);
    } else {
        println!("Invalid controller number")
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

pub fn configure_input_profile(ui: &mut ui::Ui, profile: String, dinput: bool, deadzone: i32) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_VIDEO);
    ui::ttf_init();

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

    for joystick in ui.input.joysticks.iter() {
        if !dinput {
            let controller = unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(*joystick) };
            if !controller.is_null() {
                open_controllers.push(controller);
            }
        } else {
            let joystick = unsafe { sdl3_sys::joystick::SDL_OpenJoystick(*joystick) };
            if !joystick.is_null() {
                open_joysticks.push(joystick);
            }
        }
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

    let mut new_keys = [ui::config::InputKeyButton {
        enabled: false,
        id: 0,
    }; PROFILE_SIZE];
    let mut new_joystick_buttons = [ui::config::InputKeyButton {
        enabled: false,
        id: 0,
    }; PROFILE_SIZE];
    let mut new_joystick_hat = [ui::config::InputJoystickHat {
        enabled: false,
        id: 0,
        direction: 0,
    }; PROFILE_SIZE];
    let mut new_joystick_axis = [ui::config::InputControllerAxis {
        enabled: false,
        id: 0,
        axis: 0,
    }; PROFILE_SIZE];
    let mut new_controller_buttons = [ui::config::InputKeyButton {
        enabled: false,
        id: 0,
    }; PROFILE_SIZE];
    let mut new_controller_axis = [ui::config::InputControllerAxis {
        enabled: false,
        id: 0,
        axis: 0,
    }; PROFILE_SIZE];

    let mut last_joystick_axis_result = ui::config::InputControllerAxis {
        enabled: false,
        id: 0,
        axis: 0,
    };

    let text_engine = unsafe { sdl3_ttf_sys::ttf::TTF_CreateRendererTextEngine(renderer) };
    let font = unsafe {
        let font_bytes = include_bytes!("../../data/Roboto-Regular.ttf");
        sdl3_ttf_sys::ttf::TTF_OpenFontIO(
            sdl3_sys::everything::SDL_IOFromConstMem(
                font_bytes.as_ptr() as *const std::ffi::c_void,
                font_bytes.len(),
            ),
            true,
            35.0,
        )
    };

    for (key, value) in key_labels.iter() {
        let mut event: sdl3_sys::events::SDL_Event = Default::default();
        while unsafe { sdl3_sys::events::SDL_PollEvent(&mut event) } {} // clear events

        ui::video::draw_text(
            format!("Select binding for: {key}").as_str(),
            renderer,
            text_engine,
            font,
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
                    if unsafe {
                        !event.key.repeat
                            && event.key.scancode != sdl3_sys::scancode::SDL_SCANCODE_LALT
                            && event.key.scancode != sdl3_sys::scancode::SDL_SCANCODE_RALT
                    } {
                        new_keys[*value] = ui::config::InputKeyButton {
                            enabled: true,
                            id: i32::from(unsafe { event.key.scancode }),
                        };
                        key_set = true
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_GAMEPAD_BUTTON_DOWN) {
                    if !open_controllers.is_empty() {
                        new_controller_buttons[*value] = ui::config::InputKeyButton {
                            enabled: true,
                            id: i32::from(unsafe { event.gbutton.button }),
                        };
                        key_set = true
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_GAMEPAD_AXIS_MOTION) {
                    let axis_value = unsafe { event.gaxis.value };
                    let axis = unsafe { event.gaxis.axis };
                    if !open_controllers.is_empty()
                        && axis_value.saturating_abs() > (i16::MAX as i32 * 3 / 4) as i16
                    {
                        let result = ui::config::InputControllerAxis {
                            enabled: true,
                            id: axis as i32,
                            axis: axis_value / axis_value.saturating_abs(),
                        };
                        if result != last_joystick_axis_result {
                            new_controller_axis[*value] = result;
                            last_joystick_axis_result = result;
                            key_set = true
                        }
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_JOYSTICK_BUTTON_DOWN)
                {
                    if !open_joysticks.is_empty() {
                        new_joystick_buttons[*value] = ui::config::InputKeyButton {
                            enabled: true,
                            id: i32::from(unsafe { event.jbutton.button }),
                        };
                        key_set = true
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_JOYSTICK_HAT_MOTION) {
                    let state = unsafe { event.jhat.value };
                    let hat = unsafe { event.jhat.hat };
                    if !open_joysticks.is_empty() && state != sdl3_sys::joystick::SDL_HAT_CENTERED {
                        new_joystick_hat[*value] = ui::config::InputJoystickHat {
                            enabled: true,
                            id: hat as i32,
                            direction: state,
                        };
                        key_set = true
                    }
                } else if event_type == u32::from(sdl3_sys::events::SDL_EVENT_JOYSTICK_AXIS_MOTION)
                {
                    let axis_value = unsafe { event.jaxis.value };
                    let axis = unsafe { event.jaxis.axis };
                    if !open_joysticks.is_empty()
                        && axis_value.saturating_abs() > (i16::MAX as i32 * 3 / 4) as i16
                    {
                        let result = ui::config::InputControllerAxis {
                            enabled: true,
                            id: axis as i32,
                            axis: axis_value / axis_value.saturating_abs(),
                        };
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
        sdl3_ttf_sys::ttf::TTF_CloseFont(font);
        sdl3_ttf_sys::ttf::TTF_DestroyRendererTextEngine(text_engine);
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
        deadzone,
    };
    ui.config.input.input_profiles.insert(profile, new_profile);
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
    default_keys[CHANGE_PAK] = ui::config::InputKeyButton {
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
    };
    default_controller_axis[L_CBUTTON] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTX),
        axis: -1,
    };
    default_controller_axis[D_CBUTTON] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
        axis: 1,
    };
    default_controller_axis[U_CBUTTON] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
        axis: -1,
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
    };
    default_controller_axis[AXIS_RIGHT] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX),
        axis: 1,
    };
    default_controller_axis[AXIS_UP] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
        axis: -1,
    };
    default_controller_axis[AXIS_DOWN] = ui::config::InputControllerAxis {
        enabled: true,
        id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
        axis: 1,
    };
    default_controller_buttons[CHANGE_PAK] = ui::config::InputKeyButton {
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
        }; PROFILE_SIZE],
        dinput: false,
        deadzone: DEADZONE_DEFAULT,
    }
}

pub fn init(ui: &mut ui::Ui) {
    ui.input.keyboard_state =
        unsafe { sdl3_sys::keyboard::SDL_GetKeyboardState(std::ptr::null_mut()) };
    if ui.input.keyboard_state.is_null() {
        panic!("Could not get keyboard state");
    }

    for i in 0..4 {
        let controller_assignment = &ui.config.input.controller_assignment[i];
        if controller_assignment.is_some() && ui.config.input.controller_enabled[i] {
            let mut joystick_id = 0;
            let assigned_path = controller_assignment.as_ref().unwrap();

            for joystick in ui.input.joysticks.iter() {
                let path = unsafe {
                    std::ffi::CStr::from_ptr(sdl3_sys::joystick::SDL_GetJoystickPathForID(
                        *joystick,
                    ))
                    .to_string_lossy()
                    .to_string()
                };
                if path == *assigned_path
                    && unsafe { sdl3_sys::joystick::SDL_GetJoystickFromID(*joystick) }.is_null()
                    && unsafe { sdl3_sys::gamepad::SDL_GetGamepadFromID(*joystick) }.is_null()
                {
                    joystick_id = *joystick;
                    break;
                }
            }

            if joystick_id != 0 {
                let profile_name = ui.config.input.input_profile_binding[i].clone();
                let profile = ui.config.input.input_profiles.get(&profile_name).unwrap();

                if !profile.dinput {
                    let gamepad = unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(joystick_id) };
                    if gamepad.is_null() {
                        println!("could not connect gamepad: {joystick_id}")
                    } else {
                        ui.input.controllers[i].game_controller = gamepad;
                        let properties =
                            unsafe { sdl3_sys::gamepad::SDL_GetGamepadProperties(gamepad) };
                        if properties == 0 {
                            println!("could not get gamepad properties");
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
                        println!("could not connect joystick: {joystick_id}")
                    } else {
                        ui.input.controllers[i].joystick = joystick;
                        let properties =
                            unsafe { sdl3_sys::joystick::SDL_GetJoystickProperties(joystick) };
                        if properties == 0 {
                            println!("could not get joystick properties");
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
                println!("Could not bind assigned controller");
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
