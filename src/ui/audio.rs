use crate::device;
use crate::ui;

pub struct PakAudio {
    pub mempak: Vec<u8>,
    pub rumblepak: Vec<u8>,
}

pub fn init(ui: &mut ui::Ui, frequency: u64) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_AUDIO);

    let audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: frequency as i32,
        channels: 2,
    };
    ui.audio_stream = unsafe {
        sdl3_sys::audio::SDL_OpenAudioDeviceStream(
            sdl3_sys::audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
            &audio_spec,
            None,
            std::ptr::null_mut(),
        )
    };
    if ui.audio_stream.is_null() {
        return;
    }

    if !unsafe { sdl3_sys::audio::SDL_ResumeAudioStreamDevice(ui.audio_stream) } {
        panic!("Could not resume audio stream");
    }

    let mut wav_audio_spec: sdl3_sys::audio::SDL_AudioSpec = Default::default();
    for item in [&mut ui.pak_audio.mempak, &mut ui.pak_audio.rumblepak] {
        let mut wav_length = item.len() as u32;

        let mut wav_buf_ptr: *mut u8 = std::ptr::null_mut();
        unsafe {
            if !sdl3_sys::audio::SDL_LoadWAV_IO(
                sdl3_sys::iostream::SDL_IOFromConstMem(
                    item.as_ptr() as *const std::ffi::c_void,
                    wav_length as usize,
                ),
                true,
                &mut wav_audio_spec,
                &mut wav_buf_ptr,
                &mut wav_length,
            ) {
                panic!("Could not load WAV file");
            }
            sdl3_sys::stdinc::SDL_free(wav_buf_ptr as *mut std::ffi::c_void);
        }
    }
    ui.pak_audio_stream = unsafe {
        sdl3_sys::audio::SDL_OpenAudioDeviceStream(
            sdl3_sys::audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
            &wav_audio_spec,
            None,
            std::ptr::null_mut(),
        )
    };
    if !unsafe { sdl3_sys::audio::SDL_ResumeAudioStreamDevice(ui.pak_audio_stream) } {
        panic!("Could not resume pak audio stream");
    }

    ui.audio_spec = Some(audio_spec);
}

pub fn close(ui: &mut ui::Ui) {
    unsafe {
        if !ui.audio_stream.is_null() {
            sdl3_sys::audio::SDL_DestroyAudioStream(ui.audio_stream);
            ui.audio_stream = std::ptr::null_mut();
        }
        if !ui.pak_audio_stream.is_null() {
            sdl3_sys::audio::SDL_DestroyAudioStream(ui.pak_audio_stream);
            ui.pak_audio_stream = std::ptr::null_mut();
        }
    }
}

pub fn play_pak_switch(ui: &mut ui::Ui, pak: device::controller::PakType) {
    if ui.pak_audio_stream.is_null() {
        return;
    }

    let sound;
    if pak == device::controller::PakType::RumblePak {
        sound = &ui.pak_audio.rumblepak;
    } else if pak == device::controller::PakType::MemPak {
        sound = &ui.pak_audio.mempak;
    } else {
        return;
    }
    if !unsafe {
        sdl3_sys::audio::SDL_PutAudioStreamData(
            ui.pak_audio_stream,
            sound.as_ptr() as *const std::ffi::c_void,
            sound.len() as i32,
        )
    } {
        panic!("Could not play audio");
    }
}

pub fn play_audio(device: &mut device::Device, dram_addr: usize, length: u64) {
    if device.ui.audio_stream.is_null() {
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
        unsafe { sdl3_sys::audio::SDL_GetAudioStreamQueued(device.ui.audio_stream) } as f64;
    let acceptable_latency = (device.ui.audio_spec.unwrap().freq as f64 * 0.2) * 4.0;
    let min_latency = (device.ui.audio_spec.unwrap().freq as f64 * 0.02) * 4.0;

    if audio_queued < min_latency {
        let silence_buffer: Vec<u8> = vec![0; (min_latency - audio_queued) as usize & !3];
        if !unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio_stream,
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
                device.ui.audio_stream,
                primary_buffer.as_ptr() as *const std::ffi::c_void,
                primary_buffer.len() as i32 * 2,
            )
        }
    {
        panic!("Could not play audio");
    }
}
