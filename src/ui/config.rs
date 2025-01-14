use crate::ui;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InputProfile {
    pub keys: [(bool, i32); 18],
    pub controller_buttons: [(bool, i32); 14],
    pub controller_axis: [(bool, i32, i16); 18],
    pub joystick_buttons: [(bool, u32); 14],
    pub joystick_hat: [(bool, u32, u8); 14],
    pub joystick_axis: [(bool, u32, i16); 18],
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub input_profiles: std::collections::HashMap<String, InputProfile>,
    pub input_profile_binding: [String; 4],
    pub controller_assignment: [Option<String>; 4],
    pub controller_enabled: [bool; 4],
    pub emulate_vru: bool,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Video {
    pub upscale: bool,
    pub integer_scaling: bool,
    pub fullscreen: bool,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub input: Input,
    pub video: Video,
}

impl Config {
    pub fn new() -> Config {
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
                emulate_vru: false,
            },
            video: Video {
                upscale: false,
                integer_scaling: false,
                fullscreen: false,
            },
        }
    }
}
