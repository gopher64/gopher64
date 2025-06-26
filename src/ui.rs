pub mod audio;
pub mod cheats;
pub mod config;
pub mod gui;
pub mod input;
pub mod netplay;
pub mod storage;
pub mod usb;
pub mod video;

pub struct Dirs {
    pub config_dir: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
}

pub struct Audio {
    pub audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub vru_audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub gain: f32,
}

pub struct Input {
    pub joysticks: Vec<sdl3_sys::joystick::SDL_JoystickID>,
    pub keyboard_state: *const bool,
    pub controllers: [input::Controllers; 4],
}

pub struct Storage {
    pub save_type: Vec<storage::SaveTypes>,
    pub paths: storage::Paths,
    pub saves: storage::Saves,
    pub save_state_slot: u32,
}

pub struct Video {
    pub window: *mut sdl3_sys::video::SDL_Window,
    pub fullscreen: bool,
}

pub struct Usb {
    pub usb_tx: Option<tokio::sync::broadcast::Sender<usb::UsbData>>,
    pub cart_rx: Option<tokio::sync::broadcast::Receiver<usb::UsbData>>,
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
    pub usb: Usb,
}

impl Drop for Ui {
    fn drop(&mut self) {
        if self.with_sdl {
            unsafe {
                sdl3_ttf_sys::ttf::TTF_Quit();
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
            panic!("Could not initialize SDL subsystem: {flag}, {err}");
        }
    }
}

pub fn ttf_init() {
    unsafe {
        if !sdl3_ttf_sys::ttf::TTF_Init() {
            panic!("Could not initialize TTF");
        }
    }
}

pub fn get_dirs() -> Dirs {
    let exe_path = std::env::current_exe().unwrap();
    let portable_dir = exe_path.parent();
    let portable = portable_dir.unwrap().join("portable.txt").exists();
    let config_dir;
    let data_dir;
    if portable {
        config_dir = portable_dir.unwrap().join("portable_data").join("config");
        data_dir = portable_dir.unwrap().join("portable_data").join("data");
    } else {
        config_dir = dirs::config_dir().unwrap().join("gopher64");
        data_dir = dirs::data_dir().unwrap().join("gopher64");
    };

    Dirs {
        config_dir,
        data_dir,
    }
}

impl Ui {
    fn construct_ui(joysticks: Vec<u32>, with_sdl: bool) -> Ui {
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
                joysticks,
            },
            storage: Storage {
                save_state_slot: 0,
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
                audio_stream: std::ptr::null_mut(),
                gain: 1.0,
                vru_audio_stream: std::ptr::null_mut(),
            },
            video: Video {
                window: std::ptr::null_mut(),
                fullscreen: false,
            },
            usb: Usb {
                usb_tx: None,
                cart_rx: None,
            },
            dirs,
            with_sdl,
        }
    }

    pub fn default() -> Ui {
        Self::construct_ui(vec![], false)
    }

    pub fn new() -> Ui {
        sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);
        let mut num_joysticks = 0;
        let joysticks = unsafe { sdl3_sys::joystick::SDL_GetJoysticks(&mut num_joysticks) };
        if joysticks.is_null() {
            panic!("Could not get joystick list");
        }
        let mut joystick_vec = vec![];
        for i in 0..num_joysticks {
            joystick_vec.push(unsafe { *joysticks.add(i as usize) });
        }
        unsafe { sdl3_sys::stdinc::SDL_free(joysticks as *mut std::ffi::c_void) }
        Self::construct_ui(joystick_vec, true)
    }
}
