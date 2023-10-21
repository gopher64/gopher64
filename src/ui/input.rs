use crate::ui;

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
pub enum InputType {
    Keyboard,
    JoystickHat,
    JoystickButton,
    JoystickAxis,
}

pub const R_DPAD: u32 = 0;
pub const L_DPAD: u32 = 1;
pub const D_DPAD: u32 = 2;
pub const U_DPAD: u32 = 3;
pub const START_BUTTON: u32 = 4;
pub const Z_TRIG: u32 = 5;
pub const B_BUTTON: u32 = 6;
pub const A_BUTTON: u32 = 7;
pub const R_CBUTTON: u32 = 8;
pub const L_CBUTTON: u32 = 9;
pub const D_CBUTTON: u32 = 10;
pub const U_CBUTTON: u32 = 11;
pub const R_TRIG: u32 = 12;
pub const L_TRIG: u32 = 13;
pub const X_AXIS: u32 = 16;
pub const Y_AXIS: u32 = 24;

pub const MAX_AXIS_VALUE: f64 = 85.0;

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

pub fn get(ui: &mut ui::Ui, _channel: usize) -> u32 {
    let context = ui.sdl_context.as_mut().unwrap();
    let events = context.event_pump().unwrap();
    let keyboard_state = events.keyboard_state();

    let mut keys = 0;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::D) as u32) << R_DPAD;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::A) as u32) << L_DPAD;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::S) as u32) << D_DPAD;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::W) as u32) << U_DPAD;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Return) as u32)
        << START_BUTTON;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Z) as u32) << Z_TRIG;
    keys |=
        (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LCtrl) as u32) << B_BUTTON;
    keys |=
        (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LShift) as u32) << A_BUTTON;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::L) as u32) << R_CBUTTON;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::J) as u32) << L_CBUTTON;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::K) as u32) << D_CBUTTON;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::I) as u32) << U_CBUTTON;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::C) as u32) << R_TRIG;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::X) as u32) << L_TRIG;

    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Left) {
        x = -MAX_AXIS_VALUE
    } else if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Right) {
        x = MAX_AXIS_VALUE
    }
    if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Down) {
        y = -MAX_AXIS_VALUE
    } else if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Up) {
        y = MAX_AXIS_VALUE
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

pub fn assign_controller(ui: &mut ui::Ui, controller: usize, port: usize) {
    let joystick = ui.joystick_subsystem.as_ref().unwrap();
    let num_joysticks = joystick.num_joysticks().unwrap();
    if controller < num_joysticks as usize {
        ui.config.input.controller_assignment[port - 1] = Some(controller);
    } else {
        println!("Invalid controller number")
    }
}

pub fn bind_input_profile(ui: &mut ui::Ui, profile: String, port: usize) {
    if ui.config.input.input_profiles.contains_key(&profile) {
        ui.config.input.input_profile_binding[port - 1] = Some(profile);
    } else {
        println!("Invalid profile name")
    }
}

pub fn clear_bindings(ui: &mut ui::Ui) {
    for i in 0..4 {
        ui.config.input.controller_assignment[i] = None;
        ui.config.input.input_profile_binding[i] = None;
    }
}

pub fn configure_input_profile(ui: &mut ui::Ui, profile: String) {
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
    let mut new_keys = [(InputType::Keyboard, 0); 14];

    for (key, value) in key_labels.iter() {
        println!("{}", key);
        new_keys[value.to_owned() as usize] = (InputType::Keyboard, 0);
    }

    let new_profile = ui::config::InputProfile { keys: new_keys };
    ui.config.input.input_profiles.insert(profile, new_profile);
}
