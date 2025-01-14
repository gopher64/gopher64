#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/parallel_bindings.rs"));
use crate::device;

pub fn init(device: &mut device::Device, fullscreen: bool) {
    let mut builder = device
        .ui
        .video_subsystem
        .as_ref()
        .unwrap()
        .window("gopher64", 640, 480);

    builder.position_centered().vulkan();

    if fullscreen {
        builder.fullscreen_desktop();
    } else {
        builder.resizable();
    }
    device.ui.window = Some(builder.build().unwrap());

    let gfx_info = GFX_INFO {
        RDRAM: device.rdram.mem.as_mut_ptr(),
        DMEM: device.rsp.mem.as_mut_ptr(),
        RDRAM_SIZE: device.rdram.size,
        DPC_CURRENT_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_CURRENT_REG as usize],
        DPC_START_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_START_REG as usize],
        DPC_END_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_END_REG as usize],
        DPC_STATUS_REG: &mut device.rdp.regs_dpc[device::rdp::DPC_STATUS_REG as usize],
        VI_H_START_REG: &mut device.vi.regs[device::vi::VI_H_START_REG as usize],
        VI_V_START_REG: &mut device.vi.regs[device::vi::VI_V_START_REG as usize],
        VI_X_SCALE_REG: &mut device.vi.regs[device::vi::VI_X_SCALE_REG as usize],
        VI_Y_SCALE_REG: &mut device.vi.regs[device::vi::VI_Y_SCALE_REG as usize],
        VI_WIDTH_REG: &mut device.vi.regs[device::vi::VI_WIDTH_REG as usize],
    };

    unsafe {
        rdp_init(
            device.ui.window.as_ref().unwrap().raw() as *mut std::ffi::c_void,
            gfx_info,
            device.ui.config.video.upscale,
            device.ui.config.video.integer_scaling,
            fullscreen,
        )
    }
}

pub fn close() {
    unsafe {
        rdp_close();
    }
}

pub fn update_screen() -> bool {
    // when the window is closed, running is set to false
    unsafe { rdp_update_screen() }
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        rdp_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list() -> u64 {
    unsafe { rdp_process_commands() }
}

pub fn full_sync() {
    unsafe {
        rdp_full_sync();
    }
}

pub fn draw_text(
    text: &str,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    font: &rusttype::Font,
) {
    let text_size = 32;
    let scale = rusttype::Scale::uniform(text_size as f32);
    let v_metrics = font.v_metrics(scale);
    let offset = rusttype::point(10.0, 10.0 + v_metrics.ascent);

    // Clear the canvas
    canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
    canvas.clear();

    for glyph in font.layout(text, scale, offset) {
        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y + (240 - text_size);
                if v > 0.5 {
                    canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
                    canvas
                        .draw_point(sdl2::rect::Point::new(x, y))
                        .expect("Error drawing pixel");
                }
            });
        }
    }

    // Present the canvas
    canvas.present();
}
