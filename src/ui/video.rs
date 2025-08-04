#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/parallel_bindings.rs"));
use crate::{device, ui};

pub fn init(device: &mut device::Device) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_VIDEO);

    let title = std::ffi::CString::new("gopher64").unwrap();

    let mut flags = sdl3_sys::video::SDL_WINDOW_VULKAN
        | sdl3_sys::video::SDL_WINDOW_RESIZABLE
        | sdl3_sys::video::SDL_WINDOW_INPUT_FOCUS;

    if device.ui.video.fullscreen {
        flags |= sdl3_sys::video::SDL_WINDOW_FULLSCREEN;
    }

    let window_width;
    let window_height;
    if device.cart.pal {
        window_width = if device.ui.config.video.widescreen {
            1024
        } else {
            768
        };
        window_height = 576;
    } else {
        window_width = if device.ui.config.video.widescreen {
            852
        } else {
            640
        };
        window_height = 480;
    }
    device.ui.video.window = unsafe {
        sdl3_sys::video::SDL_CreateWindow(title.as_ptr(), window_width, window_height, flags)
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
        sdl3_sys::everything::SDL_SetHint(
            sdl3_sys::everything::SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS,
            std::ffi::CString::new("1").unwrap().as_ptr(),
        );
    }

    let gfx_info = GFX_INFO {
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
        integer_scaling: device.ui.config.video.integer_scaling,
        upscale: device.ui.config.video.upscale,
        crt: device.ui.config.video.crt,
    };

    unsafe { rdp_init(device.ui.video.window as *mut std::ffi::c_void, gfx_info) }
}

pub fn close(ui: &ui::Ui) {
    unsafe {
        rdp_close();
        sdl3_sys::video::SDL_DestroyWindow(ui.video.window);
    }
}

pub fn update_screen() {
    // when the window is closed, running is set to false
    unsafe { rdp_update_screen() }
}

pub fn state_size() -> usize {
    unsafe { rdp_state_size() }
}

pub fn save_state(rdp_state: *mut u8) {
    unsafe { rdp_save_state(rdp_state) }
}

pub fn load_state(device: &mut device::Device, rdp_state: *const u8) {
    let gfx_info = GFX_INFO {
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
        integer_scaling: device.ui.config.video.integer_scaling,
        upscale: device.ui.config.video.upscale,
        crt: device.ui.config.video.crt,
    };
    unsafe {
        rdp_new_processor(gfx_info);
        rdp_load_state(rdp_state);
        for reg in 0..device::vi::VI_REGS_COUNT {
            rdp_set_vi_register(reg, device.vi.regs[reg as usize])
        }
    }
}

pub fn check_callback(device: &mut device::Device) -> bool {
    let mut speed_limiter_toggled = false;
    let mut callback = unsafe { rdp_check_callback() };
    device.cpu.running = callback.emu_running;
    if device.netplay.is_none() {
        if callback.save_state {
            device.save_state = true;
        } else if callback.load_state {
            device.load_state = true;
        }
        if device.vi.enable_speed_limiter != callback.enable_speedlimiter {
            speed_limiter_toggled = true;
            device.vi.enable_speed_limiter = callback.enable_speedlimiter;
        }
        while callback.paused {
            std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / 60.0));
            unsafe { sdl3_sys::events::SDL_PumpEvents() };
            callback = unsafe { rdp_check_callback() };
        }
    }

    if device.ui.storage.save_state_slot != callback.save_state_slot {
        ui::video::onscreen_message(
            &device.ui,
            &format!("Switching save state slot to {}", callback.save_state_slot,),
        );
        device.ui.storage.save_state_slot = callback.save_state_slot;
        device
            .ui
            .storage
            .paths
            .savestate_file_path
            .set_extension("state".to_owned() + callback.save_state_slot.to_string().as_str());
    }
    if callback.lower_volume {
        ui::audio::lower_audio_volume(&mut device.ui);
    } else if callback.raise_volume {
        ui::audio::raise_audio_volume(&mut device.ui);
    }
    speed_limiter_toggled
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        rdp_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list() -> u64 {
    unsafe { rdp_process_commands() }
}

pub fn onscreen_message(_ui: &ui::Ui, message: &str) {
    println!("Onscreen message: {message}");
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
        let ttf_text = sdl3_ttf_sys::ttf::TTF_CreateText(
            text_engine,
            font,
            c_text.as_ptr(),
            c_text.count_bytes(),
        );

        sdl3_sys::everything::SDL_RenderClear(renderer);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(ttf_text, 20.0, h as f32 / 2.0);
        sdl3_sys::render::SDL_RenderPresent(renderer);
        sdl3_ttf_sys::ttf::TTF_DestroyText(ttf_text);
    }
}
