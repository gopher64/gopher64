use crate::ui;

extern "C" {
    pub fn vk_init(rdram_ptr: usize, rdram_size: u32, fullscreen: u8);
    pub fn set_sdl_window(window: usize);
    pub fn rdp_update_screen() -> u8;
    pub fn rdp_set_vi_register(reg: u32, value: u32);
    pub fn rdp_process_commands(dpc_regs: &mut [u32; 8], SP_DMEM: &mut [u8; 8192]) -> u64;
    pub fn full_sync();
}

pub fn init(ui: &mut ui::Ui, rdram_ptr: *mut u8, rdram_size: usize, fullscreen: bool) {
    let mut builder = ui
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
    ui.window = Some(builder.build().unwrap());
    unsafe {
        set_sdl_window(ui.window.as_mut().unwrap().raw() as usize);
        vk_init(rdram_ptr as usize, rdram_size as u32, fullscreen as u8)
    }
}

pub fn update_screen() -> u8 {
    // when the window is closed, running is set to 0
    
    unsafe { rdp_update_screen() }
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        rdp_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list(dpc_regs: &mut [u32; 8], sp_dmem: &mut [u8; 8192]) -> u64 {
    
    unsafe { rdp_process_commands(dpc_regs, sp_dmem) }
}

pub fn rdp_full_sync() {
    unsafe {
        full_sync();
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
