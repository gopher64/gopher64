pub mod audio;
pub mod input;
pub mod storage;
pub mod video;

pub struct Ui {
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
        let window = video_subsystem
            .window("gopher64", 640, 480)
            .position_centered()
            .vulkan()
            .build()
            .unwrap();
        Ui {
            saves: storage::Saves { eeprom: Vec::new() },
            sdl_context: Some(sdl_context),
            video_subsystem: Some(video_subsystem),
            audio_subsystem: Some(audio_subsystem),
            window: Some(window),
            audio_device: None,
        }
    }
}
