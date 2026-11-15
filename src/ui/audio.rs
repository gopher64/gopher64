use crate::device;
use crate::netplay;
use crate::ui;

const BYTES_PER_FRAME: usize = 4; // Stereo, signed 16-bit samples.
const TARGET_FRAMES: f64 = 2.0;
const MAX_FRAMES: f64 = 12.0;

pub fn init(device: &mut device::Device) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_AUDIO);
    init_game_audio(device);
}

// Correct clock drift before it causes starvation or excessive latency. Slew at most 0.05% per second of submitted audio.
fn frequency_ratio(current: f64, queued_frames: f64, seconds: f64) -> f64 {
    let target = (1.0 + (queued_frames - TARGET_FRAMES) * 0.001).clamp(0.995, 1.005);
    let step = seconds * 0.0005;
    (current + (target - current).clamp(-step, step)).clamp(0.995, 1.005)
}

pub fn update_freq(device: &mut device::Device) {
    let stream = device.ui.audio.audio_stream;
    if stream.is_null() {
        return;
    }
    let game_audio_spec = sdl3_sys::audio::SDL_AudioSpec {
        format: sdl3_sys::audio::SDL_AUDIO_S16LE,
        freq: device.ai.freq as i32,
        channels: 2,
    };
    unsafe {
        sdl3_sys::audio::SDL_SetAudioStreamFormat(stream, &game_audio_spec, std::ptr::null());
    }
}

// A user load/rewind changes the audible timeline. Netplay rollback must preserve it.
pub fn reset_game_audio(device: &mut device::Device) {
    reset_stream(&mut device.ui.audio);
    update_freq(device);
}

fn reset_stream(audio: &mut ui::Audio) {
    unsafe {
        sdl3_sys::audio::SDL_ClearAudioStream(audio.audio_stream);
        sdl3_sys::audio::SDL_SetAudioStreamFrequencyRatio(audio.audio_stream, 1.0);
    }
    audio.frequency_ratio = 1.0;
}

pub fn init_game_audio(device: &mut device::Device) {
    device.ui.audio.frequency_ratio = 1.0;
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
    } {
        panic!("Could not initialize audio stream");
    }
    unsafe {
        sdl3_sys::audio::SDL_ResumeAudioStreamDevice(device.ui.audio.audio_stream);
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

pub fn play_audio(device: &mut device::Device, dram_addr: usize, length: u64) {
    if device.ui.audio.audio_stream.is_null() {
        return;
    }

    if netplay::in_rollback(device.netplay.as_ref()) {
        return;
    }

    let audio = &mut device.ui.audio;
    audio.primary_buffer.resize(length as usize / 2, 0);
    let primary_buffer = &mut audio.primary_buffer;
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

    queue_audio(
        audio,
        device.ai.freq,
        device.vi.frame_time,
        device.ui.speed_limiter.enabled,
    );
}

fn queue_audio(audio: &mut ui::Audio, frequency: u64, frame_time: f64, speed_limited: bool) {
    let bytes_per_second = frequency as f64 * BYTES_PER_FRAME as f64;
    let bytes_per_video_frame = bytes_per_second * frame_time;
    if bytes_per_video_frame <= 0.0 || audio.primary_buffer.is_empty() {
        return;
    }

    let incoming = std::mem::size_of_val(audio.primary_buffer.as_slice());
    let target_bytes =
        (bytes_per_video_frame * TARGET_FRAMES) as usize / BYTES_PER_FRAME * BYTES_PER_FRAME;
    // The incoming block contributes to the recovery reserve. Only pad its
    // shortfall, so a full block can restart playback without added silence.
    let padding = target_bytes.saturating_sub(incoming);
    // Prepare reusable silence before locking: the playback thread needs this
    // same mutex to consume audio. Only grow the buffer when necessary.
    if audio.silence_buffer.len() < padding {
        audio.silence_buffer.resize(padding, 0);
    }
    let seconds = incoming as f64 / bytes_per_second;
    let stream = audio.audio_stream;
    unsafe {
        if !sdl3_sys::audio::SDL_LockAudioStream(stream) {
            return;
        }
        let queued = sdl3_sys::audio::SDL_GetAudioStreamQueued(stream);
        let available = sdl3_sys::audio::SDL_GetAudioStreamAvailable(stream);
        if queued >= 0 && available >= 0 {
            let queued_frames = queued as f64 / bytes_per_video_frame;
            // Resampling retains input bytes even after all playable output is consumed.
            let starved = available == 0;
            // Test the existing backlog: a large arriving DMA block alone is
            // not evidence that playback has fallen behind.
            let overrun = queued_frames > MAX_FRAMES;
            if overrun {
                // Prefer current game audio over playing an increasingly stale
                // backlog (particularly after fast-forward or a host stall).
                // Queue current game audio immediately, without silence padding.
                sdl3_sys::audio::SDL_ClearAudioStream(stream);
            }
            if starved && !overrun && padding > 0 {
                // SDL supplies silence while starved. Prime a small reserve on
                // recovery so the next host scheduling jitter needn't starve us
                // again, counting the incoming audio toward that reserve.
                // All padding is aligned to complete stereo frames.
                sdl3_sys::audio::SDL_PutAudioStreamData(
                    stream,
                    audio.silence_buffer.as_ptr().cast(),
                    padding as i32,
                );
            }
            let ratio = if speed_limited {
                frequency_ratio(
                    audio.frequency_ratio,
                    if overrun {
                        TARGET_FRAMES
                    } else if starved {
                        // Retained resampler input isn't playable audio. An
                        // underrun must slow consumption even if this block
                        // is large enough to need no recovery padding.
                        0.0
                    } else {
                        queued_frames
                    },
                    seconds,
                )
            } else {
                1.0
            };
            audio.frequency_ratio = ratio;
            sdl3_sys::audio::SDL_SetAudioStreamFrequencyRatio(stream, ratio as f32);
            sdl3_sys::audio::SDL_PutAudioStreamData(
                stream,
                audio.primary_buffer.as_ptr().cast(),
                incoming as i32,
            );
        }
        sdl3_sys::audio::SDL_UnlockAudioStream(stream);
    }
}
