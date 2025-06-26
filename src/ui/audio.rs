use crate::device;
use crate::netplay;
use crate::ui;

pub struct EventAudio {
    pub mempak: Vec<u8>,
    pub rumblepak: Vec<u8>,
    pub transferpak: Vec<u8>,
    pub netplay_desync: Vec<u8>,
    pub netplay_lost_connection: Vec<u8>,
    pub netplay_disconnected: [Vec<u8>; 4],
    pub cheats_enabled: Vec<u8>,
}

pub fn init(ui: &mut ui::Ui, frequency: u64) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_AUDIO);

    let device_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: 48000,
        channels: 2,
    };
    ui.audio.audio_device = unsafe {
        sdl3_sys::audio::SDL_OpenAudioDevice(
            sdl3_sys::audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
            &device_audio_spec,
        )
    };
    if ui.audio.audio_device == 0 {
        panic!("Could not open audio device");
    }

    if !unsafe {
        sdl3_sys::audio::SDL_GetAudioDeviceFormat(
            ui.audio.audio_device,
            &mut ui.audio.audio_device_spec,
            std::ptr::null_mut(),
        )
    } {
        panic!("Could not get audio device format");
    }

    let wav_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: 24000,
        channels: 1,
    };

    ui.audio.event_audio_stream = unsafe {
        sdl3_sys::audio::SDL_CreateAudioStream(&wav_audio_spec, &ui.audio.audio_device_spec)
    };
    if !unsafe {
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.event_audio_stream, ui.audio.gain)
            && sdl3_sys::audio::SDL_BindAudioStream(
                ui.audio.audio_device,
                ui.audio.event_audio_stream,
            )
    } {
        panic!("Could not bind audio stream");
    }

    if ui.config.input.emulate_vru {
        let vru_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
            format: sdl3_sys::audio::SDL_AUDIO_F32,
            freq: 16000,
            channels: 1,
        };
        ui.audio.vru_audio_stream = unsafe {
            sdl3_sys::audio::SDL_OpenAudioDeviceStream(
                sdl3_sys::audio::SDL_AUDIO_DEVICE_DEFAULT_RECORDING,
                &vru_audio_spec,
                None,
                std::ptr::null_mut(),
            )
        };
        if ui.audio.vru_audio_stream.is_null() {
            panic!("Could not open vru audio stream");
        }
        unsafe {
            sdl3_sys::audio::SDL_PauseAudioStreamDevice(ui.audio.vru_audio_stream);
        }
    }

    init_game_audio(ui, frequency);
}

pub fn init_game_audio(ui: &mut ui::Ui, frequency: u64) {
    let game_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: frequency as i32,
        channels: 2,
    };

    ui.audio.audio_stream = unsafe {
        sdl3_sys::audio::SDL_CreateAudioStream(&game_audio_spec, &ui.audio.audio_device_spec)
    };
    if ui.audio.audio_stream.is_null() {
        return;
    }
    if !unsafe {
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.audio_stream, ui.audio.gain)
            && sdl3_sys::audio::SDL_BindAudioStream(ui.audio.audio_device, ui.audio.audio_stream)
    } {
        panic!("Could not bind audio stream");
    }
}

pub fn close(ui: &mut ui::Ui) {
    close_game_audio(ui);
    unsafe {
        if !ui.audio.event_audio_stream.is_null() {
            sdl3_sys::audio::SDL_DestroyAudioStream(ui.audio.event_audio_stream);
            ui.audio.event_audio_stream = std::ptr::null_mut();
        }
        if ui.config.input.emulate_vru {
            sdl3_sys::audio::SDL_DestroyAudioStream(ui.audio.vru_audio_stream);
            ui.audio.vru_audio_stream = std::ptr::null_mut();
        }
        sdl3_sys::audio::SDL_CloseAudioDevice(ui.audio.audio_device);
        ui.audio.audio_device = 0;
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
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.event_audio_stream, ui.audio.gain);
    }
}

pub fn raise_audio_volume(ui: &mut ui::Ui) {
    unsafe {
        ui.audio.gain = sdl3_sys::audio::SDL_GetAudioStreamGain(ui.audio.audio_stream) + 0.05;
        if ui.audio.gain > 2.0 {
            ui.audio.gain = 2.0;
        }
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.audio_stream, ui.audio.gain);
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.event_audio_stream, ui.audio.gain);
    }
}

pub fn play_cheat_event(ui: &ui::Ui) {
    if ui.audio.event_audio_stream.is_null() {
        return;
    }
    let sound = &ui.audio.event_audio.cheats_enabled;
    if !unsafe {
        sdl3_sys::audio::SDL_PutAudioStreamData(
            ui.audio.event_audio_stream,
            sound.as_ptr() as *const std::ffi::c_void,
            sound.len() as i32,
        )
    } {
        panic!("Could not play audio");
    }
}

pub fn play_netplay_audio(ui: &ui::Ui, error: u32) {
    if ui.audio.event_audio_stream.is_null() {
        return;
    }
    let sound = match error {
        netplay::NETPLAY_ERROR_DESYNC => &ui.audio.event_audio.netplay_desync,
        netplay::NETPLAY_ERROR_LOST_CONNECTION => &ui.audio.event_audio.netplay_lost_connection,
        netplay::NETPLAY_ERROR_PLAYER_1_DISCONNECTED => {
            &ui.audio.event_audio.netplay_disconnected[0]
        }
        netplay::NETPLAY_ERROR_PLAYER_2_DISCONNECTED => {
            &ui.audio.event_audio.netplay_disconnected[1]
        }
        netplay::NETPLAY_ERROR_PLAYER_3_DISCONNECTED => {
            &ui.audio.event_audio.netplay_disconnected[2]
        }
        netplay::NETPLAY_ERROR_PLAYER_4_DISCONNECTED => {
            &ui.audio.event_audio.netplay_disconnected[3]
        }
        _ => panic!("Invalid netplay error"),
    };
    if !unsafe {
        sdl3_sys::audio::SDL_PutAudioStreamData(
            ui.audio.event_audio_stream,
            sound.as_ptr() as *const std::ffi::c_void,
            sound.len() as i32,
        )
    } {
        panic!("Could not play audio");
    }
}

pub fn play_pak_switch(ui: &ui::Ui, pak: device::controller::PakType) {
    if ui.audio.event_audio_stream.is_null() {
        return;
    }

    let sound = match pak {
        device::controller::PakType::RumblePak => &ui.audio.event_audio.rumblepak,
        device::controller::PakType::MemPak => &ui.audio.event_audio.mempak,
        device::controller::PakType::TransferPak => &ui.audio.event_audio.transferpak,
        _ => panic!("Invalid pak type"),
    };
    if !unsafe {
        sdl3_sys::audio::SDL_PutAudioStreamData(
            ui.audio.event_audio_stream,
            sound.as_ptr() as *const std::ffi::c_void,
            sound.len() as i32,
        )
    } {
        panic!("Could not play audio");
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
    let acceptable_latency = (device.ai.freq as f64 * 0.2) * 4.0;
    let min_latency = (device.ai.freq as f64 * 0.02) * 4.0;

    if audio_queued < min_latency {
        let silence_buffer: Vec<u8> = vec![0; min_latency as usize & !3];
        if !unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio.audio_stream,
                silence_buffer.as_ptr() as *const std::ffi::c_void,
                silence_buffer.len() as i32,
            )
        } {
            panic!("Could not play audio");
        }
    }

    if audio_queued < acceptable_latency
        && !unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio.audio_stream,
                primary_buffer.as_ptr() as *const std::ffi::c_void,
                primary_buffer.len() as i32 * 2,
            )
        }
    {
        panic!("Could not play audio");
    }
}

pub fn get_vru_mic_state(ui: &ui::Ui) -> bool {
    unsafe { !sdl3_sys::audio::SDL_AudioStreamDevicePaused(ui.audio.vru_audio_stream) }
}

pub fn set_vru_mic_state(ui: &ui::Ui, talking: bool) {
    if talking {
        unsafe {
            sdl3_sys::audio::SDL_ClearAudioStream(ui.audio.vru_audio_stream);
            sdl3_sys::audio::SDL_ResumeAudioStreamDevice(ui.audio.vru_audio_stream)
        };
    } else {
        unsafe {
            sdl3_sys::audio::SDL_PauseAudioStreamDevice(ui.audio.vru_audio_stream);
            sdl3_sys::audio::SDL_FlushAudioStream(ui.audio.vru_audio_stream)
        };
    }
}

pub fn process_vru_input(ui: &mut ui::Ui) -> u16 {
    let size = unsafe { sdl3_sys::audio::SDL_GetAudioStreamAvailable(ui.audio.vru_audio_stream) };
    let mut audio_data: Vec<f32> = vec![0.0; size as usize / 4];
    let num_bytes = unsafe {
        sdl3_sys::audio::SDL_GetAudioStreamData(
            ui.audio.vru_audio_stream,
            audio_data.as_mut_ptr() as *mut std::ffi::c_void,
            size,
        )
    };
    if num_bytes == -1 {
        panic!("Could not get vru audio stream data");
    }
    //todo
    0x7FFF
}
