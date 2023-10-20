use crate::ui;

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
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::D) as u32) << 0;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::A) as u32) << 1;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::S) as u32) << 2;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::W) as u32) << 3;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Return) as u32) << 4;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Z) as u32) << 5;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LCtrl) as u32) << 6;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LShift) as u32) << 7;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::L) as u32) << 8;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::J) as u32) << 9;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::K) as u32) << 10;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::I) as u32) << 11;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::C) as u32) << 12;
    keys |= (keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::X) as u32) << 13;

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

    keys |= (x.round() as i8 as u32) << 16;
    keys |= (y.round() as i8 as u32) << 24;
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
