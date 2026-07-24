use crate::ui;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct InputKeyButton {
    pub id: i32,
}

#[derive(PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputControllerAxis {
    pub id: i32,
    pub axis: i16,
    pub initial_state: i16,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct InputJoystickHat {
    pub id: i32,
    pub direction: u8,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum InputItem {
    Key(InputKeyButton),
    ControllerButton(InputKeyButton),
    ControllerAxis(InputControllerAxis),
    JoystickButton(InputKeyButton),
    JoystickHat(InputJoystickHat),
    JoystickAxis(InputControllerAxis),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct InputProfile {
    pub inputs: [[Option<InputItem>; 2]; ui::input_profile::PROFILE_SIZE],
    pub dinput: bool,
    pub deadzone: i32,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub input_profiles: std::collections::BTreeMap<String, InputProfile>,
    pub input_profile_binding: [String; 4],
    pub controller_assignment: [Option<String>; 4],
    pub controller_enabled: [bool; 4],
    pub transfer_pak: [bool; 4],
    pub gb_rom_path: [String; 4],
    pub gb_ram_path: [String; 4],
    pub emulate_vru: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Video {
    pub upscale: u32,
    pub ssaa: bool,
    pub integer_scaling: bool,
    pub fullscreen: bool,
    pub widescreen: bool,
    pub vsync: bool,
    pub crt: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Emulation {
    pub overclock: bool,
    pub disable_expansion_pak: bool,
    pub usb: bool,
    pub rewind: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cheats {
    pub cheats: rustc_hash::FxHashMap<String, rustc_hash::FxHashMap<String, Option<String>>>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Ui {
    pub theme: i32,
    pub rom_dir: std::path::PathBuf,
    pub recent_roms: Vec<String>,
    #[serde(default)]
    pub favorites: Vec<String>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub input: Input,
    pub video: Video,
    pub emulation: Emulation,
    #[serde(default)]
    pub ui: Ui,
    #[serde(skip)]
    write_to_disk: bool,
}

impl Drop for Cheats {
    fn drop(&mut self) {
        write_cheats(self);
    }
}

fn write_cheats(cheats: &Cheats) {
    let dirs = ui::get_dirs();
    let file_path = dirs.config_dir.join("cheats.json");
    if let Ok(f) = std::fs::File::create(file_path)
        && let Err(e) = serde_json::to_writer_pretty(f, &cheats)
    {
        eprintln!("Error writing cheats: {}", e);
    }
}

impl Cheats {
    pub fn new() -> Cheats {
        let dirs = ui::get_dirs();
        let file_path = dirs.config_dir.join("cheats.json");
        let cheats_file = std::fs::read(file_path);
        if let Ok(cheats_file) = cheats_file {
            let result = serde_json::from_slice(cheats_file.as_ref());
            if let Ok(result) = result {
                return result;
            }
        }
        Cheats {
            cheats: rustc_hash::FxHashMap::default(),
        }
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        if self.write_to_disk {
            write_config(self);
        }
    }
}

fn write_config(config: &Config) {
    let dirs = ui::get_dirs();
    let file_path = dirs.config_dir.join("config.json");
    if let Ok(f) = std::fs::File::create(file_path)
        && let Err(e) = serde_json::to_writer_pretty(f, &config)
    {
        eprintln!("Error writing config: {}", e);
    }
}

// Migrate the earlier #1129 layout (rom_dir/recent_roms/favorites at the top
// level, before they moved under `ui`) into `ui`, so upgrading from that build
// doesn't silently drop them. Only fills empty `ui` fields, so main's `ui`
// layout is left untouched.
fn migrate_legacy_ui(config: &mut Config, raw: &[u8]) {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(raw) else {
        return;
    };
    if config.ui.rom_dir.as_os_str().is_empty()
        && let Some(s) = v.get("rom_dir").and_then(|x| x.as_str())
    {
        config.ui.rom_dir = s.into();
    }
    if config.ui.recent_roms.is_empty()
        && let Some(a) = v.get("recent_roms").and_then(|x| x.as_array())
    {
        config.ui.recent_roms = a
            .iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect();
    }
    if config.ui.favorites.is_empty()
        && let Some(a) = v.get("favorites").and_then(|x| x.as_array())
    {
        config.ui.favorites = a
            .iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect();
    }
}

impl Config {
    pub fn new() -> Config {
        let dirs = ui::get_dirs();
        let file_path = dirs.config_dir.join("config.json");
        let config_file = std::fs::read(file_path);
        let mut input_data: Option<Input> = None;
        if let Ok(config_file) = config_file {
            let result = serde_json::from_slice::<Config>(config_file.as_ref());
            if let Ok(mut result) = result {
                result.write_to_disk = true;
                migrate_legacy_ui(&mut result, config_file.as_ref());
                return result;
            }

            // try to restore input data from old config file
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&config_file)
                && let Some(data) = value.get("input")
                && let Ok(data) = serde_json::from_value::<Input>(data.clone())
            {
                input_data = Some(data);
            }
        }

        let mut input_profiles = std::collections::BTreeMap::new();
        input_profiles.insert(
            "default".to_string(),
            ui::input_profile::get_default_profile(),
        );
        Config {
            input: input_data.unwrap_or(Input {
                input_profile_binding: [
                    "default".to_string(),
                    "default".to_string(),
                    "default".to_string(),
                    "default".to_string(),
                ],
                controller_assignment: [None, None, None, None],
                input_profiles,
                controller_enabled: [true, false, false, false],
                transfer_pak: [false, false, false, false],
                gb_rom_path: [String::new(), String::new(), String::new(), String::new()],
                gb_ram_path: [String::new(), String::new(), String::new(), String::new()],
                emulate_vru: false,
            }),
            video: Video {
                upscale: 1,
                ssaa: false,
                integer_scaling: false,
                fullscreen: false,
                widescreen: false,
                vsync: true,
                crt: false,
            },
            emulation: Emulation {
                overclock: false,
                disable_expansion_pak: false,
                usb: false,
                rewind: false,
            },
            ui: Ui {
                theme: 0,
                rom_dir: std::path::PathBuf::new(),
                recent_roms: Vec::new(),
                favorites: Vec::new(),
            },
            write_to_disk: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_top_level_ui_fields() {
        let raw =
            br#"{"rom_dir":"/roms","recent_roms":["/roms/a.z64"],"favorites":["/roms/b.z64"]}"#;
        let mut cfg = Config::default();
        migrate_legacy_ui(&mut cfg, raw);
        assert_eq!(cfg.ui.rom_dir, std::path::PathBuf::from("/roms"));
        assert_eq!(cfg.ui.recent_roms, vec!["/roms/a.z64".to_string()]);
        assert_eq!(cfg.ui.favorites, vec!["/roms/b.z64".to_string()]);
    }

    #[test]
    fn leaves_main_ui_layout_untouched() {
        let raw = br#"{"ui":{"rom_dir":"/keep","recent_roms":[],"favorites":[]}}"#;
        let mut cfg = Config::default();
        cfg.ui.rom_dir = std::path::PathBuf::from("/keep");
        migrate_legacy_ui(&mut cfg, raw);
        assert_eq!(cfg.ui.rom_dir, std::path::PathBuf::from("/keep"));
    }
}
