pub mod audio;
pub mod config;
pub mod gui;
pub mod input;
pub mod storage;
pub mod video;
pub mod vru;

pub struct Ui {
    pub controllers: [input::Controllers; 4],
    pub config_file_path: std::path::PathBuf,
    pub config: config::Config,
    pub save_type: Vec<storage::SaveTypes>,
    pub game_id: String,
    pub game_hash: String,
    pub paths: storage::Paths,
    pub saves: storage::Saves,
    pub pak_audio: Option<audio::PakAudio>,
    pub audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub audio_spec: Option<sdl3_sys::audio::SDL_AudioSpec>,
}

impl Drop for Ui {
    fn drop(&mut self) {
        unsafe {
            sdl3_sys::init::SDL_Quit();
        }
        write_config(self);
    }
}

fn write_config(ui: &Ui) {
    let f = std::fs::File::create(ui.config_file_path.clone()).unwrap();
    serde_json::to_writer_pretty(f, &ui.config).unwrap();
}

impl Ui {
    pub fn new(config_dir: std::path::PathBuf) -> Ui {
        let config_file_path = config_dir.join("config.json");
        let config_file = std::fs::read(config_file_path.clone());
        let mut config_map = config::Config::new();
        if config_file.is_ok() {
            let result = serde_json::from_slice(config_file.unwrap().as_ref());
            if result.is_ok() {
                config_map = result.unwrap();
            }
        }

        Ui {
            controllers: [
                input::Controllers {
                    game_controller: std::ptr::null_mut(),
                    joystick: std::ptr::null_mut(),
                    keyboard_state: std::ptr::null(),
                    rumble: false,
                },
                input::Controllers {
                    game_controller: std::ptr::null_mut(),
                    joystick: std::ptr::null_mut(),
                    keyboard_state: std::ptr::null(),
                    rumble: false,
                },
                input::Controllers {
                    game_controller: std::ptr::null_mut(),
                    joystick: std::ptr::null_mut(),
                    keyboard_state: std::ptr::null(),
                    rumble: false,
                },
                input::Controllers {
                    game_controller: std::ptr::null_mut(),
                    joystick: std::ptr::null_mut(),
                    keyboard_state: std::ptr::null(),
                    rumble: false,
                },
            ],
            config_file_path,
            config: config_map,
            save_type: vec![],
            game_id: String::new(),
            game_hash: String::new(),
            paths: storage::Paths {
                eep_file_path: std::path::PathBuf::new(),
                fla_file_path: std::path::PathBuf::new(),
                sra_file_path: std::path::PathBuf::new(),
                pak_file_path: std::path::PathBuf::new(),
                romsave_file_path: std::path::PathBuf::new(),
            },
            saves: storage::Saves {
                eeprom: (Vec::new(), false),
                sram: (Vec::new(), false),
                flash: (Vec::new(), false),
                mempak: (Vec::new(), false),
                romsave: (serde_json::Map::new(), false),
            },
            pak_audio: None,
            audio_stream: std::ptr::null_mut(),
            audio_spec: None,
        }
    }
}
