#![allow(dead_code)]
mod cheats;
mod device;
mod netplay;
mod retroachievements;
mod savestates;
mod ui;

#[unsafe(no_mangle)]
pub extern "C" fn run_game(c_rom_contents: *const u8, rom_size: usize) {
    let rom_contents = unsafe { std::slice::from_raw_parts(c_rom_contents, rom_size) };
    let mut device = device::Device::new();
    device::run_game(
        &mut device,
        &rom_contents,
        ui::GameSettings {
            overclock: false,
            disable_expansion_pak: false,
            cheats: std::collections::HashMap::new(),
            load_savestate_slot: None,
        },
    );
}
