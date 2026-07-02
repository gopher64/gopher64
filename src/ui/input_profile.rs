use crate::ui;

const R_DPAD: usize = 0;
const L_DPAD: usize = 1;
const D_DPAD: usize = 2;
const U_DPAD: usize = 3;
pub const START_BUTTON: usize = 4;
pub const Z_TRIG: usize = 5;
pub const B_BUTTON: usize = 6;
const A_BUTTON: usize = 7;
const R_CBUTTON: usize = 8;
pub const L_CBUTTON: usize = 9;
const D_CBUTTON: usize = 10;
const U_CBUTTON: usize = 11;
pub const R_TRIG: usize = 12;
pub const L_TRIG: usize = 13;
pub const AXIS_RIGHT: usize = 14;
pub const AXIS_LEFT: usize = 15;
pub const AXIS_DOWN: usize = 16;
pub const AXIS_UP: usize = 17;
pub const HOTKEY: usize = 18;
pub const PROFILE_SIZE: usize = 19;

pub const DEADZONE_DEFAULT: i32 = 5;

const GATHER_TIME: usize = 5;

pub fn configure_input_profile(
    config: &mut ui::config::Config,
    profile_name: String,
    dinput: bool,
    deadzone: i32,
) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_VIDEO);
    ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);
    ui::ttf_init();

    if profile_name == "default" {
        eprintln!("Profile name cannot be default");
        return;
    }
    if profile_name.is_empty() {
        eprintln!("Profile name cannot be empty");
        return;
    }

    let mut profile = if let Some(profile) = config.input.input_profiles.get(&profile_name) {
        profile.clone()
    } else {
        ui::config::InputProfile {
            inputs: [const { [None, None] }; PROFILE_SIZE],
            dinput,
            deadzone,
        }
    };

    let mut open_joysticks: Vec<*mut sdl3_sys::joystick::SDL_Joystick> = Vec::new();
    let mut open_controllers: Vec<*mut sdl3_sys::gamepad::SDL_Gamepad> = Vec::new();

    for joystick in ui::input::get_joysticks().iter() {
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
    #[cfg(target_os = "android")]
    let window_flags = sdl3_sys::video::SDL_WINDOW_FULLSCREEN;
    #[cfg(not(target_os = "android"))]
    let window_flags = sdl3_sys::video::SDL_WindowFlags(0);
    if !unsafe {
        sdl3_sys::render::SDL_CreateWindowAndRenderer(
            title.as_ptr(),
            852,
            480,
            window_flags,
            &mut window,
            &mut renderer,
        )
    } {
        panic!("Could not create window and renderer")
    }
    if !unsafe { sdl3_sys::video::SDL_ShowWindow(window) } {
        panic!("Could not show window")
    }

    let text_engine = unsafe { sdl3_ttf_sys::ttf::TTF_CreateRendererTextEngine(renderer) };
    let font = unsafe {
        let font_bytes = include_bytes!("../../data/ui/RobotoMono-Regular.ttf");
        sdl3_ttf_sys::ttf::TTF_OpenFontIO(
            sdl3_sys::everything::SDL_IOFromConstMem(
                font_bytes.as_ptr() as *const std::ffi::c_void,
                font_bytes.len(),
            ),
            true,
            12.0,
        )
    };

    set_font_size(window, font);

    if draw_profile_screen(window, renderer, text_engine, font, &mut profile, dinput) {
        profile.deadzone = deadzone;
        profile.dinput = dinput;
        config.input.input_profiles.insert(profile_name, profile);
    }

    for joystick in open_joysticks {
        unsafe { sdl3_sys::joystick::SDL_CloseJoystick(joystick) }
    }
    for controller in open_controllers {
        unsafe { sdl3_sys::gamepad::SDL_CloseGamepad(controller) }
    }

    unsafe {
        sdl3_ttf_sys::ttf::TTF_CloseFont(font);
        sdl3_ttf_sys::ttf::TTF_DestroyRendererTextEngine(text_engine);
        sdl3_sys::render::SDL_DestroyRenderer(renderer);
        sdl3_sys::video::SDL_DestroyWindow(window);
    }
}

fn set_font_size(window: *mut sdl3_sys::video::SDL_Window, font: *mut sdl3_ttf_sys::ttf::TTF_Font) {
    let mut rect: sdl3_sys::rect::SDL_Rect = Default::default();
    unsafe {
        sdl3_ttf_sys::ttf::TTF_SetFontSize(font, 12.0);
        sdl3_sys::video::SDL_GetWindowSafeArea(window, &mut rect)
    };
    let num_rows = PROFILE_SIZE + 2;
    let min_row_height = rect.h / num_rows as i32;
    while unsafe { sdl3_ttf_sys::ttf::TTF_GetFontLineSkip(font) } < min_row_height {
        unsafe {
            let font_size = sdl3_ttf_sys::ttf::TTF_GetFontSize(font);
            sdl3_ttf_sys::ttf::TTF_SetFontSize(font, font_size + 0.5);
        }
    }
}

fn reset_gather(auto_advance: bool) -> (usize, Option<std::time::Instant>) {
    if auto_advance {
        (GATHER_TIME, Some(std::time::Instant::now()))
    } else {
        (0, None)
    }
}

fn advance_cursor(
    selected: &mut (usize, usize),
    auto_advance: &mut bool,
) -> (usize, Option<std::time::Instant>) {
    selected.0 += 1;
    if selected.0 == PROFILE_SIZE {
        *auto_advance = false;
    }
    reset_gather(*auto_advance)
}

fn draw_profile_screen(
    window: *mut sdl3_sys::video::SDL_Window,
    renderer: *mut sdl3_sys::render::SDL_Renderer,
    text_engine: *mut sdl3_ttf_sys::ttf::TTF_TextEngine,
    font: *mut sdl3_ttf_sys::ttf::TTF_Font,
    profile: &mut ui::config::InputProfile,
    dinput: bool,
) -> bool {
    let mut selected: (usize, usize) = (0, 0);
    let mut auto_advance = dinput && !unsafe { sdl3_sys::keyboard::SDL_HasKeyboard() };

    let (mut gather, mut gather_timer) = reset_gather(auto_advance);
    loop {
        if let Some(timer) = gather_timer
            && timer.elapsed() > std::time::Duration::from_secs(1)
        {
            gather -= 1;
            if gather > 0 {
                gather_timer = Some(std::time::Instant::now());
            } else {
                (gather, gather_timer) = reset_gather(auto_advance);
            }
        }
        let mut event: sdl3_sys::events::SDL_Event = Default::default();

        if unsafe { sdl3_sys::events::SDL_WaitEventTimeout(&mut event, 20) } {
            match event.event_type() {
                sdl3_sys::events::SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED => {
                    set_font_size(window, font);
                }
                sdl3_sys::events::SDL_EVENT_WINDOW_CLOSE_REQUESTED => break,
                sdl3_sys::events::SDL_EVENT_GAMEPAD_BUTTON_DOWN => {
                    if gather == 0 {
                        match sdl3_sys::gamepad::SDL_GamepadButton(
                            unsafe { event.gbutton.button } as i32
                        ) {
                            sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_DOWN => {
                                move_down(&mut selected);
                            }
                            sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_UP => {
                                move_up(&mut selected);
                            }
                            sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_LEFT => {
                                move_left(&mut selected);
                            }
                            sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_RIGHT => {
                                move_right(&mut selected);
                            }
                            sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_WEST
                            | sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_NORTH => {
                                delete_binding(selected, profile);
                            }
                            sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_SOUTH
                            | sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_EAST
                                if select_or_save(selected, &mut gather, &mut gather_timer) =>
                            {
                                return true;
                            }
                            _ => (),
                        }
                    } else if !dinput {
                        profile.inputs[selected.0][selected.1] = Some(
                            ui::config::InputItem::ControllerButton(ui::config::InputKeyButton {
                                id: i32::from(unsafe { event.gbutton.button }),
                            }),
                        );
                        (gather, gather_timer) = advance_cursor(&mut selected, &mut auto_advance);
                    }
                }
                sdl3_sys::events::SDL_EVENT_KEY_DOWN => {
                    let scancode = unsafe { event.key }.scancode;
                    if gather == 0 || scancode == sdl3_sys::scancode::SDL_SCANCODE_AC_BACK {
                        match scancode {
                            sdl3_sys::scancode::SDL_SCANCODE_AC_BACK => break,
                            sdl3_sys::scancode::SDL_SCANCODE_DOWN => {
                                move_down(&mut selected);
                            }
                            sdl3_sys::scancode::SDL_SCANCODE_UP => {
                                move_up(&mut selected);
                            }
                            sdl3_sys::scancode::SDL_SCANCODE_LEFT => {
                                move_left(&mut selected);
                            }
                            sdl3_sys::scancode::SDL_SCANCODE_RIGHT => {
                                move_right(&mut selected);
                            }
                            sdl3_sys::scancode::SDL_SCANCODE_DELETE
                            | sdl3_sys::scancode::SDL_SCANCODE_BACKSPACE => {
                                delete_binding(selected, profile);
                            }
                            sdl3_sys::scancode::SDL_SCANCODE_RETURN
                                if select_or_save(selected, &mut gather, &mut gather_timer) =>
                            {
                                return true;
                            }
                            _ => (),
                        }
                    } else {
                        if scancode != sdl3_sys::scancode::SDL_SCANCODE_LALT
                            && scancode != sdl3_sys::scancode::SDL_SCANCODE_RALT
                            && !unsafe { event.key }.repeat
                        {
                            profile.inputs[selected.0][selected.1] =
                                Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
                                    id: i32::from(scancode),
                                }));
                            (gather, gather_timer) =
                                advance_cursor(&mut selected, &mut auto_advance);
                        }
                    }
                }
                sdl3_sys::events::SDL_EVENT_GAMEPAD_AXIS_MOTION => {
                    let axis_value = unsafe { event.gaxis }.value;
                    let axis = unsafe { event.gaxis }.axis;
                    if !dinput
                        && gather != 0
                        && axis_value.saturating_abs() > (i16::MAX as i32 * 3 / 4) as i16
                    {
                        profile.inputs[selected.0][selected.1] =
                            Some(ui::config::InputItem::ControllerAxis(
                                ui::config::InputControllerAxis {
                                    id: i32::from(axis),
                                    axis: axis_value.signum(),
                                    initial_state: 0,
                                },
                            ));
                        (gather, gather_timer) = advance_cursor(&mut selected, &mut auto_advance);
                    }
                }
                sdl3_sys::events::SDL_EVENT_JOYSTICK_BUTTON_DOWN => {
                    if dinput {
                        if gather == 0 {
                            if let Some(ui::config::InputItem::JoystickButton(joystick_button)) =
                                &profile.inputs[D_DPAD][0]
                                && joystick_button.id == unsafe { event.jbutton.button } as i32
                            {
                                move_down(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickButton(
                                joystick_button,
                            )) = &profile.inputs[U_DPAD][0]
                                && joystick_button.id == unsafe { event.jbutton.button } as i32
                            {
                                move_up(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickButton(
                                joystick_button,
                            )) = &profile.inputs[L_DPAD][0]
                                && joystick_button.id == unsafe { event.jbutton.button } as i32
                            {
                                move_left(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickButton(
                                joystick_button,
                            )) = &profile.inputs[R_DPAD][0]
                                && joystick_button.id == unsafe { event.jbutton.button } as i32
                            {
                                move_right(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickButton(
                                joystick_button,
                            )) = &profile.inputs[B_BUTTON][0]
                                && joystick_button.id == unsafe { event.jbutton.button } as i32
                            {
                                delete_binding(selected, profile);
                            } else if let Some(ui::config::InputItem::JoystickButton(
                                joystick_button,
                            )) = &profile.inputs[A_BUTTON][0]
                                && joystick_button.id == unsafe { event.jbutton.button } as i32
                                && select_or_save(selected, &mut gather, &mut gather_timer)
                            {
                                return true;
                            }
                        } else {
                            profile.inputs[selected.0][selected.1] = Some(
                                ui::config::InputItem::JoystickButton(ui::config::InputKeyButton {
                                    id: i32::from(unsafe { event.jbutton.button }),
                                }),
                            );
                            (gather, gather_timer) =
                                advance_cursor(&mut selected, &mut auto_advance);
                        }
                    }
                }
                sdl3_sys::events::SDL_EVENT_JOYSTICK_HAT_MOTION => {
                    let state = unsafe { event.jhat.value };
                    let hat = unsafe { event.jhat.hat };
                    if dinput {
                        if gather == 0 {
                            if let Some(ui::config::InputItem::JoystickHat(joystick_hat)) =
                                &profile.inputs[D_DPAD][0]
                                && joystick_hat.id == hat as i32
                                && joystick_hat.direction == state
                            {
                                move_down(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickHat(joystick_hat)) =
                                &profile.inputs[U_DPAD][0]
                                && joystick_hat.id == hat as i32
                                && joystick_hat.direction == state
                            {
                                move_up(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickHat(joystick_hat)) =
                                &profile.inputs[L_DPAD][0]
                                && joystick_hat.id == hat as i32
                                && joystick_hat.direction == state
                            {
                                move_left(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickHat(joystick_hat)) =
                                &profile.inputs[R_DPAD][0]
                                && joystick_hat.id == hat as i32
                                && joystick_hat.direction == state
                            {
                                move_right(&mut selected);
                            } else if let Some(ui::config::InputItem::JoystickHat(joystick_hat)) =
                                &profile.inputs[B_BUTTON][0]
                                && joystick_hat.id == hat as i32
                                && joystick_hat.direction == state
                            {
                                delete_binding(selected, profile);
                            } else if let Some(ui::config::InputItem::JoystickHat(joystick_hat)) =
                                &profile.inputs[A_BUTTON][0]
                                && joystick_hat.id == hat as i32
                                && joystick_hat.direction == state
                                && select_or_save(selected, &mut gather, &mut gather_timer)
                            {
                                return true;
                            }
                        } else if state != sdl3_sys::joystick::SDL_HAT_CENTERED {
                            profile.inputs[selected.0][selected.1] = Some(
                                ui::config::InputItem::JoystickHat(ui::config::InputJoystickHat {
                                    id: hat as i32,
                                    direction: state,
                                }),
                            );
                            (gather, gather_timer) =
                                advance_cursor(&mut selected, &mut auto_advance);
                        }
                    }
                }
                sdl3_sys::events::SDL_EVENT_JOYSTICK_AXIS_MOTION => {
                    let axis_value = unsafe { event.jaxis }.value;
                    let axis = unsafe { event.jaxis }.axis;
                    let mut initial_state = 0;
                    let has_initial_state = unsafe {
                        sdl3_sys::joystick::SDL_GetJoystickAxisInitialState(
                            sdl3_sys::joystick::SDL_GetJoystickFromID(event.jaxis.which),
                            axis as i32,
                            &mut initial_state,
                        )
                    };
                    initial_state = if has_initial_state {
                        if initial_state < i16::MIN / 2 {
                            i16::MIN
                        } else if initial_state > i16::MAX / 2 {
                            i16::MAX
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    if dinput && gather != 0 && axis_value.abs_diff(initial_state) > (u16::MAX / 4)
                    {
                        profile.inputs[selected.0][selected.1] = Some(
                            ui::config::InputItem::JoystickAxis(ui::config::InputControllerAxis {
                                id: i32::from(axis),
                                axis: axis_value.signum(),
                                initial_state,
                            }),
                        );
                        (gather, gather_timer) = advance_cursor(&mut selected, &mut auto_advance);
                    }
                }
                _ => (),
            }
        }
        unsafe {
            sdl3_sys::everything::SDL_RenderClear(renderer);
            let ttf_text =
                sdl3_ttf_sys::ttf::TTF_CreateText(text_engine, font, std::ptr::null(), 0);
            render_screen(window, renderer, ttf_text, font, profile, selected, gather);
            sdl3_ttf_sys::ttf::TTF_DestroyText(ttf_text);
            sdl3_sys::render::SDL_RenderPresent(renderer);
        }
    }
    false
}

fn move_left(selected: &mut (usize, usize)) {
    selected.1 = selected.1.saturating_sub(1)
}

fn move_right(selected: &mut (usize, usize)) {
    selected.1 = selected.1.saturating_add(1).min(1)
}

fn move_up(selected: &mut (usize, usize)) {
    selected.0 = selected.0.saturating_sub(1)
}

fn move_down(selected: &mut (usize, usize)) {
    selected.0 = selected.0.saturating_add(1).min(PROFILE_SIZE)
}

fn delete_binding(selected: (usize, usize), profile: &mut ui::config::InputProfile) {
    if selected.0 < PROFILE_SIZE {
        profile.inputs[selected.0][selected.1] = None;
    }
}

fn select_or_save(
    selected: (usize, usize),
    gather: &mut usize,
    gather_timer: &mut Option<std::time::Instant>,
) -> bool {
    if selected.0 == PROFILE_SIZE {
        return true;
    } else {
        *gather = GATHER_TIME;
        *gather_timer = Some(std::time::Instant::now());
    }
    false
}

fn render_screen(
    window: *mut sdl3_sys::video::SDL_Window,
    renderer: *mut sdl3_sys::render::SDL_Renderer,
    ttf_text: *mut sdl3_ttf_sys::ttf::TTF_Text,
    font: *mut sdl3_ttf_sys::ttf::TTF_Font,
    profile: &ui::config::InputProfile,
    selected: (usize, usize),
    gather: usize,
) {
    let mut rect: sdl3_sys::rect::SDL_Rect = Default::default();
    unsafe { sdl3_sys::video::SDL_GetWindowSafeArea(window, &mut rect) };

    let row_height = unsafe { sdl3_ttf_sys::ttf::TTF_GetFontLineSkip(font) } as usize;
    let key_labels: [&str; PROFILE_SIZE] = [
        "D Right",
        "D Left",
        "D Down",
        "D Up",
        "Start",
        "Z",
        "B",
        "A",
        "C Right",
        "C Left",
        "C Down",
        "C Up",
        "R",
        "L",
        "Control Stick Right",
        "Control Stick Left",
        "Control Stick Down",
        "Control Stick Up",
        "Hotkey Activator",
    ];

    // headers
    unsafe {
        sdl3_ttf_sys::ttf::TTF_SetFontStyle(
            font,
            sdl3_ttf_sys::ttf::TTF_STYLE_BOLD | sdl3_ttf_sys::ttf::TTF_STYLE_UNDERLINE,
        );
        let c_text = std::ffi::CString::new("Inputs").unwrap();
        sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, c_text.as_ptr(), 0);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(ttf_text, rect.x as f32, rect.y as f32);

        let c_text = std::ffi::CString::new("Binding 1").unwrap();
        sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, c_text.as_ptr(), 0);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(
            ttf_text,
            rect.x as f32 + (rect.w / 3) as f32 * 1.0,
            rect.y as f32,
        );

        let c_text = std::ffi::CString::new("Binding 2").unwrap();
        sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, c_text.as_ptr(), 0);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(
            ttf_text,
            rect.x as f32 + (rect.w / 3) as f32 * 2.0,
            rect.y as f32,
        );
        sdl3_ttf_sys::ttf::TTF_SetFontStyle(font, sdl3_ttf_sys::ttf::TTF_STYLE_NORMAL);
    }

    for (i, label) in key_labels.iter().enumerate() {
        let c_text = std::ffi::CString::new(*label).unwrap();
        unsafe {
            sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, c_text.as_ptr(), 0);
            sdl3_ttf_sys::ttf::TTF_DrawRendererText(
                ttf_text,
                rect.x as f32,
                rect.y as f32 + ((i + 1) * row_height) as f32,
            );

            let binding_1 =
                std::ffi::CString::new(get_binding_text(&profile.inputs[i][0])).unwrap();
            sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, binding_1.as_ptr(), 0);
            draw_binding_text(renderer, ttf_text, rect, row_height, selected, gather, i, 0);

            let binding_2 =
                std::ffi::CString::new(get_binding_text(&profile.inputs[i][1])).unwrap();
            sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, binding_2.as_ptr(), 0);
            draw_binding_text(renderer, ttf_text, rect, row_height, selected, gather, i, 1);
        }
    }
    let c_text = std::ffi::CString::new("Save and Exit").unwrap();
    unsafe {
        sdl3_ttf_sys::ttf::TTF_SetFontStyle(font, sdl3_ttf_sys::ttf::TTF_STYLE_BOLD);
        sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, c_text.as_ptr(), 0);
        let (mut text_w, mut text_h) = (0, 0);
        sdl3_ttf_sys::ttf::TTF_GetTextSize(ttf_text, &mut text_w, &mut text_h);

        if selected.0 == PROFILE_SIZE {
            draw_highlight(
                renderer,
                rect.x as f32 + (rect.w - text_w) as f32 / 2.0,
                rect.y as f32 + ((PROFILE_SIZE + 1) * row_height) as f32,
                text_w,
                text_h,
            );
        }

        sdl3_ttf_sys::ttf::TTF_DrawRendererText(
            ttf_text,
            rect.x as f32 + (rect.w - text_w) as f32 / 2.0,
            rect.y as f32 + ((PROFILE_SIZE + 1) * row_height) as f32,
        );
    };
}

fn draw_highlight(
    renderer: *mut sdl3_sys::render::SDL_Renderer,
    x: f32,
    y: f32,
    text_w: i32,
    text_h: i32,
) {
    unsafe {
        sdl3_sys::render::SDL_SetRenderDrawColor(renderer, 50, 100, 180, 255);

        let highlight = sdl3_sys::rect::SDL_FRect {
            x,
            y,
            w: text_w as f32,
            h: text_h as f32,
        };
        sdl3_sys::render::SDL_RenderFillRect(renderer, &highlight);
        sdl3_sys::render::SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_binding_text(
    renderer: *mut sdl3_sys::render::SDL_Renderer,
    ttf_text: *mut sdl3_ttf_sys::ttf::TTF_Text,
    rect: sdl3_sys::rect::SDL_Rect,
    row_height: usize,
    selected: (usize, usize),
    gather: usize,
    i: usize,
    binding: usize,
) {
    unsafe {
        if selected.0 == i && selected.1 == binding {
            if gather > 0 {
                let gather_text = std::ffi::CString::new(format!("{}...", gather)).unwrap();
                sdl3_ttf_sys::ttf::TTF_SetTextString(ttf_text, gather_text.as_ptr(), 0);
            }
            let (mut text_w, mut text_h) = (0, 0);
            sdl3_ttf_sys::ttf::TTF_GetTextSize(ttf_text, &mut text_w, &mut text_h);
            draw_highlight(
                renderer,
                rect.x as f32 + (rect.w / 3) as f32 * if selected.1 == 0 { 1.0 } else { 2.0 },
                rect.y as f32 + ((i + 1) * row_height) as f32,
                text_w,
                text_h,
            );
        }
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(
            ttf_text,
            rect.x as f32 + (rect.w / 3) as f32 * (binding as f32 + 1.0),
            rect.y as f32 + ((i + 1) * row_height) as f32,
        );
    }
}

fn get_binding_text(binding: &Option<ui::config::InputItem>) -> String {
    if let Some(binding) = binding {
        match binding {
            ui::config::InputItem::Key(key) => {
                return unsafe {
                    std::ffi::CStr::from_ptr(sdl3_sys::keyboard::SDL_GetScancodeName(
                        sdl3_sys::scancode::SDL_Scancode(key.id),
                    ))
                    .to_str()
                    .unwrap()
                    .to_string()
                };
            }
            ui::config::InputItem::ControllerButton(controller_button) => {
                return unsafe {
                    std::ffi::CStr::from_ptr(sdl3_sys::gamepad::SDL_GetGamepadStringForButton(
                        sdl3_sys::gamepad::SDL_GamepadButton(controller_button.id),
                    ))
                    .to_str()
                    .unwrap()
                    .to_string()
                };
            }
            ui::config::InputItem::ControllerAxis(controller_axis) => {
                return unsafe {
                    let axis =
                        std::ffi::CStr::from_ptr(sdl3_sys::gamepad::SDL_GetGamepadStringForAxis(
                            sdl3_sys::gamepad::SDL_GamepadAxis(controller_axis.id),
                        ))
                        .to_str()
                        .unwrap()
                        .to_string();
                    let direction = if controller_axis.axis > 0 { "+" } else { "-" };
                    format!("{} {}", axis, direction)
                };
            }
            ui::config::InputItem::JoystickButton(joystick_button) => {
                return format!("Joystick Button {}", joystick_button.id);
            }
            ui::config::InputItem::JoystickHat(joystick_hat) => {
                let direction = match joystick_hat.direction {
                    1 => "Up",
                    2 => "Right",
                    4 => "Down",
                    8 => "Left",
                    _ => "Unknown",
                };
                return format!("Joystick Hat {} {}", joystick_hat.id, direction);
            }
            ui::config::InputItem::JoystickAxis(joystick_axis) => {
                let direction = if joystick_axis.axis > 0 { "+" } else { "-" };
                return format!("Joystick Axis {} {}", joystick_axis.id, direction);
            }
        }
    }
    "(unset)".to_string()
}

pub fn get_default_profile() -> ui::config::InputProfile {
    let mut inputs: [[Option<ui::config::InputItem>; 2]; PROFILE_SIZE] =
        [const { [None, None] }; PROFILE_SIZE];

    inputs[R_DPAD] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_D),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_RIGHT),
            },
        )),
    ];
    inputs[L_DPAD] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_A),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_LEFT),
            },
        )),
    ];
    inputs[D_DPAD] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_S),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_DOWN),
            },
        )),
    ];
    inputs[U_DPAD] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_W),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_DPAD_UP),
            },
        )),
    ];
    inputs[START_BUTTON] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_RETURN),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_START),
            },
        )),
    ];
    inputs[Z_TRIG] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_Z),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFT_TRIGGER),
                axis: 1,
                initial_state: 0,
            },
        )),
    ];
    inputs[B_BUTTON] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_LCTRL),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_WEST),
            },
        )),
    ];
    inputs[A_BUTTON] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_LSHIFT),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_SOUTH),
            },
        )),
    ];
    inputs[R_CBUTTON] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_L),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTX),
                axis: 1,
                initial_state: 0,
            },
        )),
    ];
    inputs[L_CBUTTON] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_J),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTX),
                axis: -1,
                initial_state: 0,
            },
        )),
    ];
    inputs[D_CBUTTON] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_K),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
                axis: 1,
                initial_state: 0,
            },
        )),
    ];
    inputs[U_CBUTTON] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_I),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_RIGHTY),
                axis: -1,
                initial_state: 0,
            },
        )),
    ];
    inputs[R_TRIG] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_C),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER),
            },
        )),
    ];
    inputs[L_TRIG] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_X),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_LEFT_SHOULDER),
            },
        )),
    ];
    inputs[AXIS_LEFT] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_LEFT),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX),
                axis: -1,
                initial_state: 0,
            },
        )),
    ];
    inputs[AXIS_RIGHT] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_RIGHT),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX),
                axis: 1,
                initial_state: 0,
            },
        )),
    ];
    inputs[AXIS_UP] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_UP),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
                axis: -1,
                initial_state: 0,
            },
        )),
    ];
    inputs[AXIS_DOWN] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_DOWN),
        })),
        Some(ui::config::InputItem::ControllerAxis(
            ui::config::InputControllerAxis {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_AXIS_LEFTY),
                axis: 1,
                initial_state: 0,
            },
        )),
    ];
    inputs[HOTKEY] = [
        Some(ui::config::InputItem::Key(ui::config::InputKeyButton {
            id: i32::from(sdl3_sys::scancode::SDL_SCANCODE_COMMA),
        })),
        Some(ui::config::InputItem::ControllerButton(
            ui::config::InputKeyButton {
                id: i32::from(sdl3_sys::gamepad::SDL_GAMEPAD_BUTTON_BACK),
            },
        )),
    ];

    ui::config::InputProfile {
        inputs,
        dinput: false,
        deadzone: DEADZONE_DEFAULT,
    }
}
