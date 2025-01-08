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
    pub sdl_context: Option<sdl2::Sdl>,
    pub video_subsystem: Option<sdl2::VideoSubsystem>,
    pub audio_subsystem: Option<sdl2::AudioSubsystem>,
    pub joystick_subsystem: Option<sdl2::JoystickSubsystem>,
    #[allow(dead_code)]
    pub controller_subsystem: Option<sdl2::GameControllerSubsystem>,
    pub window: Option<sdl2::video::Window>,
    pub audio_device: Option<sdl2::audio::AudioQueue<i16>>,
}

impl Drop for Ui {
    fn drop(&mut self) {
        write_config(self);
    }
}

fn write_config(ui: &Ui) {
    let f = std::fs::File::create(ui.config_file_path.clone()).unwrap();
    serde_json::to_writer_pretty(f, &ui.config).unwrap();
}

impl Ui {
    pub fn new() -> Ui {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();
        let joystick_subsystem = sdl_context.joystick().unwrap();
        let controller_subsystem = sdl_context.game_controller().unwrap();

        let config_file_path = dirs::config_dir()
            .unwrap()
            .join("gopher64")
            .join("config.json");
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
                    game_controller: None,
                    joystick: None,
                },
                input::Controllers {
                    game_controller: None,
                    joystick: None,
                },
                input::Controllers {
                    game_controller: None,
                    joystick: None,
                },
                input::Controllers {
                    game_controller: None,
                    joystick: None,
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
            sdl_context: Some(sdl_context),
            video_subsystem: Some(video_subsystem),
            audio_subsystem: Some(audio_subsystem),
            joystick_subsystem: Some(joystick_subsystem),
            controller_subsystem: Some(controller_subsystem),
            window: None,
            audio_device: None,
        }
    }
}
