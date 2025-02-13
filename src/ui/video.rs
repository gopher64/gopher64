#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/parallel_bindings.rs"));
use crate::{device, ui};

pub fn init(device: &mut device::Device) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_VIDEO);

    let title = std::ffi::CString::new("gopher64").unwrap();

    let mut flags = sdl3_sys::video::SDL_WINDOW_VULKAN;

    if device.ui.fullscreen {
        flags |= sdl3_sys::video::SDL_WINDOW_FULLSCREEN;
    } else {
        flags |= sdl3_sys::video::SDL_WINDOW_RESIZABLE;
    }
    flags |= sdl3_sys::video::SDL_WINDOW_INPUT_FOCUS;

    let mut window_width = 640;
    let mut window_height = 480;
    if device.cart.pal {
        window_width = 768;
        window_height = 576;
    }
    device.ui.window = unsafe {
        sdl3_sys::video::SDL_CreateWindow(title.as_ptr(), window_width, window_height, flags)
    };
    if device.ui.window.is_null() {
        panic!("Could not create window");
    }
    if !unsafe { sdl3_sys::video::SDL_ShowWindow(device.ui.window) } {
        panic!("Could not show window");
    }
    unsafe { sdl3_sys::everything::SDL_HideCursor() };

    let gfx_info = GFX_INFO {
        RDRAM: device.rdram.mem.as_mut_ptr(),
        DMEM: device.rsp.mem.as_mut_ptr(),
        RDRAM_SIZE: device.rdram.size,
        DPC_CURRENT_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_CURRENT_REG as usize],
        DPC_START_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_START_REG as usize],
        DPC_END_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_END_REG as usize],
        DPC_STATUS_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_STATUS_REG as usize],
        PAL: device.cart.pal,
    };

    unsafe {
        rdp_init(
            device.ui.window as *mut std::ffi::c_void,
            gfx_info,
            device.ui.config.video.upscale,
            device.ui.config.video.integer_scaling,
            device.ui.fullscreen,
        )
    }
}

pub fn close(ui: &ui::Ui) {
    unsafe {
        rdp_close();
        sdl3_sys::video::SDL_DestroyWindow(ui.window);
    }
}

pub fn update_screen() {
    // when the window is closed, running is set to false
    unsafe { rdp_update_screen() }
}

pub fn load_state(device: &mut device::Device) {
    let gfx_info = GFX_INFO {
        RDRAM: device.rdram.mem.as_mut_ptr(),
        DMEM: device.rsp.mem.as_mut_ptr(),
        RDRAM_SIZE: device.rdram.size,
        DPC_CURRENT_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_CURRENT_REG as usize],
        DPC_START_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_START_REG as usize],
        DPC_END_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_END_REG as usize],
        DPC_STATUS_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_STATUS_REG as usize],
        PAL: device.cart.pal,
    };
    unsafe {
        rdp_new_processor(gfx_info, device.ui.config.video.upscale);
    }
}

pub fn check_callback(device: &mut device::Device) {
    let callback = unsafe { rdp_check_callback() };
    device.cpu.running = callback.emu_running;
    if device.netplay.is_none() {
        if callback.save_state {
            device.save_state = true;
        } else if callback.load_state {
            device.load_state = true;
        }
    }
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        rdp_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list() -> u64 {
    unsafe { rdp_process_commands() }
}

pub fn draw_text(text: &str, renderer: *mut sdl3_sys::render::SDL_Renderer, font: &rusttype::Font) {
    let text_size = 32;
    let scale = rusttype::Scale::uniform(text_size as f32);
    let v_metrics = font.v_metrics(scale);
    let offset = rusttype::point(10.0, 10.0 + v_metrics.ascent);

    // Clear the canvas
    unsafe {
        sdl3_sys::render::SDL_SetRenderDrawColor(
            renderer,
            0,
            0,
            0,
            sdl3_sys::pixels::SDL_ALPHA_OPAQUE,
        );
        sdl3_sys::render::SDL_RenderClear(renderer);
    };

    for glyph in font.layout(text, scale, offset) {
        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y + (240 - text_size);
                if v > 0.5 {
                    unsafe {
                        sdl3_sys::render::SDL_SetRenderDrawColor(
                            renderer,
                            255,
                            255,
                            255,
                            sdl3_sys::pixels::SDL_ALPHA_OPAQUE,
                        );
                        sdl3_sys::render::SDL_RenderPoint(renderer, x as f32, y as f32);
                    };
                }
            });
        }
    }

    // Present the canvas
    if !unsafe { sdl3_sys::render::SDL_RenderPresent(renderer) } {
        panic!("Could not present renderer");
    }
}
