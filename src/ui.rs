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

pub struct Audio {
    pub audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub event_audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub audio_device: u32,
    pub event_audio: audio::EventAudio,
}

pub struct Input {
    pub num_joysticks: i32,
    pub joysticks: *mut sdl3_sys::joystick::SDL_JoystickID,
    pub keyboard_state: *const bool,
    pub controllers: [input::Controllers; 4],
}

pub struct Storage {
    pub save_type: Vec<storage::SaveTypes>,
    pub paths: storage::Paths,
    pub saves: storage::Saves,
}

pub struct Video {
    pub window: *mut sdl3_sys::video::SDL_Window,
    pub fullscreen: bool,
}

pub struct Ui {
    pub dirs: Dirs,
    pub config: config::Config,
    pub game_id: String,
    pub game_hash: String,
    pub with_sdl: bool,
    pub audio: Audio,
    pub input: Input,
    pub storage: Storage,
    pub video: Video,
}

impl Drop for Ui {
    fn drop(&mut self) {
        if self.with_sdl {
            unsafe {
                sdl3_sys::stdinc::SDL_free(self.input.joysticks as *mut std::ffi::c_void);
                sdl3_sys::init::SDL_Quit();
            }
        }
    }
}

pub fn sdl_init(flag: sdl3_sys::init::SDL_InitFlags) {
    unsafe {
        let init = sdl3_sys::init::SDL_WasInit(0);
        if init & flag == 0 && !sdl3_sys::init::SDL_InitSubSystem(flag) {
            let err = std::ffi::CStr::from_ptr(sdl3_sys::error::SDL_GetError())
                .to_str()
                .unwrap();
            panic!("Could not initialize SDL subsystem: {}, {}", flag, err);
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
    fn construct_ui(num_joysticks: i32, joysticks: *mut u32, with_sdl: bool) -> Ui {
        let dirs = get_dirs();

        Ui {
            input: Input {
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
                num_joysticks,
                joysticks,
            },
            storage: Storage {
                save_type: vec![],
                paths: storage::Paths {
                    eep_file_path: std::path::PathBuf::new(),
                    fla_file_path: std::path::PathBuf::new(),
                    sra_file_path: std::path::PathBuf::new(),
                    pak_file_path: std::path::PathBuf::new(),
                    sdcard_file_path: std::path::PathBuf::new(),
                    romsave_file_path: std::path::PathBuf::new(),
                    savestate_file_path: std::path::PathBuf::new(),
                },
                saves: storage::Saves {
                    write_to_disk: true,
                    eeprom: storage::Save {
                        data: Vec::new(),
                        written: false,
                    },
                    sram: storage::Save {
                        data: Vec::new(),
                        written: false,
                    },
                    flash: storage::Save {
                        data: Vec::new(),
                        written: false,
                    },
                    mempak: storage::Save {
                        data: Vec::new(),
                        written: false,
                    },
                    sdcard: storage::Save {
                        data: Vec::new(),
                        written: false,
                    },
                    romsave: storage::RomSave {
                        data: std::collections::HashMap::new(),
                        written: false,
                    },
                },
            },
            config: config::Config::new(),
            game_id: String::new(),
            game_hash: String::new(),
            audio: Audio {
                event_audio: audio::EventAudio {
                    mempak: include_bytes!("../data/mempak.wav").to_vec(),
                    rumblepak: include_bytes!("../data/rumblepak.wav").to_vec(),
                    transferpak: include_bytes!("../data/transferpak.wav").to_vec(),
                    netplay_desync: include_bytes!("../data/netplay_desync.wav").to_vec(),
                    netplay_lost_connection: include_bytes!("../data/netplay_lost_connection.wav")
                        .to_vec(),
                    netplay_disconnected: [
                        include_bytes!("../data/netplay_p1_disconnected.wav").to_vec(),
                        include_bytes!("../data/netplay_p2_disconnected.wav").to_vec(),
                        include_bytes!("../data/netplay_p3_disconnected.wav").to_vec(),
                        include_bytes!("../data/netplay_p4_disconnected.wav").to_vec(),
                    ],
                },
                audio_stream: std::ptr::null_mut(),
                event_audio_stream: std::ptr::null_mut(),
                audio_device: 0,
            },
            video: Video {
                window: std::ptr::null_mut(),
                fullscreen: false,
            },
            dirs,
            with_sdl,
        }
    }

    pub fn default() -> Ui {
        Self::construct_ui(0, std::ptr::null_mut(), false)
    }

    pub fn new() -> Ui {
        sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);
        let mut num_joysticks = 0;
        let joysticks = unsafe { sdl3_sys::joystick::SDL_GetJoysticks(&mut num_joysticks) };
        if joysticks.is_null() {
            panic!("Could not get joystick list");
        }
        Self::construct_ui(num_joysticks, joysticks, true)
    }
}
