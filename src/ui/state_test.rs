use crate::{device, savestates, ui};
use sha2::digest::Digest;

// fixed so that the reported hash is reproducible across runs
pub const RNG_SEED: u64 = 0x6770_6865_7236_3400;
pub const RTC_TIMESTAMP: i64 = 946684800; // 2000-01-01 00:00:00 UTC

enum Phase {
    BeforeSave,
    AfterSave,
    AfterLoad,
}

/// Savestate round-trip test (--savestate-test FRAMES):
/// run FRAMES, save state, run FRAMES and hash the device (A),
/// load the state, run FRAMES and hash the device again (B). A and B must match.
pub struct StateTest {
    frames: u64,
    frame_counter: u64,
    phase: Phase,
    hash: String,
    finished: bool,
}

impl StateTest {
    pub fn new(frames: u64) -> Self {
        Self {
            frames,
            frame_counter: 0,
            phase: Phase::BeforeSave,
            hash: String::new(),
            finished: false,
        }
    }

    pub fn finished(&self) -> bool {
        self.finished
    }
}

fn hash_device(device: &device::Device) -> String {
    ui::video::idle();
    let data = postcard::to_stdvec(device).unwrap();
    sha2::Sha256::digest(&data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn frame(device: &mut device::Device) {
    let Some(mut test) = device.ui.state_test.take() else {
        return;
    };

    if !test.finished() {
        test.frame_counter += 1;
        if test.frame_counter >= test.frames {
            test.frame_counter = 0;
            advance(device, &mut test);
        }
    }

    device.ui.state_test = Some(test);
}

fn advance(device: &mut device::Device, test: &mut StateTest) {
    match test.phase {
        Phase::BeforeSave => {
            savestates::create_savestate(device, false, None);
            test.phase = Phase::AfterSave;
        }
        Phase::AfterSave => {
            test.hash = hash_device(device);
            if !savestates::load_savestate(device, false, None) {
                eprintln!("Savestate test failed: could not load savestate");
                test.finished = true;
            }
            test.phase = Phase::AfterLoad;
        }
        Phase::AfterLoad => {
            let hash = hash_device(device);
            if hash == test.hash {
                println!("Savestate test passed: {hash}");
            } else {
                eprintln!(
                    "Savestate test failed: hash before load {} does not match hash after load {hash}",
                    test.hash
                );
            }
            test.finished = true;
        }
    }
}
