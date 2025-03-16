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
}

pub fn init(ui: &mut ui::Ui, frequency: u64) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_AUDIO);

    let game_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: frequency as i32,
        channels: 2,
    };
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

    let mut dst = Default::default();
    if !unsafe {
        sdl3_sys::audio::SDL_GetAudioDeviceFormat(
            ui.audio.audio_device,
            &mut dst,
            std::ptr::null_mut(),
        )
    } {
        panic!("Could not get audio device format");
    }

    ui.audio.audio_stream =
        unsafe { sdl3_sys::audio::SDL_CreateAudioStream(&game_audio_spec, &dst) };
    if ui.audio.audio_stream.is_null() {
        return;
    }
    if !unsafe {
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.audio_stream, ui.audio.gain)
            && sdl3_sys::audio::SDL_BindAudioStream(ui.audio.audio_device, ui.audio.audio_stream)
    } {
        panic!("Could not bind audio stream");
    }

    let wav_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: 24000,
        channels: 1,
    };

    ui.audio.event_audio_stream =
        unsafe { sdl3_sys::audio::SDL_CreateAudioStream(&wav_audio_spec, &dst) };
    if !unsafe {
        sdl3_sys::audio::SDL_SetAudioStreamGain(ui.audio.event_audio_stream, ui.audio.gain)
            && sdl3_sys::audio::SDL_BindAudioStream(
                ui.audio.audio_device,
                ui.audio.event_audio_stream,
            )
    } {
        panic!("Could not bind audio stream");
    }
}

pub fn close(ui: &mut ui::Ui) {
    unsafe {
        if !ui.audio.audio_stream.is_null() {
            sdl3_sys::audio::SDL_DestroyAudioStream(ui.audio.audio_stream);
            ui.audio.audio_stream = std::ptr::null_mut();
        }
        if !ui.audio.event_audio_stream.is_null() {
            sdl3_sys::audio::SDL_DestroyAudioStream(ui.audio.event_audio_stream);
            ui.audio.event_audio_stream = std::ptr::null_mut();
        }
        sdl3_sys::audio::SDL_CloseAudioDevice(ui.audio.audio_device);
        ui.audio.audio_device = 0;
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

pub fn play_netplay_audio(ui: &mut ui::Ui, error: u32) {
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

pub fn play_pak_switch(ui: &mut ui::Ui, pak: device::controller::PakType) {
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

pub fn play_audio(device: &mut device::Device, dram_addr: usize, length: u64) {
    if device.ui.audio.audio_stream.is_null() {
        return;
    }

    let mut primary_buffer: Vec<i16> = vec![0; length as usize / 2];
    let mut i = 0;
    while i < length as usize / 2 {
        // Left channel
        primary_buffer[i] = device.rdram.mem[dram_addr + (i * 2) + 2] as i16
            | ((device.rdram.mem[dram_addr + (i * 2) + 3] as i16) << 8);

        // Right channel
        primary_buffer[i + 1] = device.rdram.mem[dram_addr + (i * 2)] as i16
            | ((device.rdram.mem[dram_addr + (i * 2) + 1] as i16) << 8);
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
