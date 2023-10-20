#[derive(serde::Serialize, serde::Deserialize)]
pub struct InputProfile {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    input_profiles: std::collections::HashMap<String, InputProfile>,
    input_profile_binding: [Option<String>; 4],
    controller_assignment: [Option<usize>; 4],
}

impl Config {
    pub fn new() -> Config {
        Config {
            input_profile_binding: Default::default(),
            controller_assignment: [None; 4],
            input_profiles: std::collections::HashMap::new(),
        }
    }
}
