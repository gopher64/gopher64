#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
include!(concat!(env!("OUT_DIR"), "/parallel_bindings.rs"));
use crate::{device, netplay, retroachievements, ui};

const PAL_WIDESCREEN_WIDTH: i32 = 512;
const PAL_STANDARD_WIDTH: i32 = 384;
const PAL_HEIGHT: i32 = 288;
const NTSC_WIDESCREEN_WIDTH: i32 = 426;
const NTSC_STANDARD_WIDTH: i32 = 320;
const NTSC_HEIGHT: i32 = 240;

fn build_gfx_info(device: &mut device::Device, netplay: bool) -> GFX_INFO {
    GFX_INFO {
        RDRAM: device.rdram.mem.as_mut_ptr(),
        DMEM: device.rsp.mem.as_mut_ptr(),
        RDRAM_SIZE: device.rdram.size,
        DPC_CURRENT_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_CURRENT_REG],
        DPC_START_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_START_REG],
        DPC_END_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_END_REG],
        DPC_STATUS_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_STATUS_REG],
        PAL: device.cart.pal,
        widescreen: device.ui.config.video.widescreen,
        fullscreen: device.ui.video.fullscreen,
        vsync: if !netplay {
            device.ui.config.video.vsync
        } else {
            false
        },
        integer_scaling: device.ui.config.video.integer_scaling,
        upscale: device.ui.config.video.upscale,
        ssaa: device.ui.config.video.ssaa,
        crt: device.ui.config.video.crt,
    }
}

pub fn init(device: &mut device::Device, netplay: bool) {
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
    let scale = if device.ui.config.video.upscale > 1 && !device.ui.config.video.ssaa {
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
        let err = sdl3_sys::error::SDL_GetError();
        panic!(
            "Could not create window: {}",
            if err.is_null() {
                "Unknown error"
            } else {
                unsafe { std::ffi::CStr::from_ptr(err).to_str().unwrap() }
            }
        );
    }
    if !unsafe { sdl3_sys::video::SDL_ShowWindow(device.ui.video.window) } {
        let err = sdl3_sys::error::SDL_GetError();
        panic!(
            "Could not show window: {}",
            if err.is_null() {
                "Unknown error"
            } else {
                unsafe { std::ffi::CStr::from_ptr(err).to_str().unwrap() }
            }
        );
    }
    unsafe {
        sdl3_sys::everything::SDL_HideCursor();
    }

    let gfx_info = build_gfx_info(device, netplay);

    unsafe {
        let font_bytes = include_bytes!("../../data/ui/RobotoMono-Regular.ttf");
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

pub fn idle() {
    unsafe { rdp_idle() }
}

pub fn load_state(device: &mut device::Device, rdp_state: *const u8) {
    let gfx_info = build_gfx_info(device, device.netplay.is_some());
    unsafe {
        rdp_load_state(gfx_info, rdp_state);
        for reg in 0..device::vi::VI_REGS_COUNT {
            rdp_set_vi_register(reg as u32, device.vi.regs[reg])
        }
    }
}

pub fn pause_loop(ui: &mut ui::Ui, frame_time: f64) {
    let mut paused = true;
    let mut frame_advance = false;
    let mut pause_counter = 0;
    while paused && !frame_advance {
        std::thread::sleep(std::time::Duration::from_secs_f64(frame_time));
        ui::input::get(ui, 0, pause_counter); // to gather hotkey input
        unsafe { sdl3_sys::events::SDL_PumpEvents() };
        retroachievements::do_idle();
        let callback = unsafe { rdp_check_callback() };
        paused = callback.paused;
        frame_advance = callback.frame_advance;
        pause_counter += 1;
    }
}

pub fn check_callback(device: &mut device::Device) -> (bool, bool) {
    let mut speed_limiter_toggled = false;
    let callback = unsafe { rdp_check_callback() };
    device.cpu.running = callback.emu_running;
    if device.netplay.is_none() {
        if callback.save_state {
            device.savestate.save_state = true;
        } else if callback.load_state {
            device.savestate.load_state = true;
        }
        if callback.load_rewind && device.ui.config.emulation.rewind {
            device.savestate.load_rewind = true;
        }
        if callback.reset_game {
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG] |= device::cop0::COP0_CAUSE_IP4;
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG] &=
                !device::cop0::COP0_CAUSE_EXCCODE_MASK;

            device::events::create_event(
                device,
                device::events::EVENT_TYPE_NMI,
                device.cpu.clock_rate, // 1 second
            );
        }
        if device.speed_limiter.enabled != callback.enable_speedlimiter {
            speed_limiter_toggled = true;
            device.speed_limiter.enabled = callback.enable_speedlimiter;
        }
    }

    if device.ui.storage.save_state_slot != callback.save_state_slot {
        onscreen_message(
            &format!("Switching savestate slot to {}", callback.save_state_slot),
            ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
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

    if let Some(netplay) = &mut device.netplay
        && netplay.player_number == 0
    {
        if callback.decrease_input_delay {
            netplay::send_input_delay(netplay, netplay.input_delay - 1);
        } else if callback.increase_input_delay {
            netplay::send_input_delay(netplay, netplay.input_delay + 1);
        }
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

pub const CANVAS_W: i32 = 852;
pub const CANVAS_H: i32 = 480;

pub fn set_logical_canvas(renderer: *mut sdl3_sys::render::SDL_Renderer) {
    unsafe {
        sdl3_sys::render::SDL_SetRenderLogicalPresentation(
            renderer,
            CANVAS_W,
            CANVAS_H,
            sdl3_sys::render::SDL_LOGICAL_PRESENTATION_LETTERBOX,
        );
    }
}

/// Persistent SDL/TTF handles shared by the profile-config renderers. Bundled
/// into one struct so each renderer stays within clippy's argument budget and
/// the always-together handles travel as a unit.
pub struct ProfileCtx {
    pub renderer: *mut sdl3_sys::render::SDL_Renderer,
    pub image_texture: *mut sdl3_sys::render::SDL_Texture,
    pub text_engine: *mut sdl3_ttf_sys::ttf::TTF_TextEngine,
    /// Large font for the guided capture prompt.
    pub font: *mut sdl3_ttf_sys::ttf::TTF_Font,
    /// Small font for review rows, footer legends and progress text.
    pub list_font: *mut sdl3_ttf_sys::ttf::TTF_Font,
}

/// Logical-space rectangles for touch hit-testing the review list.
pub struct ListHitAreas {
    pub rows: Vec<sdl3_sys::rect::SDL_FRect>,
    pub save: sdl3_sys::rect::SDL_FRect,
    pub cancel: sdl3_sys::rect::SDL_FRect,
}

/// ~1 Hz triangle pulse between 60 and 230 (motion-safe, under 3 Hz).
fn pulse_alpha(ticks_ms: u64) -> u8 {
    let phase = (ticks_ms % 1000) as f32 / 1000.0;
    let tri = 1.0 - (phase * 2.0 - 1.0).abs();
    (60.0 + tri * 170.0) as u8
}

/// Concentric rings approximating a halo around the selected input's pad art.
fn draw_glow(
    renderer: *mut sdl3_sys::render::SDL_Renderer,
    glow: Option<(f32, f32, f32)>,
    ticks_ms: u64,
) {
    let Some((cx, cy, r)) = glow else { return };
    unsafe {
        sdl3_sys::render::SDL_SetRenderDrawBlendMode(
            renderer,
            sdl3_sys::blendmode::SDL_BLENDMODE_BLEND,
        );
        let a = pulse_alpha(ticks_ms);
        let cxp = cx * CANVAS_W as f32;
        let cyp = cy * CANVAS_H as f32;
        let rp = r * CANVAS_W as f32;
        for i in 0..4 {
            let grow = rp + i as f32 * 6.0;
            let alpha = a.saturating_sub(i as u8 * 45);
            sdl3_sys::render::SDL_SetRenderDrawColor(renderer, 255, 213, 74, alpha);
            let rect = sdl3_sys::rect::SDL_FRect {
                x: cxp - grow,
                y: cyp - grow,
                w: grow * 2.0,
                h: grow * 2.0,
            };
            sdl3_sys::render::SDL_RenderRect(renderer, &rect);
        }
    }
}

/// Draw `text` with its top-left at (`x`, `y`).
fn draw_line(
    text_engine: *mut sdl3_ttf_sys::ttf::TTF_TextEngine,
    font: *mut sdl3_ttf_sys::ttf::TTF_Font,
    text: &str,
    x: f32,
    y: f32,
) {
    if text.is_empty() {
        return;
    }
    unsafe {
        let c_text = std::ffi::CString::new(text).unwrap();
        let ttf_text = sdl3_ttf_sys::ttf::TTF_CreateText(text_engine, font, c_text.as_ptr(), 0);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(ttf_text, x, y);
        sdl3_ttf_sys::ttf::TTF_DestroyText(ttf_text);
    }
}

/// Draw `text` horizontally centered on the canvas at vertical position `y`.
fn draw_centered(
    text_engine: *mut sdl3_ttf_sys::ttf::TTF_TextEngine,
    font: *mut sdl3_ttf_sys::ttf::TTF_Font,
    text: &str,
    y: f32,
) {
    if text.is_empty() {
        return;
    }
    unsafe {
        let c_text = std::ffi::CString::new(text).unwrap();
        let ttf_text = sdl3_ttf_sys::ttf::TTF_CreateText(text_engine, font, c_text.as_ptr(), 0);
        let (mut text_w, mut text_h) = (0, 0);
        sdl3_ttf_sys::ttf::TTF_GetTextSize(ttf_text, &mut text_w, &mut text_h);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(ttf_text, (CANVAS_W - text_w) as f32 / 2.0, y);
        sdl3_ttf_sys::ttf::TTF_DestroyText(ttf_text);
    }
}

/// Draw `text` right-aligned, ending 10 px from the canvas' right edge.
fn draw_right(
    text_engine: *mut sdl3_ttf_sys::ttf::TTF_TextEngine,
    font: *mut sdl3_ttf_sys::ttf::TTF_Font,
    text: &str,
    y: f32,
) {
    if text.is_empty() {
        return;
    }
    unsafe {
        let c_text = std::ffi::CString::new(text).unwrap();
        let ttf_text = sdl3_ttf_sys::ttf::TTF_CreateText(text_engine, font, c_text.as_ptr(), 0);
        let (mut text_w, mut text_h) = (0, 0);
        sdl3_ttf_sys::ttf::TTF_GetTextSize(ttf_text, &mut text_w, &mut text_h);
        sdl3_ttf_sys::ttf::TTF_DrawRendererText(ttf_text, (CANVAS_W - 10 - text_w) as f32, y);
        sdl3_ttf_sys::ttf::TTF_DestroyText(ttf_text);
    }
}

/// Guided/single capture screen: dimmed pad art, a pulsing glow on the input
/// to press, a big label naming it, progress, and a skip legend.
pub fn draw_capture(
    ctx: &ProfileCtx,
    label: &str,
    next_label: Option<&str>,
    index: usize,
    total: usize,
    glow: Option<(f32, f32, f32)>,
    ticks_ms: u64,
) {
    unsafe {
        sdl3_sys::render::SDL_SetRenderDrawColor(ctx.renderer, 0, 0, 0, 255);
        sdl3_sys::everything::SDL_RenderClear(ctx.renderer);

        sdl3_sys::render::SDL_SetTextureAlphaMod(ctx.image_texture, 70);
        let full = sdl3_sys::rect::SDL_FRect {
            x: 0.0,
            y: 0.0,
            w: CANVAS_W as f32,
            h: CANVAS_H as f32,
        };
        sdl3_sys::render::SDL_RenderTexture(
            ctx.renderer,
            ctx.image_texture,
            std::ptr::null(),
            &full,
        );
        sdl3_sys::render::SDL_SetTextureAlphaMod(ctx.image_texture, 255);

        draw_glow(ctx.renderer, glow, ticks_ms);

        draw_centered(ctx.text_engine, ctx.font, label, 36.0);
        draw_centered(ctx.text_engine, ctx.list_font, "press any input", 96.0);
        if let Some(next) = next_label {
            draw_centered(
                ctx.text_engine,
                ctx.list_font,
                format!("next: {next}").as_str(),
                122.0,
            );
        }

        let footer = format!("skip: Esc / East / tap      {}/{}", index + 1, total);
        draw_centered(
            ctx.text_engine,
            ctx.list_font,
            footer.as_str(),
            (CANVAS_H - 28) as f32,
        );

        sdl3_sys::render::SDL_RenderPresent(ctx.renderer);
    }
}

/// Review list: faint pad backdrop, full-width rows with the selected row
/// highlighted, a pulsing glow on the selected input's pad location, an
/// on-screen ✕, and (on the empty right half) an optional warning + control
/// legend. Returns the logical-space hit areas for touch.
pub fn draw_review_list(
    ctx: &ProfileCtx,
    rows: &[String],
    selected: usize,
    glow: Option<(f32, f32, f32)>,
    warning: Option<&str>,
    ticks_ms: u64,
) -> ListHitAreas {
    unsafe {
        sdl3_sys::render::SDL_SetRenderDrawColor(ctx.renderer, 0, 0, 0, 255);
        sdl3_sys::everything::SDL_RenderClear(ctx.renderer);

        sdl3_sys::render::SDL_SetTextureAlphaMod(ctx.image_texture, 40);
        let full = sdl3_sys::rect::SDL_FRect {
            x: 0.0,
            y: 0.0,
            w: CANVAS_W as f32,
            h: CANVAS_H as f32,
        };
        sdl3_sys::render::SDL_RenderTexture(
            ctx.renderer,
            ctx.image_texture,
            std::ptr::null(),
            &full,
        );
        sdl3_sys::render::SDL_SetTextureAlphaMod(ctx.image_texture, 255);

        draw_glow(ctx.renderer, glow, ticks_ms);

        let line_h: f32 = 24.0;
        let fh = sdl3_ttf_sys::ttf::TTF_GetFontHeight(ctx.list_font) as f32;
        sdl3_sys::render::SDL_SetRenderDrawBlendMode(
            ctx.renderer,
            sdl3_sys::blendmode::SDL_BLENDMODE_BLEND,
        );

        let mut row_rects: Vec<sdl3_sys::rect::SDL_FRect> = Vec::with_capacity(rows.len());
        for (i, row) in rows.iter().enumerate() {
            let y = i as f32 * line_h;
            let rect = sdl3_sys::rect::SDL_FRect {
                x: 0.0,
                y,
                w: CANVAS_W as f32,
                h: line_h,
            };
            if i == selected {
                sdl3_sys::render::SDL_SetRenderDrawColor(ctx.renderer, 50, 100, 180, 220);
                sdl3_sys::render::SDL_RenderFillRect(ctx.renderer, &rect);
            }
            draw_line(
                ctx.text_engine,
                ctx.list_font,
                row.as_str(),
                10.0,
                y + (line_h - fh) / 2.0,
            );
            row_rects.push(rect);
        }

        // Right-half overlay text (rows are left-aligned and short there).
        if let Some(w) = warning {
            draw_right(ctx.text_engine, ctx.list_font, w, 52.0);
        }
        draw_right(
            ctx.text_engine,
            ctx.list_font,
            "Up/Down move   Enter rebind   Esc quit",
            (CANVAS_H - 22) as f32,
        );

        // On-screen cancel (✕), drawn last so it sits above row 0.
        let cancel = sdl3_sys::rect::SDL_FRect {
            x: (CANVAS_W - 44) as f32,
            y: 4.0,
            w: 40.0,
            h: 40.0,
        };
        sdl3_sys::render::SDL_SetRenderDrawColor(ctx.renderer, 30, 30, 30, 235);
        sdl3_sys::render::SDL_RenderFillRect(ctx.renderer, &cancel);
        sdl3_sys::render::SDL_SetRenderDrawColor(ctx.renderer, 220, 80, 80, 255);
        sdl3_sys::render::SDL_RenderRect(ctx.renderer, &cancel);
        let pad = 11.0;
        sdl3_sys::render::SDL_RenderLine(
            ctx.renderer,
            cancel.x + pad,
            cancel.y + pad,
            cancel.x + cancel.w - pad,
            cancel.y + cancel.h - pad,
        );
        sdl3_sys::render::SDL_RenderLine(
            ctx.renderer,
            cancel.x + cancel.w - pad,
            cancel.y + pad,
            cancel.x + pad,
            cancel.y + cancel.h - pad,
        );

        sdl3_sys::render::SDL_RenderPresent(ctx.renderer);

        let save = row_rects.pop().unwrap_or(sdl3_sys::rect::SDL_FRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        });
        ListHitAreas {
            rows: row_rects,
            save,
            cancel,
        }
    }
}
