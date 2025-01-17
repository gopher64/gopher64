use crate::device;
use crate::ui;

pub struct PakAudio {
    mempak: Vec<u8>,
    rumblepak: Vec<u8>,
}

pub fn init(ui: &mut ui::Ui, frequency: u64) {
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(frequency as i32),
        channels: Some(2),
        samples: None,
    };
    ui.audio_device = Some(
        ui.audio_subsystem
            .as_ref()
            .unwrap()
            .open_queue::<i16, _>(None, &desired_spec)
            .unwrap(),
    );
    let audio_device = ui.audio_device.as_ref().unwrap();
    audio_device.resume();

    let mempak_audio = Box::new(sdl2::audio::AudioSpecWAV::load_wav_rw(
        &mut sdl2::rwops::RWops::from_bytes(include_bytes!("../../data/mempak.wav"))
            .expect("Could not mempak WAV file"),
    ))
    .expect("Could not load mempak WAV file");
    let rumblepak_audio = Box::new(
        sdl2::audio::AudioSpecWAV::load_wav_rw(
            &mut sdl2::rwops::RWops::from_bytes(include_bytes!("../../data/rumblepak.wav"))
                .expect("Could not load rumblepak WAV file"),
        )
        .expect("Could not load rumblepak WAV file"),
    );

    let cvt = sdl2::audio::AudioCVT::new(
        mempak_audio.format,
        mempak_audio.channels,
        mempak_audio.freq,
        audio_device.spec().format,
        audio_device.spec().channels,
        audio_device.spec().freq,
    )
    .expect("Could not create AudioCVT");

    ui.pak_audio = Some(PakAudio {
        mempak: cvt.convert(mempak_audio.buffer().to_vec()),
        rumblepak: cvt.convert(rumblepak_audio.buffer().to_vec()),
    });
}

pub fn play_pak_switch(ui: &mut ui::Ui, pak: device::controller::PakType) {
    let sound;
    if pak == device::controller::PakType::RumblePak {
        sound = &ui.pak_audio.as_ref().unwrap().rumblepak;
    } else if pak == device::controller::PakType::MemPak {
        sound = &ui.pak_audio.as_ref().unwrap().mempak;
    } else {
        return;
    }
    let audio_device = ui.audio_device.as_ref().unwrap();
    let i16_buffer: Vec<i16> = sound
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    audio_device.queue_audio(&i16_buffer).unwrap();
}

pub fn play_audio(device: &mut device::Device, dram_addr: usize, length: u64) {
    let audio_device = device.ui.audio_device.as_ref().unwrap();
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

    let audio_queued = audio_device.size() as f64;
    let acceptable_latency = (audio_device.spec().freq as f64 * 0.2) * 4.0;
    let min_latency = (audio_device.spec().freq as f64 * 0.02) * 4.0;

    if audio_queued < min_latency {
        let silence_buffer: Vec<i16> = vec![0; ((min_latency - audio_queued) * 2.0) as usize & !1];
        audio_device.queue_audio(&silence_buffer).unwrap();
    }

    if audio_queued < acceptable_latency {
        audio_device.queue_audio(&primary_buffer).unwrap();
    }
}
