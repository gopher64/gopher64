#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
include!(concat!(env!("OUT_DIR"), "/parallel_bindings.rs"));
use crate::{device, retroachievements, ui};

const PAL_WIDESCREEN_WIDTH: i32 = 512;
const PAL_STANDARD_WIDTH: i32 = 384;
const PAL_HEIGHT: i32 = 288;
const NTSC_WIDESCREEN_WIDTH: i32 = 426;
const NTSC_STANDARD_WIDTH: i32 = 320;
const NTSC_HEIGHT: i32 = 240;

fn build_gfx_info(device: &mut device::Device) -> GFX_INFO {
    GFX_INFO {
        RDRAM: device.rdram.mem.as_mut_ptr(),
        DMEM: device.rsp.mem.as_mut_ptr(),
        RDRAM_SIZE: device.rdram.size,
        DPC_CURRENT_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_CURRENT_REG as usize],
        DPC_START_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_START_REG as usize],
        DPC_END_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_END_REG as usize],
        DPC_STATUS_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_STATUS_REG as usize],
        PAL: device.cart.pal,
        widescreen: device.ui.config.video.widescreen,
        fullscreen: device.ui.video.fullscreen,
        vsync: if device.netplay.is_none() {
            device.ui.config.video.vsync
        } else {
            false
        },
        integer_scaling: device.ui.config.video.integer_scaling,
        upscale: device.ui.config.video.upscale,
        crt: device.ui.config.video.crt,
    }
}

pub fn init(device: &mut device::Device) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_VIDEO);
    ui::ttf_init();

    let window_title = std::ffi::CString::new("gopher64").unwrap();

    let mut flags = sdl3_sys::video::SDL_WINDOW_VULKAN
        | sdl3_sys::video::SDL_WINDOW_RESIZABLE
        | sdl3_sys::video::SDL_WINDOW_INPUT_FOCUS;

    if device.ui.video.fullscreen {
        flags |= sdl3_sys::video::SDL_WINDOW_FULLSCREEN;
    }

    let window_width;
    let window_height;
    let scale = if device.ui.config.video.upscale > 1 {
        device.ui.config.video.upscale as i32
    } else {
        2
    };
    if device.cart.pal {
        window_width = if device.ui.config.video.widescreen {
            PAL_WIDESCREEN_WIDTH * scale
        } else {
            PAL_STANDARD_WIDTH * scale
        };
        window_height = PAL_HEIGHT * scale;
    } else {
        window_width = if device.ui.config.video.widescreen {
            NTSC_WIDESCREEN_WIDTH * scale
        } else {
            NTSC_STANDARD_WIDTH * scale
        };
        window_height = NTSC_HEIGHT * scale;
    }
    device.ui.video.window = unsafe {
        sdl3_sys::video::SDL_CreateWindow(window_title.as_ptr(), window_width, window_height, flags)
    };
    if device.ui.video.window.is_null() {
        panic!("Could not create window: {}", unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::error::SDL_GetError())
                .to_str()
                .unwrap()
        });
    }
    if !unsafe { sdl3_sys::video::SDL_ShowWindow(device.ui.video.window) } {
        panic!("Could not show window: {}", unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::error::SDL_GetError())
                .to_str()
                .unwrap()
        });
    }
    unsafe {
        sdl3_sys::everything::SDL_HideCursor();
        let hint = std::ffi::CString::new("1").unwrap();
        sdl3_sys::everything::SDL_SetHint(
            sdl3_sys::everything::SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS,
            hint.as_ptr(),
        );
    }

    let gfx_info = build_gfx_info(device);

    unsafe {
        let font_bytes = include_bytes!("../../data/RobotoMono-Regular.ttf");
        rdp_init(
            device.ui.video.window as *mut std::ffi::c_void,
            gfx_info,
            font_bytes.as_ptr() as *const std::ffi::c_void,
            font_bytes.len(),
            device.ui.storage.save_state_slot,
        )
    }

    fps_counter(&mut device.ui);
}

fn fps_counter(ui: &mut ui::Ui) {
    let mut fps_rx = ui.video.fps_rx.take().unwrap();
    let mut vis_rx = ui.video.vis_rx.take().unwrap();
    tokio::spawn(async move {
        loop {
            let mut fps: u32 = 0;
            let mut vis: u32 = 0;
            loop {
                match fps_rx.try_recv() {
                    Ok(_fps_update) => fps += 1,
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return,
                }
            }
            loop {
                match vis_rx.try_recv() {
                    Ok(_vis_update) => vis += 1,
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return,
                }
            }
            unsafe { rdp_set_fps(fps, vis) };
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

pub fn close(ui: &ui::Ui) {
    unsafe {
        rdp_close();
        sdl3_sys::video::SDL_DestroyWindow(ui.video.window);
    }
}

pub fn update_screen() {
    unsafe { rdp_update_screen() }
}

pub fn render_frame() {
    unsafe { rdp_render_frame() }
}

pub fn state_size() -> usize {
    unsafe { rdp_state_size() }
}

pub fn save_state(rdp_state: *mut u8) {
    unsafe { rdp_save_state(rdp_state) }
}

pub fn load_state(device: &mut device::Device, rdp_state: *const u8) {
    let gfx_info = build_gfx_info(device);
    unsafe {
        rdp_new_processor(gfx_info);
        rdp_load_state(rdp_state);
        for reg in 0..device::vi::VI_REGS_COUNT {
            rdp_set_vi_register(reg, device.vi.regs[reg as usize])
        }
    }
}

pub fn pause_loop(frame_time: f64) {
    let mut paused = true;
    let mut frame_advance = false;
    while paused && !frame_advance {
        std::thread::sleep(std::time::Duration::from_secs_f64(frame_time));
        unsafe { sdl3_sys::events::SDL_PumpEvents() };
        retroachievements::do_idle();
        let callback = unsafe { rdp_check_callback() };
        paused = callback.paused;
        frame_advance = callback.frame_advance;
    }
}

pub fn check_callback(device: &mut device::Device) -> (bool, bool) {
    let mut speed_limiter_toggled = false;
    let callback = unsafe { rdp_check_callback() };
    device.cpu.running = callback.emu_running;
    if device.netplay.is_none() {
        if callback.save_state {
            device.save_state = true;
        } else if callback.load_state {
            device.load_state = true;
        }
        if callback.reset_game {
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG as usize] |=
                device::cop0::COP0_CAUSE_IP4;
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG as usize] &=
                !device::cop0::COP0_CAUSE_EXCCODE_MASK;

            device::events::create_event(
                device,
                device::events::EVENT_TYPE_NMI,
                device.cpu.clock_rate, // 1 second
            );
        }
        if device.vi.enable_speed_limiter != callback.enable_speedlimiter {
            speed_limiter_toggled = true;
            device.vi.enable_speed_limiter = callback.enable_speedlimiter;
        }
    }

    if device.ui.storage.save_state_slot != callback.save_state_slot {
        onscreen_message(
            &format!("Switching savestate slot to {}", callback.save_state_slot),
            ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
        );
        device.ui.storage.save_state_slot = callback.save_state_slot;
        device
            .ui
            .storage
            .paths
            .savestate_file_path
            .set_extension(format!("state{}", callback.save_state_slot));
    }
    if callback.lower_volume {
        ui::audio::lower_audio_volume(&mut device.ui);
    } else if callback.raise_volume {
        ui::audio::raise_audio_volume(&mut device.ui);
    }
    (speed_limiter_toggled, callback.paused)
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        rdp_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list() -> u64 {
    unsafe { rdp_process_commands() }
}

pub fn check_framebuffers(address: u32, length: u32) {
    unsafe { rdp_check_framebuffers(address, length) }
}

pub fn onscreen_message(message: &str, milliseconds: MESSAGE_LENGTH) {
    unsafe {
        let c_message = std::ffi::CString::new(message).unwrap();
        rdp_onscreen_message(c_message.as_ptr(), milliseconds)
    };
}

pub fn draw_text(
    text: &str,
    renderer: *mut sdl3_sys::render::SDL_Renderer,
    text_engine: *mut sdl3_ttf_sys::ttf::TTF_TextEngine,
    font: *mut sdl3_ttf_sys::ttf::TTF_Font,
) {
    unsafe {
        let (mut w, mut h) = (0, 0);
        sdl3_sys::render::SDL_GetRenderOutputSize(renderer, &mut w, &mut h);

        let c_text = std::ffi::CString::new(text).unwrap();
        let ttf_text = sdl3_ttf_sys::ttf::TTF_CreateText(text_engine, font, c_text.as_ptr(), 0);

        sdl3_sys::everything::SDL_RenderClear(renderer);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(ttf_text, 20.0, h as f32 / 2.0);
        sdl3_sys::render::SDL_RenderPresent(renderer);
        sdl3_ttf_sys::ttf::TTF_DestroyText(ttf_text);
    }
}
