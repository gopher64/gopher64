use crate::ui;

unsafe extern "C" {
    pub fn lle_init(rdram_ptr: usize, rdram_size: u32, fullscreen: u8);
    pub fn lle_close();
    pub fn lle_set_sdl_window(window: usize);
    pub fn lle_update_screen() -> bool;
    pub fn lle_set_vi_register(reg: u32, value: u32);
    pub fn rdp_process_commands(dpc_regs: &mut [u32; 8], SP_DMEM: &mut [u8; 8192]) -> u64;
    pub fn lle_full_sync();
    pub fn hle_init();
    pub fn hle_close();
    pub fn hle_process_dlist() -> u64;
    pub fn hle_update_screen() -> bool;
}

pub fn init(ui: &mut ui::Ui, rdram_ptr: *mut u8, rdram_size: usize, fullscreen: bool) {
    let mut builder = ui
        .video_subsystem
        .as_ref()
        .unwrap()
        .window("gopher64", 640, 480);
    if ui.config.video.lle {
        builder.position_centered().vulkan();
    } else {
        builder.position_centered().opengl();
    }
    if fullscreen {
        builder.fullscreen_desktop();
    } else {
        builder.resizable();
    }
    ui.window = Some(builder.build().unwrap());
    if ui.config.video.lle {
        unsafe {
            lle_set_sdl_window(ui.window.as_mut().unwrap().raw() as usize);
            lle_init(rdram_ptr as usize, rdram_size as u32, fullscreen as u8)
        }
    } else {
        ui.gl_context = Some(ui.window.as_ref().unwrap().gl_create_context().unwrap());
        unsafe {
            hle_init();
        }
    }
}

pub fn close(ui: &mut ui::Ui) {
    if ui.config.video.lle {
        unsafe {
            lle_close();
        }
    } else {
        unsafe {
            hle_close();
        }
    }
}

pub fn update_screen(lle: bool) -> bool {
    // when the window is closed, running is set to false
    if lle {
        unsafe { lle_update_screen() }
    } else {
        unsafe { hle_update_screen() }
    }
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        lle_set_vi_register(reg, value);
    }
}

pub fn process_dlist(lle: bool) -> u64 {
    if !lle {
        unsafe { hle_process_dlist() }
    } else {
        panic!("process_dlist in LLE mode")
    }
}

pub fn process_rdp_list(dpc_regs: &mut [u32; 8], sp_dmem: &mut [u8; 8192], lle: bool) -> u64 {
    if lle {
        unsafe { rdp_process_commands(dpc_regs, sp_dmem) }
    } else {
        panic!("process_rdp_list in HLE mode")
    }
}

pub fn rdp_full_sync() {
    unsafe {
        lle_full_sync();
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
