use crate::device;
use crate::ui;

struct PakAudioData {
    converted_data: Vec<u8>,
    source: Vec<u8>,
}

pub struct PakAudio {
    mempak: PakAudioData,
    rumblepak: PakAudioData,
}

pub fn init(ui: &mut ui::Ui, frequency: u64) {
    if !unsafe { sdl3_sys::init::SDL_InitSubSystem(sdl3_sys::init::SDL_INIT_AUDIO) } {
        panic!("Could not initialize SDL audio");
    }

    let desired_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: frequency as i32,
        channels: 2,
    };
    ui.audio_stream = Some(unsafe {
        sdl3_sys::audio::SDL_OpenAudioDeviceStream(
            sdl3_sys::audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
            &desired_spec,
            None,
            std::ptr::null_mut(),
        )
    });
    unsafe { sdl3_sys::audio::SDL_ResumeAudioStreamDevice(ui.audio_stream.unwrap()) };

    ui.pak_audio = Some(PakAudio {
        mempak: PakAudioData {
            converted_data: Vec::new(),
            source: include_bytes!("../../data/mempak.wav").to_vec(),
        },
        rumblepak: PakAudioData {
            converted_data: Vec::new(),
            source: include_bytes!("../../data/rumblepak.wav").to_vec(),
        },
    });
    let pak_audio = ui.pak_audio.as_mut().unwrap();
    for item in [&mut pak_audio.mempak, &mut pak_audio.rumblepak] {
        let mut length = item.source.len() as u32;

        let mut audio_spec: sdl3_sys::audio::SDL_AudioSpec = Default::default();
        let mut buf_ptr: *mut u8 = std::ptr::null_mut();
        if !unsafe {
            sdl3_sys::audio::SDL_LoadWAV_IO(
                sdl3_sys::iostream::SDL_IOFromConstMem(
                    item.source.as_ptr() as *const std::ffi::c_void,
                    length as usize,
                ),
                true,
                &mut audio_spec,
                &mut buf_ptr,
                &mut length,
            )
        } {
            panic!("Could not load WAV file");
        }

        let mut dst_ptr: *mut u8 = std::ptr::null_mut();
        let mut dst_length: i32 = 0;
        if !unsafe {
            sdl3_sys::audio::SDL_ConvertAudioSamples(
                &audio_spec,
                buf_ptr,
                length as i32,
                &desired_spec,
                &mut dst_ptr,
                &mut dst_length,
            )
        } {
            panic!("Could not convert WAV file");
        }
        item.converted_data =
            unsafe { Vec::from_raw_parts(dst_ptr, dst_length as usize, dst_length as usize) };
    }

    ui.audio_spec = Some(desired_spec);
}

pub fn play_pak_switch(ui: &mut ui::Ui, pak: device::controller::PakType) {
    let sound;
    if pak == device::controller::PakType::RumblePak {
        sound = &ui.pak_audio.as_ref().unwrap().rumblepak.converted_data;
    } else if pak == device::controller::PakType::MemPak {
        sound = &ui.pak_audio.as_ref().unwrap().mempak.converted_data;
    } else {
        return;
    }
    let i16_buffer: Vec<i16> = sound
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    unsafe {
        sdl3_sys::audio::SDL_PutAudioStreamData(
            ui.audio_stream.unwrap(),
            i16_buffer.as_ptr() as *const std::ffi::c_void,
            i16_buffer.len() as i32,
        );
    }
}

pub fn play_audio(device: &mut device::Device, dram_addr: usize, length: u64) {
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
        unsafe { sdl3_sys::audio::SDL_GetAudioStreamQueued(device.ui.audio_stream.unwrap()) }
            as f64;
    let acceptable_latency = (device.ui.audio_spec.unwrap().freq as f64 * 0.2) * 4.0;
    let min_latency = (device.ui.audio_spec.unwrap().freq as f64 * 0.02) * 4.0;

    if audio_queued < min_latency {
        let silence_buffer: Vec<i16> = vec![0; ((min_latency - audio_queued) * 2.0) as usize & !1];
        unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio_stream.unwrap(),
                silence_buffer.as_ptr() as *const std::ffi::c_void,
                silence_buffer.len() as i32,
            );
        }
    }

    if audio_queued < acceptable_latency {
        unsafe {
            sdl3_sys::audio::SDL_PutAudioStreamData(
                device.ui.audio_stream.unwrap(),
                primary_buffer.as_ptr() as *const std::ffi::c_void,
                primary_buffer.len() as i32,
            );
        }
    }
}
