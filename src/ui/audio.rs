use crate::device;
use crate::ui;

pub fn init(ui: &mut ui::Ui, frequency: u64) {
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(frequency as i32),
        channels: Some(2),
        samples: None,
    };
    ui.audio_device = Some(
        ui.audio_subsystem
            .as_mut()
            .unwrap()
            .open_queue::<i16, _>(None, &desired_spec)
            .unwrap(),
    );
    ui.audio_device.as_mut().unwrap().resume();
}

pub fn play_audio(device: &mut device::Device, dram_addr: usize, length: u64) {
    let audio_device = device.ui.audio_device.as_mut().unwrap();
    let mut primary_buffer: Vec<i16> = vec![0; length as usize / 2];
    let mut i = 0;
    while i < length as usize / 2 {
        // Left channel
        primary_buffer[i] = device.rdram.mem[dram_addr + (i * 2) + 2] as i16
            | (device.rdram.mem[dram_addr + (i * 2) + 3] as i16) << 8;

        // Right channel
        primary_buffer[i + 1] = device.rdram.mem[dram_addr + (i * 2)] as i16
            | (device.rdram.mem[dram_addr + (i * 2) + 1] as i16) << 8;
        i += 2;
    }

    let audio_queued = audio_device.size() as f64;
    let acceptable_latency = (audio_device.spec().freq as f64 * 0.2) * 4.0;
    let min_latency = (audio_device.spec().freq as f64 * 0.02) * 4.0;

    if audio_device.status() != sdl2::audio::AudioStatus::Paused && audio_queued < min_latency {
        audio_device.pause();
    } else if audio_device.status() == sdl2::audio::AudioStatus::Paused
        && audio_queued >= (min_latency * 2.0)
    {
        audio_device.resume();
    }

    if audio_queued < acceptable_latency {
        let _ = audio_device.queue_audio(&primary_buffer);
    }
}
