use crate::device;
use crate::ui;

static ADJUST_LOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn init(device: &mut device::Device) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_AUDIO);
    init_game_audio(device);
}

unsafe extern "C" fn audio_callback(
    _userdata: *mut std::ffi::c_void,
    audio_stream: *mut sdl3_sys::audio::SDL_AudioStream,
    additional_amount: i32,
    _total_amount: i32,
) {
    if additional_amount > 0 {
        unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                audio_stream,
                vec![0; additional_amount as usize].as_ptr() as *const std::ffi::c_void,
                additional_amount,
            )
        };

        adjust_audio_frequency(audio_stream, -0.0005);
    }
}

pub fn init_game_audio(device: &mut device::Device) {
    let game_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: device.ai.freq as i32,
        channels: 2,
    };

    device.ui.audio.audio_stream = unsafe {
        sdl3_sys::audio::SDL_OpenAudioDeviceStream(
            sdl3_sys::audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
            &game_audio_spec,
            None,
            std::ptr::null_mut(),
        )
    };
    if device.ui.audio.audio_stream.is_null() {
        panic!("Could not create audio stream");
    }
    if !unsafe {
        sdl3_sys::audio::SDL_SetAudioStreamGain(device.ui.audio.audio_stream, device.ui.audio.gain)
            && sdl3_sys::audio::SDL_SetAudioStreamGetCallback(
                device.ui.audio.audio_stream,
                Some(audio_callback),
                std::ptr::null_mut(),
            )
    } {
        panic!("Could not initialize audio stream");
    }
    if device.ai.regs[device::ai::AI_STATUS_REG as usize] & device::ai::AI_STATUS_BUSY != 0 {
        resume_game_audio(&mut device.ui);
    } else {
        pause_game_audio(&mut device.ui);
    }
}

pub fn close(ui: &mut ui::Ui) {
    close_game_audio(ui);
}

pub fn resume_game_audio(ui: &mut ui::Ui) {
    unsafe {
        sdl3_sys::audio::SDL_ResumeAudioStreamDevice(ui.audio.audio_stream);
    }
}

pub fn pause_game_audio(ui: &mut ui::Ui) {
    unsafe {
        sdl3_sys::audio::SDL_PauseAudioStreamDevice(ui.audio.audio_stream);
    }
}

pub fn close_game_audio(ui: &mut ui::Ui) {
    unsafe {
        if !ui.audio.audio_stream.is_null() {
            sdl3_sys::audio::SDL_DestroyAudioStream(ui.audio.audio_stream);
            ui.audio.audio_stream = std::ptr::null_mut();
        }
    }
}

pub fn lower_audio_volume(ui: &mut ui::Ui) {
    unsafe {
        ui.audio.gain = sdl3_sys::audio::SDL_GetAudioStreamGain(ui.audio.audio_stream) - 0.05;
        if ui.audio.gain < 0.0 {
            ui.audio.gain = 0.0;
        }
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.audio_stream, ui.audio.gain);
    }
    ui::video::onscreen_message(
        &format!("Audio volume: {:.0}%", ui.audio.gain * 100.0),
        ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
    );
}

pub fn raise_audio_volume(ui: &mut ui::Ui) {
    unsafe {
        ui.audio.gain = sdl3_sys::audio::SDL_GetAudioStreamGain(ui.audio.audio_stream) + 0.05;
        if ui.audio.gain > 2.0 {
            ui.audio.gain = 2.0;
        }
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.audio_stream, ui.audio.gain);
    }
    ui::video::onscreen_message(
        &format!("Audio volume: {:.0}%", ui.audio.gain * 100.0),
        ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
    );
}

unsafe extern "C" fn lock_callback(
    _userdata: *mut std::ffi::c_void,
    _id: sdl3_sys::timer::SDL_TimerID,
    _interval: u32,
) -> u32 {
    ADJUST_LOCKED.store(false, std::sync::atomic::Ordering::Relaxed);
    0
}

fn adjust_audio_frequency(audio_stream: *mut sdl3_sys::audio::SDL_AudioStream, frequency: f32) {
    if ADJUST_LOCKED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    ADJUST_LOCKED.store(true, std::sync::atomic::Ordering::Relaxed);

    unsafe {
        sdl3_sys::timer::SDL_AddTimer(1000, Some(lock_callback), std::ptr::null_mut());
        let current_ratio = sdl3_sys::everything::SDL_GetAudioStreamFrequencyRatio(audio_stream);
        sdl3_sys::everything::SDL_SetAudioStreamFrequencyRatio(
            audio_stream,
            (current_ratio + frequency).clamp(0.995, 1.005),
        );

        /*
        println!(
            "Adjusted audio frequency ratio to {}",
            sdl3_sys::everything::SDL_GetAudioStreamFrequencyRatio(audio_stream)
        );
        */
    }
}

pub fn play_audio(device: &device::Device, dram_addr: usize, length: u64) {
    if device.ui.audio.audio_stream.is_null() {
        return;
    }

    let mut primary_buffer: Vec<i16> = vec![0; length as usize / 2];
    let mut i = 0;
    while i < length as usize / 2 {
        // Left channel
        primary_buffer[i] = *device.rdram.mem.get(dram_addr + (i * 2) + 2).unwrap_or(&0) as i16
            | ((*device.rdram.mem.get(dram_addr + (i * 2) + 3).unwrap_or(&0) as i16) << 8);

        // Right channel
        primary_buffer[i + 1] = *device.rdram.mem.get(dram_addr + (i * 2)).unwrap_or(&0) as i16
            | ((*device.rdram.mem.get(dram_addr + (i * 2) + 1).unwrap_or(&0) as i16) << 8);
        i += 2;
    }

    let audio_queued =
        unsafe { sdl3_sys::audio::SDL_GetAudioStreamQueued(device.ui.audio.audio_stream) } as f64;
    let samples_per_frame = device.ai.freq as f64 * device.vi.frame_time * 4.0;
    let max_latency = samples_per_frame * 8.0;
    if audio_queued < max_latency {
        unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio.audio_stream,
                primary_buffer.as_ptr() as *const std::ffi::c_void,
                primary_buffer.len() as i32 * 2,
            )
        };
    } else if device.vi.enable_speed_limiter {
        adjust_audio_frequency(device.ui.audio.audio_stream, 0.0005);
    }
}
