use crate::ui;

pub const R_DPAD: usize = 0;
pub const L_DPAD: usize = 1;
pub const D_DPAD: usize = 2;
pub const U_DPAD: usize = 3;
pub const START_BUTTON: usize = 4;
pub const Z_TRIG: usize = 5;
pub const B_BUTTON: usize = 6;
pub const A_BUTTON: usize = 7;
pub const R_CBUTTON: usize = 8;
pub const L_CBUTTON: usize = 9;
pub const D_CBUTTON: usize = 10;
pub const U_CBUTTON: usize = 11;
pub const R_TRIG: usize = 12;
pub const L_TRIG: usize = 13;
pub const X_AXIS: usize = 16;
pub const Y_AXIS: usize = 24;

pub const AXIS_LEFT: usize = 14;
pub const AXIS_RIGHT: usize = 15;
pub const AXIS_UP: usize = 16;
pub const AXIS_DOWN: usize = 17;

pub const MAX_AXIS_VALUE: f64 = 85.0;

pub struct Controllers {
    pub game_controller: Option<sdl2::controller::GameController>,
    pub joystick: Option<sdl2::joystick::Joystick>,
}

pub fn bound_axis(x: &mut f64, y: &mut f64) {
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

pub fn get(ui: &mut ui::Ui, channel: usize) -> u32 {
    let context = ui.sdl_context.as_mut().unwrap();
    let events = context.event_pump().unwrap();
    let keyboard_state = events.keyboard_state();

    let profile_name = ui.config.input.input_profile_binding[channel].clone();
    let profile = ui.config.input.input_profiles.get(&profile_name).unwrap();
    let mut keys = 0;
    for i in 0..14 {
        let profile_key = profile.keys[i];
        if profile_key.0 {
            unsafe {
                keys |= (keyboard_state.is_scancode_pressed(std::mem::transmute(profile_key.1))
                    as u32)
                    << i;
            }
        }

        let controller = &ui.controllers[channel].game_controller;
        if controller.is_some() {
            let profile_controller_button = profile.controller_buttons[i];
            if profile_controller_button.0 {
                unsafe {
                    keys |= (controller
                        .as_ref()
                        .unwrap()
                        .button(std::mem::transmute(profile_controller_button.1))
                        as u32)
                        << i;
                }
            }

            let profile_controller_axis = profile.controller_axis[i];
            if profile_controller_axis.0 {
                let axis_position = unsafe {
                    controller
                        .as_ref()
                        .unwrap()
                        .axis(std::mem::transmute(profile_controller_axis.1))
                };
                if axis_position * profile_controller_axis.2 > 0 {
                    if axis_position.abs() > std::i16::MAX / 2 {
                        keys |= 1 << i;
                    }
                }
            }
        }

        // TODO: joystick hat, button, axis
    }

    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    unsafe {
        if profile.keys[AXIS_LEFT].0 {
            if keyboard_state.is_scancode_pressed(std::mem::transmute(profile.keys[AXIS_LEFT].1)) {
                x = -MAX_AXIS_VALUE
            }
        }
        if profile.keys[AXIS_RIGHT].0 {
            if keyboard_state.is_scancode_pressed(std::mem::transmute(profile.keys[AXIS_RIGHT].1)) {
                x = MAX_AXIS_VALUE
            }
        }
        if profile.keys[AXIS_DOWN].0 {
            if keyboard_state.is_scancode_pressed(std::mem::transmute(profile.keys[AXIS_DOWN].1)) {
                y = -MAX_AXIS_VALUE
            }
        }
        if profile.keys[AXIS_UP].0 {
            if keyboard_state.is_scancode_pressed(std::mem::transmute(profile.keys[AXIS_UP].1)) {
                y = MAX_AXIS_VALUE
            }
        }
    }
    bound_axis(&mut x, &mut y);

    keys |= (x.round() as i8 as u32) << X_AXIS;
    keys |= (y.round() as i8 as u32) << Y_AXIS;
    return keys;
}

pub fn list_controllers(ui: &mut ui::Ui) {
    let joystick = ui.joystick_subsystem.as_ref().unwrap();
    let num_joysticks = joystick.num_joysticks().unwrap();
    if num_joysticks == 0 {
        println!("No controllers connected")
    }
    for i in 0..num_joysticks {
        println!("{}: {}", i, joystick.name_for_index(i).unwrap())
    }
}

pub fn assign_controller(ui: &mut ui::Ui, controller: u32, port: usize) {
    let joystick = ui.joystick_subsystem.as_ref().unwrap();
    let num_joysticks = joystick.num_joysticks().unwrap();
    if controller < num_joysticks {
        ui.config.input.controller_assignment[port - 1] = Some(controller);
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

pub fn configure_input_profile(ui: &mut ui::Ui, profile: String) {
    if profile == "default" {
        println!("Profile name cannot be default");
        return;
    }
    let key_labels = [
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
    ];

    let new_keys = [(false, 0); 18];
    let new_controller_buttons = [(false, 0); 18];
    let new_controller_axis = [(false, 0, 0); 18];

    for (key, _value) in key_labels.iter() {
        println!("{}", key);
        // TODO: configure input profile
        //new_keys[value.to_owned() as usize] = (InputType::None, 0);
    }

    let new_profile = ui::config::InputProfile {
        keys: new_keys,
        controller_buttons: new_controller_buttons,
        controller_axis: new_controller_axis,
    };
    ui.config.input.input_profiles.insert(profile, new_profile);
}

pub fn get_default_profile() -> ui::config::InputProfile {
    let mut default_controller_buttons = [(false, 0); 18];
    let mut default_controller_axis = [(false, 0, 0); 18];
    let mut default_keys = [(false, 0); 18];
    default_keys[R_DPAD] = (true, sdl2::keyboard::Scancode::D as i32);
    default_keys[L_DPAD] = (true, sdl2::keyboard::Scancode::A as i32);
    default_keys[D_DPAD] = (true, sdl2::keyboard::Scancode::S as i32);
    default_keys[U_DPAD] = (true, sdl2::keyboard::Scancode::W as i32);
    default_keys[START_BUTTON] = (true, sdl2::keyboard::Scancode::Return as i32);
    default_keys[Z_TRIG] = (true, sdl2::keyboard::Scancode::Z as i32);
    default_keys[B_BUTTON] = (true, sdl2::keyboard::Scancode::LCtrl as i32);
    default_keys[A_BUTTON] = (true, sdl2::keyboard::Scancode::LShift as i32);
    default_keys[R_CBUTTON] = (true, sdl2::keyboard::Scancode::L as i32);
    default_keys[L_CBUTTON] = (true, sdl2::keyboard::Scancode::J as i32);
    default_keys[D_CBUTTON] = (true, sdl2::keyboard::Scancode::K as i32);
    default_keys[U_CBUTTON] = (true, sdl2::keyboard::Scancode::I as i32);
    default_keys[R_TRIG] = (true, sdl2::keyboard::Scancode::C as i32);
    default_keys[L_TRIG] = (true, sdl2::keyboard::Scancode::X as i32);
    default_keys[AXIS_LEFT] = (true, sdl2::keyboard::Scancode::Left as i32);
    default_keys[AXIS_RIGHT] = (true, sdl2::keyboard::Scancode::Right as i32);
    default_keys[AXIS_UP] = (true, sdl2::keyboard::Scancode::Up as i32);
    default_keys[AXIS_DOWN] = (true, sdl2::keyboard::Scancode::Down as i32);

    default_controller_buttons[R_DPAD] = (true, sdl2::controller::Button::DPadRight as i32);
    default_controller_buttons[L_DPAD] = (true, sdl2::controller::Button::DPadLeft as i32);
    default_controller_buttons[D_DPAD] = (true, sdl2::controller::Button::DPadDown as i32);
    default_controller_buttons[U_DPAD] = (true, sdl2::controller::Button::DPadUp as i32);
    default_controller_buttons[START_BUTTON] = (true, sdl2::controller::Button::Start as i32);
    default_controller_axis[Z_TRIG] = (true, sdl2::controller::Axis::TriggerLeft as i32, 1);
    default_controller_buttons[B_BUTTON] = (true, sdl2::controller::Button::X as i32);
    default_controller_buttons[A_BUTTON] = (true, sdl2::controller::Button::A as i32);
    default_controller_axis[R_CBUTTON] = (true, sdl2::controller::Axis::RightX as i32, 1);
    default_controller_axis[L_CBUTTON] = (true, sdl2::controller::Axis::RightX as i32, -1);
    default_controller_axis[D_CBUTTON] = (true, sdl2::controller::Axis::RightY as i32, 1);
    default_controller_axis[U_CBUTTON] = (true, sdl2::controller::Axis::RightY as i32, -1);
    default_controller_buttons[R_TRIG] = (true, sdl2::controller::Button::RightShoulder as i32);
    default_controller_buttons[L_TRIG] = (true, sdl2::controller::Button::LeftShoulder as i32);
    default_controller_axis[AXIS_LEFT] = (true, sdl2::controller::Axis::LeftX as i32, -1);
    default_controller_axis[AXIS_RIGHT] = (true, sdl2::controller::Axis::LeftX as i32, 1);
    default_controller_axis[AXIS_UP] = (true, sdl2::controller::Axis::LeftY as i32, -1);
    default_controller_axis[AXIS_DOWN] = (true, sdl2::controller::Axis::LeftY as i32, 1);

    ui::config::InputProfile {
        keys: default_keys,
        controller_buttons: default_controller_buttons,
        controller_axis: default_controller_axis,
    }
}

pub fn init(ui: &mut ui::Ui) {
    for i in 0..4 {
        let controller_assignment = ui.config.input.controller_assignment[i];
        if controller_assignment.is_some() {
            let controller_result = ui
                .sdl_context
                .as_ref()
                .unwrap()
                .game_controller()
                .unwrap()
                .open(controller_assignment.unwrap());
            if controller_result.is_err() {
                let joystick_result = ui
                    .sdl_context
                    .as_ref()
                    .unwrap()
                    .joystick()
                    .unwrap()
                    .open(controller_assignment.unwrap());
                if joystick_result.is_err() {
                    println!("could not connect joystick")
                } else {
                    ui.controllers[i].joystick = Some(joystick_result.unwrap());
                }
            } else {
                ui.controllers[i].game_controller = Some(controller_result.unwrap());
            }
        }
    }
}
