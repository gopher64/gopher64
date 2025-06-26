use crate::ui;

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputKeyButton {
    pub enabled: bool,
    pub id: i32,
}

#[derive(PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputControllerAxis {
    pub enabled: bool,
    pub id: i32,
    pub axis: i16,
}

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputJoystickHat {
    pub enabled: bool,
    pub id: i32,
    pub direction: u8,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InputProfile {
    pub keys: [InputKeyButton; ui::input::PROFILE_SIZE],
    pub controller_buttons: [InputKeyButton; ui::input::PROFILE_SIZE],
    pub controller_axis: [InputControllerAxis; ui::input::PROFILE_SIZE],
    pub joystick_buttons: [InputKeyButton; ui::input::PROFILE_SIZE],
    pub joystick_hat: [InputJoystickHat; ui::input::PROFILE_SIZE],
    pub joystick_axis: [InputControllerAxis; ui::input::PROFILE_SIZE],
    pub dinput: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub input_profiles: std::collections::HashMap<String, InputProfile>,
    pub input_profile_binding: [String; 4],
    pub controller_assignment: [Option<String>; 4],
    pub controller_enabled: [bool; 4],
    pub transfer_pak: [bool; 4],
    pub emulate_vru: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Video {
    pub upscale: u32,
    pub integer_scaling: bool,
    pub fullscreen: bool,
    pub widescreen: bool,
    pub crt: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Emulation {
    pub overclock: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub input: Input,
    pub video: Video,
    pub emulation: Emulation,
}

impl Drop for Config {
    fn drop(&mut self) {
        write_config(self);
    }
}

fn write_config(config: &Config) {
    let dirs = ui::get_dirs();
    let file_path = dirs.config_dir.join("config.json");
    let f = std::fs::File::create(file_path).unwrap();
    serde_json::to_writer_pretty(f, &config).unwrap();
}

impl Config {
    pub fn new() -> Config {
        let dirs = ui::get_dirs();
        let file_path = dirs.config_dir.join("config.json");
        let config_file = std::fs::read(file_path);
        if let Ok(config_file) = config_file {
            let result = serde_json::from_slice(config_file.as_ref());
            if let Ok(result) = result {
                return result;
            }
        }
        let mut input_profiles = std::collections::HashMap::new();
        input_profiles.insert("default".to_string(), ui::input::get_default_profile());
        Config {
            input: Input {
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
                emulate_vru: false,
            },
            video: Video {
                upscale: 1,
                integer_scaling: false,
                fullscreen: false,
                widescreen: false,
                crt: false,
            },
            emulation: Emulation { overclock: false },
        }
    }
}
