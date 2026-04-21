#![allow(dead_code)]
mod cheats;
mod device;
mod netplay;
mod retroachievements;
mod savestates;
mod ui;

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn gopher64_run_game(rom_contents: *const u8, rom_size: usize) {
    let dirs = ui::get_dirs();

    std::fs::create_dir_all(dirs.config_dir).unwrap();
    std::fs::create_dir_all(dirs.cache_dir).unwrap();
    std::fs::create_dir_all(dirs.data_dir.join("saves")).unwrap();
    std::fs::create_dir_all(dirs.data_dir.join("states")).unwrap();

    let mut device = device::Device::new();

    device.ui.video.fullscreen = device.ui.config.video.fullscreen;
    device::run_game(
        &mut device,
        unsafe { std::slice::from_raw_parts(rom_contents, rom_size) },
        ui::GameSettings {
            overclock: false,
            disable_expansion_pak: false,
            cheats: std::collections::HashMap::new(),
            load_savestate_slot: None,
        },
    );
}
