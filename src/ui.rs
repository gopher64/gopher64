pub mod audio;
pub mod config;
pub mod gui;
pub mod input;
pub mod storage;
pub mod video;
pub mod vru;

pub struct Dirs {
    pub config_dir: std::path::PathBuf,
    pub cache_dir: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
}

pub struct Ui {
    pub dirs: Dirs,
    pub keyboard_state: *const bool,
    pub controllers: [input::Controllers; 4],
    pub config_file_path: std::path::PathBuf,
    pub config: config::Config,
    pub save_type: Vec<storage::SaveTypes>,
    pub game_id: String,
    pub game_hash: String,
    pub paths: storage::Paths,
    pub saves: storage::Saves,
    pub pak_audio: audio::PakAudio,
    pub window: *mut sdl3_sys::video::SDL_Window,
    pub audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub pak_audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub audio_freq: f64,
    pub audio_device: u32,
    pub num_joysticks: i32,
    pub joysticks: *mut sdl3_sys::joystick::SDL_JoystickID,
}

impl Drop for Ui {
    fn drop(&mut self) {
        unsafe {
            sdl3_sys::stdinc::SDL_free(self.joysticks as *mut std::ffi::c_void);
            sdl3_sys::init::SDL_Quit();
        }
        write_config(self);
    }
}

fn write_config(ui: &Ui) {
    let f = std::fs::File::create(ui.config_file_path.clone()).unwrap();
    serde_json::to_writer_pretty(f, &ui.config).unwrap();
}

pub fn sdl_init(flag: sdl3_sys::init::SDL_InitFlags) {
    unsafe {
        let init = sdl3_sys::init::SDL_WasInit(0);
        if init & flag == 0 && !sdl3_sys::init::SDL_InitSubSystem(flag) {
            panic!("Could not initialize SDL subsystem: {}", flag);
        }
    }
}

pub fn get_dirs() -> Dirs {
    let exe_path = std::env::current_exe().unwrap();
    let portable_dir = exe_path.parent();
    let portable = portable_dir.unwrap().join("portable.txt").exists();
    let config_dir;
    let cache_dir;
    let data_dir;
    if portable {
        config_dir = portable_dir.unwrap().join("portable_data").join("config");
        cache_dir = portable_dir.unwrap().join("portable_data").join("cache");
        data_dir = portable_dir.unwrap().join("portable_data").join("data");
    } else {
        config_dir = dirs::config_dir().unwrap().join("gopher64");
        cache_dir = dirs::cache_dir().unwrap().join("gopher64");
        data_dir = dirs::data_dir().unwrap().join("gopher64");
    };

    Dirs {
        config_dir,
        cache_dir,
        data_dir,
    }
}

impl Ui {
    pub fn new() -> Ui {
        sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);
        let mut num_joysticks = 0;
        let joysticks = unsafe { sdl3_sys::joystick::SDL_GetJoysticks(&mut num_joysticks) };
        if joysticks.is_null() {
            panic!("Could not get joystick list");
        }

        let dirs = get_dirs();
        let config_file_path = dirs.config_dir.join("config.json");
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
                    rumble: false,
                },
                input::Controllers {
                    game_controller: std::ptr::null_mut(),
                    joystick: std::ptr::null_mut(),
                    rumble: false,
                },
                input::Controllers {
                    game_controller: std::ptr::null_mut(),
                    joystick: std::ptr::null_mut(),
                    rumble: false,
                },
                input::Controllers {
                    game_controller: std::ptr::null_mut(),
                    joystick: std::ptr::null_mut(),
                    rumble: false,
                },
            ],
            keyboard_state: std::ptr::null_mut(),
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
            pak_audio: audio::PakAudio {
                mempak: include_bytes!("../data/mempak.wav").to_vec(),
                rumblepak: include_bytes!("../data/rumblepak.wav").to_vec(),
            },
            window: std::ptr::null_mut(),
            audio_stream: std::ptr::null_mut(),
            pak_audio_stream: std::ptr::null_mut(),
            audio_freq: 0.0,
            audio_device: 0,
            num_joysticks,
            joysticks,
            dirs,
        }
    }
}
