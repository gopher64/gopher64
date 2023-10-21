use crate::ui;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InputProfile {
    pub keys: [(ui::input::InputType, usize); 14],
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub input_profiles: std::collections::HashMap<String, InputProfile>,
    pub input_profile_binding: [Option<String>; 4],
    pub controller_assignment: [Option<usize>; 4],
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub input: Input,
}

impl Config {
    pub fn new() -> Config {
        Config {
            input: Input {
                input_profile_binding: Default::default(),
                controller_assignment: [None; 4],
                input_profiles: std::collections::HashMap::new(),
            },
        }
    }
}
