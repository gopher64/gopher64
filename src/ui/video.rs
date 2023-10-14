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
    let running = unsafe { rdp_update_screen() };
    return running;
}

pub fn set_register(reg: u32, value: u32) {
    unsafe {
        rdp_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list(dpc_regs: &mut [u32; 8], sp_dmem: &mut [u8; 8192]) -> u64 {
    let timer = unsafe { rdp_process_commands(dpc_regs, sp_dmem) };
    return timer;
}

pub fn rdp_full_sync() {
    unsafe {
        full_sync();
    }
}
