pub mod audio;
pub mod input;
pub mod storage;
pub mod video;

pub struct Ui {
    pub save_type: Vec<storage::SaveTypes>,
    pub game_name: String,
    pub game_id: String,
    pub paths: storage::Paths,
    pub saves: storage::Saves,
    pub sdl_context: Option<sdl2::Sdl>,
    pub video_subsystem: Option<sdl2::VideoSubsystem>,
    pub audio_subsystem: Option<sdl2::AudioSubsystem>,
    pub window: Option<sdl2::video::Window>,
    pub audio_device: Option<sdl2::audio::AudioQueue<i16>>,
}

impl Ui {
    pub fn new() -> Ui {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();
        Ui {
            save_type: vec![],
            game_name: String::new(),
            game_id: String::new(),
            paths: storage::Paths {
                eep_file_path: std::path::PathBuf::new(),
                fla_file_path: std::path::PathBuf::new(),
                sra_file_path: std::path::PathBuf::new(),
                pak_file_path: std::path::PathBuf::new(),
            },
            saves: storage::Saves {
                eeprom: Vec::new(),
                sram: Vec::new(),
                flash: Vec::new(),
                mempak: Vec::new(),
            },
            sdl_context: Some(sdl_context),
            video_subsystem: Some(video_subsystem),
            audio_subsystem: Some(audio_subsystem),
            window: None,
            audio_device: None,
        }
    }
}
