#[cfg(target_os = "android")]
pub mod android;
pub mod audio;
#[cfg(feature = "gui")]
pub mod cheats;
pub mod config;
#[cfg(feature = "gui")]
pub mod gui;
pub mod input;
#[cfg(feature = "gui")]
pub mod netplay;
#[cfg(feature = "gui")]
pub mod retroachievements;
pub mod storage;
pub mod usb;
pub mod video;
#[cfg(all(feature = "gui", not(target_os = "android")))]
pub mod vru;
use clap::Parser;

pub static WEB_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("GIT_DESCRIBE")
        ))
        .build()
        .unwrap()
});

/// N64 emulator
#[derive(Parser, Debug)]
#[command(author, version=env!("GIT_DESCRIBE"), about, long_about = None, arg_required_else_help = if cfg!(feature = "gui") { false } else { true })]
pub struct Args {
    pub game: Option<String>,
    #[arg(short, long)]
    pub fullscreen: bool,
    #[arg(long)]
    pub overclock: Option<bool>,
    #[arg(long)]
    pub disable_expansion_pak: Option<bool>,
    #[arg(long, value_name = "CHEATS_FILE", hide = true)]
    pub cheats: Option<String>,
    #[arg(long, value_name = "NETPLAY_PEER_ADDR", hide = true)]
    pub netplay_peer_addr: Option<String>,
    #[arg(long, value_name = "NETPLAY_PLAYER_NUMBER", hide = true)]
    pub netplay_player_number: Option<u8>,
    #[arg(
        long,
        value_name = "PROFILE_NAME",
        help = "Create a new input profile (keyboard/gamepad mappings)"
    )]
    pub configure_input_profile: Option<String>,
    #[arg(long, help = "Use DirectInput when configuring a new input profile")]
    pub use_dinput: bool,
    #[arg(
        long,
        value_name = "DEADZONE_PERCENTAGE",
        help = "Used along with --configure-input-profile to set the deadzone for analog sticks"
    )]
    pub deadzone: Option<i32>,
    #[arg(
        long,
        value_name = "PROFILE_NAME",
        help = "Must also specify --port. Used to bind a previously created profile to a port"
    )]
    pub bind_input_profile: Option<String>,
    #[arg(
        long,
        help = "Lists connected controllers which can be used in --assign-controller"
    )]
    pub list_controllers: bool,
    #[arg(
        long,
        value_name = "CONTROLLER_NUMBER",
        help = "Must also specify --port. Used to assign a controller listed in --list-controllers to a port"
    )]
    pub assign_controller: Option<i32>,
    #[arg(
        long,
        value_name = "PORT",
        help = "Valid values: 1-4. To be used alongside --bind-input-profile and --assign-controller"
    )]
    pub port: Option<usize>,
    #[arg(
        long,
        help = "Clear all input profile bindings and controller assignments"
    )]
    pub clear_input_bindings: bool,
    #[arg(
        long,
        value_name = "SLOT",
        help = "Load savestate from slot 0-9 when starting the game"
    )]
    pub load_state: Option<u32>,
    #[arg(
        long = "ra-username",
        value_name = "USERNAME",
        help = "Username for RetroAchievements"
    )]
    pub ra_username: Option<String>,
    #[arg(
        long = "ra-password",
        value_name = "PASSWORD",
        help = "Password for RetroAchievements"
    )]
    pub ra_password: Option<String>,
}

#[derive(Clone)]
pub struct Dirs {
    pub config_dir: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub cache_dir: std::path::PathBuf,
}

pub struct Audio {
    pub audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    pub gain: f32,
}

pub struct Input {
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
    pub fps_tx: tokio::sync::mpsc::Sender<bool>,
    pub fps_rx: Option<tokio::sync::mpsc::Receiver<bool>>,
    pub vis_tx: tokio::sync::mpsc::Sender<bool>,
    pub vis_rx: Option<tokio::sync::mpsc::Receiver<bool>>,
}

pub struct Usb {
    pub usb_tx: Option<tokio::sync::broadcast::Sender<usb::UsbData>>,
    pub cart_rx: Option<tokio::sync::broadcast::Receiver<usb::UsbData>>,
}

#[derive(Clone)]
pub struct GameSettings {
    pub overclock: bool,
    pub disable_expansion_pak: bool,
    pub cheats: std::collections::HashMap<String, Option<String>>,
    pub load_savestate_slot: Option<u32>,
}

pub struct Ui {
    pub dirs: Dirs,
    pub config: config::Config,
    pub game_id: String,
    pub game_hash: String,
    pub audio: Audio,
    pub input: Input,
    pub storage: Storage,
    pub video: Video,
    pub usb: Usb,
}

pub fn sdl_hints() {
    unsafe {
        let hint = std::ffi::CString::new("1").unwrap();
        sdl3_sys::everything::SDL_SetHint(
            sdl3_sys::everything::SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS,
            hint.as_ptr(),
        );
        sdl3_sys::everything::SDL_SetHint(
            sdl3_sys::everything::SDL_HINT_ANDROID_ALLOW_RECREATE_ACTIVITY,
            hint.as_ptr(),
        );
    }
}

pub fn sdl_init(flag: sdl3_sys::init::SDL_InitFlags) {
    unsafe {
        if sdl3_sys::init::SDL_WasInit(flag) == 0 && !sdl3_sys::init::SDL_InitSubSystem(flag) {
            let err = sdl3_sys::error::SDL_GetError();
            panic!(
                "Could not initialize SDL subsystem: {}, {}",
                u32::from(flag),
                if err.is_null() {
                    "Unknown error"
                } else {
                    std::ffi::CStr::from_ptr(err).to_str().unwrap()
                }
            );
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

pub fn sdl_close() {
    unsafe {
        sdl3_ttf_sys::ttf::TTF_Quit();
        sdl3_sys::init::SDL_Quit();
    }
}

pub fn get_dirs() -> Dirs {
    #[cfg(target_os = "android")]
    return android::get_dirs();

    #[cfg(not(target_os = "android"))]
    {
        let exe_path = std::env::current_exe().unwrap();
        let portable_dir = exe_path.parent();
        let portable = portable_dir.unwrap().join("portable.txt").exists();
        if portable {
            Dirs {
                config_dir: portable_dir.unwrap().join("portable_data").join("config"),
                data_dir: portable_dir.unwrap().join("portable_data").join("data"),
                cache_dir: portable_dir.unwrap().join("portable_data").join("cache"),
            }
        } else {
            Dirs {
                config_dir: dirs::config_dir().unwrap().join("gopher64"),
                data_dir: dirs::data_dir().unwrap().join("gopher64"),
                cache_dir: dirs::cache_dir().unwrap().join("gopher64"),
            }
        }
    }
}

impl Ui {
    pub fn new() -> Ui {
        let dirs = get_dirs();

        let (fps_tx, fps_rx) = tokio::sync::mpsc::channel(1000);
        let (vis_tx, vis_rx) = tokio::sync::mpsc::channel(1000);
        Ui {
            input: Input {
                controllers: [
                    input::Controllers {
                        game_controller: std::ptr::null_mut(),
                        joystick: std::ptr::null_mut(),
                        rumble: false,
                        guid: sdl3_sys::guid::SDL_GUID::default(),
                        last_key_state: 0,
                    },
                    input::Controllers {
                        game_controller: std::ptr::null_mut(),
                        joystick: std::ptr::null_mut(),
                        rumble: false,
                        guid: sdl3_sys::guid::SDL_GUID::default(),
                        last_key_state: 0,
                    },
                    input::Controllers {
                        game_controller: std::ptr::null_mut(),
                        joystick: std::ptr::null_mut(),
                        rumble: false,
                        guid: sdl3_sys::guid::SDL_GUID::default(),
                        last_key_state: 0,
                    },
                    input::Controllers {
                        game_controller: std::ptr::null_mut(),
                        joystick: std::ptr::null_mut(),
                        rumble: false,
                        guid: sdl3_sys::guid::SDL_GUID::default(),
                        last_key_state: 0,
                    },
                ],
                keyboard_state: std::ptr::null_mut(),
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
                        write_pending: false,
                    },
                    sram: storage::Save {
                        data: Vec::new(),
                        write_pending: false,
                    },
                    flash: storage::Save {
                        data: Vec::new(),
                        write_pending: false,
                    },
                    mempak: storage::Save {
                        data: Vec::new(),
                        write_pending: false,
                    },
                    sdcard: storage::Save {
                        data: Vec::new(),
                        write_pending: false,
                    },
                    romsave: storage::RomSave {
                        data: std::collections::HashMap::new(),
                        write_pending: false,
                    },
                },
            },
            config: config::Config::new(),
            game_id: String::new(),
            game_hash: String::new(),
            audio: Audio {
                audio_stream: std::ptr::null_mut(),
                gain: 1.0,
            },
            video: Video {
                window: std::ptr::null_mut(),
                fullscreen: false,
                fps_tx,
                fps_rx: Some(fps_rx),
                vis_tx,
                vis_rx: Some(vis_rx),
            },
            usb: Usb {
                usb_tx: None,
                cart_rx: None,
            },
            dirs,
        }
    }
}
