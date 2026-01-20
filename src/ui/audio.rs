use crate::device;
use crate::ui;

pub fn init(ui: &mut ui::Ui, frequency: u64) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_AUDIO);
    init_game_audio(ui, frequency);
}

pub fn init_game_audio(ui: &mut ui::Ui, frequency: u64) {
    let game_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: frequency as i32,
        channels: 2,
    };

    ui.audio.audio_stream = unsafe {
        sdl3_sys::audio::SDL_OpenAudioDeviceStream(
            sdl3_sys::audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
            &game_audio_spec,
            None,
            std::ptr::null_mut(),
        )
    };
    if ui.audio.audio_stream.is_null() {
        panic!("Could not create audio stream");
    }
    if !unsafe {
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.audio_stream, ui.audio.gain)
            && sdl3_sys::audio::SDL_ResumeAudioStreamDevice(ui.audio.audio_stream)
    } {
        panic!("Could not resume audio stream");
    }
}

pub fn close(ui: &mut ui::Ui) {
    close_game_audio(ui);
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
}

pub fn raise_audio_volume(ui: &mut ui::Ui) {
    unsafe {
        ui.audio.gain = sdl3_sys::audio::SDL_GetAudioStreamGain(ui.audio.audio_stream) + 0.05;
        if ui.audio.gain > 2.0 {
            ui.audio.gain = 2.0;
        }
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.audio_stream, ui.audio.gain);
    }
}

fn adjust_audio_frequency(device: &device::Device, frequency: f32) {
    if !device.vi.enable_speed_limiter {
        return;
    }

    unsafe {
        let current_ratio =
            sdl3_sys::everything::SDL_GetAudioStreamFrequencyRatio(device.ui.audio.audio_stream);
        sdl3_sys::everything::SDL_SetAudioStreamFrequencyRatio(
            device.ui.audio.audio_stream,
            (current_ratio + frequency).clamp(0.99, 1.01),
        );
        /*
        println!(
            "Adjusted audio frequency ratio to {}",
            sdl3_sys::everything::SDL_GetAudioStreamFrequencyRatio(device.ui.audio.audio_stream)
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
    let max_latency = samples_per_frame * 10.0;

    if audio_queued < samples_per_frame {
        let silence_buffer: Vec<u8> = vec![0; samples_per_frame as usize & !3];
        if !unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio.audio_stream,
                silence_buffer.as_ptr() as *const std::ffi::c_void,
                silence_buffer.len() as i32,
            )
        } {
            panic!("Could not play audio");
        }
        /*
        println!(
            "Audio underrun: queued {} samples, expected at least {} samples",
            audio_queued, samples_per_frame
        );
        */
        adjust_audio_frequency(device, -0.0005);
    }

    if audio_queued < max_latency {
        if !unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio.audio_stream,
                primary_buffer.as_ptr() as *const std::ffi::c_void,
                primary_buffer.len() as i32 * 2,
            )
        } {
            panic!("Could not play audio");
        }
    } else {
        /*
        println!(
            "Audio overrun: queued {} samples, expected at most {} samples",
            audio_queued, max_latency
        );
        */
        adjust_audio_frequency(device, 0.0005);
    }
}
