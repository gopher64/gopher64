use crate::device;
use crate::ui;

#[repr(C)]
pub struct GfxInfo {
    pub header: *mut u8,
    pub rdram: *mut u8,
    pub dmem: *mut u8,
    pub imem: *mut u8,
    pub mi_intr_reg: *mut u32,
    pub dpc_start_reg: *mut u32,
    pub dpc_end_reg: *mut u32,
    pub dpc_current_reg: *mut u32,
    pub dpc_status_reg: *mut u32,
    pub dpc_clock_reg: *mut u32,
    pub dpc_bufbusy_reg: *mut u32,
    pub dpc_pipebusy_reg: *mut u32,
    pub dpc_tmem_reg: *mut u32,
    pub vi_status_reg: *mut u32,
    pub vi_origin_reg: *mut u32,
    pub vi_width_reg: *mut u32,
    pub vi_intr_reg: *mut u32,
    pub vi_v_current_line_reg: *mut u32,
    pub vi_timing_reg: *mut u32,
    pub vi_v_sync_reg: *mut u32,
    pub vi_h_sync_reg: *mut u32,
    pub vi_leap_reg: *mut u32,
    pub vi_h_start_reg: *mut u32,
    pub vi_v_start_reg: *mut u32,
    pub vi_v_burst_reg: *mut u32,
    pub vi_x_scale_reg: *mut u32,
    pub vi_y_scale_reg: *mut u32,
    pub check_interrupts: *mut usize,
    pub version: u32,
    pub sp_status_reg: *mut u32,
    pub rdram_size: *mut u32,
}

unsafe extern "C" {
    pub fn lle_init(window: usize, gfx_info: GfxInfo, fullscreen: bool);
    pub fn lle_close();
    pub fn lle_update_screen() -> bool;
    pub fn lle_set_vi_register(reg: u32, value: u32);
    pub fn rdp_process_commands() -> u64;
    pub fn lle_full_sync();
}

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

    let gfx_info = GfxInfo {
        header: device.cart.rom.as_mut_ptr(),
        rdram: device.rdram.mem.as_mut_ptr(),
        dmem: device.rsp.mem.as_mut_ptr(),
        imem: std::ptr::null_mut(),
        mi_intr_reg: &mut device.mi.regs[device::mi::MI_INTR_REG as usize],
        dpc_start_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_START_REG as usize],
        dpc_end_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_END_REG as usize],
        dpc_current_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_CURRENT_REG as usize],
        dpc_status_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_STATUS_REG as usize],
        dpc_clock_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_CLOCK_REG as usize],
        dpc_bufbusy_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_BUFBUSY_REG as usize],
        dpc_pipebusy_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_PIPEBUSY_REG as usize],
        dpc_tmem_reg: &mut device.rdp.regs_dpc[device::rdp::DPC_TMEM_REG as usize],
        vi_status_reg: &mut device.vi.regs[device::vi::VI_STATUS_REG as usize],
        vi_origin_reg: &mut device.vi.regs[device::vi::VI_ORIGIN_REG as usize],
        vi_width_reg: &mut device.vi.regs[device::vi::VI_WIDTH_REG as usize],
        vi_intr_reg: &mut device.vi.regs[device::vi::VI_V_INTR_REG as usize],
        vi_v_current_line_reg: &mut device.vi.regs[device::vi::VI_CURRENT_REG as usize],
        vi_timing_reg: &mut device.vi.regs[device::vi::VI_BURST_REG as usize],
        vi_v_sync_reg: &mut device.vi.regs[device::vi::VI_V_SYNC_REG as usize],
        vi_h_sync_reg: &mut device.vi.regs[device::vi::VI_H_SYNC_REG as usize],
        vi_leap_reg: &mut device.vi.regs[device::vi::VI_LEAP_REG as usize],
        vi_h_start_reg: &mut device.vi.regs[device::vi::VI_H_START_REG as usize],
        vi_v_start_reg: &mut device.vi.regs[device::vi::VI_V_START_REG as usize],
        vi_v_burst_reg: &mut device.vi.regs[device::vi::VI_V_BURST_REG as usize],
        vi_x_scale_reg: &mut device.vi.regs[device::vi::VI_X_SCALE_REG as usize],
        vi_y_scale_reg: &mut device.vi.regs[device::vi::VI_Y_SCALE_REG as usize],
        check_interrupts: std::ptr::null_mut(),
        version: 2,
        sp_status_reg: &mut device.rsp.regs[device::rsp_interface::SP_STATUS_REG as usize],
        rdram_size: &mut device.rdram.size,
    };

    unsafe {
        lle_init(
            device.ui.window.as_mut().unwrap().raw() as usize,
            gfx_info,
            fullscreen,
        )
    }
}

pub fn close(ui: &mut ui::Ui) {
    unsafe {
        lle_close();
    }
}

pub fn update_screen() -> bool {
    // when the window is closed, running is set to false
    unsafe { lle_update_screen() }
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        lle_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list() -> u64 {
    unsafe { rdp_process_commands() }
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
