use crate::{device, ui};
use device::pif::{JoybusAction, JoybusError, write_reply};

pub mod bk2;
pub mod gbcart;
pub mod mempak;
pub mod rumble;
pub mod transferpak;
pub mod vru;

pub const JCMD_STATUS: u8 = 0x00;
pub const JCMD_CONTROLLER_READ: u8 = 0x01;
const JCMD_PAK_READ: u8 = 0x02;
const JCMD_PAK_WRITE: u8 = 0x03;
pub const JCMD_RESET: u8 = 0xff;

//const JDT_NONE: u16 = 0x0000;
const JDT_JOY_ABS_COUNTERS: u16 = 0x0001; /* joystick with absolute coordinates */
//const JDT_JOY_REL_COUNTERS: u16 = 0x0002; /* joystick with relative coordinates (= mouse) */
const JDT_JOY_PORT: u16 = 0x0004; /* has port for external paks */
const PAK_CHUNK_SIZE: usize = 0x20;
const CONT_STATUS_PAK_PRESENT: u8 = 1;
const CONT_STATUS_PAK_NOT_PRESENT: u8 = 2;
const CONT_FLAVOR: u16 = JDT_JOY_ABS_COUNTERS | JDT_JOY_PORT;

#[derive(Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PakType {
    None = 0,
    MemPak = 1,
    RumblePak = 2,
    TransferPak = 3,
}

pub fn process(
    device: &mut device::Device,
    channel: usize,
    tx: &[u8],
    rx: &mut [u8],
) -> Result<JoybusAction, JoybusError> {
    let pak_type = device.pif.channels[channel].pak_type;
    match tx[0] {
        JCMD_RESET | JCMD_STATUS => {
            let status = if pak_type == PakType::None {
                CONT_STATUS_PAK_NOT_PRESENT
            } else {
                CONT_STATUS_PAK_PRESENT
            };
            let [lo, hi] = CONT_FLAVOR.to_le_bytes();
            let reply = [lo, hi, status];
            write_reply(rx, &reply);
        }
        JCMD_CONTROLLER_READ => {
            let input = if let Some(netplay) = &mut device.netplay {
                netplay.inputs[channel]
            } else if let Some(playback) = &device.bk2_playback {
                playback.input(channel)
            } else if device.ui.state_test.is_some() {
                ui::input::InputData {
                    data: 0,
                    pak_change_pressed: false,
                }
            } else {
                ui::input::get(&mut device.ui, channel)
            };

            let reply = input.data.to_ne_bytes();
            write_reply(rx, &reply);
            if input.pak_change_pressed && pak_type != PakType::None {
                return Ok(JoybusAction::SwitchPak);
            }
        }
        JCMD_PAK_READ => {
            let [_, hi, lo, ..] = tx else {
                return Err(JoybusError::Timeout);
            };
            let address = u16::from_be_bytes([*hi, *lo & 0xe0]);
            let mut reply = [0; PAK_CHUNK_SIZE + 1];
            let data = reply.first_chunk_mut::<PAK_CHUNK_SIZE>().unwrap();
            write_reply(data, rx);
            match pak_type {
                PakType::MemPak => mempak::read(device, channel, address, data),
                PakType::RumblePak => rumble::read(address, data),
                PakType::TransferPak => transferpak::read(device, channel, address, data),
                PakType::None => {}
            }
            reply[PAK_CHUNK_SIZE] = pak_crc(pak_type, data);
            write_reply(rx, &reply);
        }
        JCMD_PAK_WRITE => {
            let [_, hi, lo, data @ ..] = tx else {
                return Err(JoybusError::Timeout);
            };
            let data = &data[..data.len().min(PAK_CHUNK_SIZE)];
            let Some(&last_byte) = data.last() else {
                return Err(JoybusError::Timeout);
            };
            let address = u16::from_be_bytes([*hi, *lo & 0xe0]);
            match pak_type {
                PakType::MemPak => mempak::write(device, channel, address, data),
                PakType::RumblePak => rumble::write(device, channel, address, last_byte),
                PakType::TransferPak => transferpak::write(device, channel, address, data),
                PakType::None => {}
            }
            write_reply(rx, &[pak_crc(pak_type, data)]);
        }
        cmd => {
            eprintln!("unknown controller command {cmd}");
            return Err(JoybusError::Timeout);
        }
    }
    Ok(JoybusAction::None)
}

fn pak_crc(pak_type: PakType, data: &[u8]) -> u8 {
    // A partial Pak write does not produce a full-block CRC.
    let crc = if data.len() == PAK_CHUNK_SIZE {
        data_crc(data)
    } else {
        0
    };
    if pak_type == PakType::None { !crc } else { crc }
}

fn data_crc(data: &[u8]) -> u8 {
    let mut crc = 0;

    for byte in data.iter().copied().chain([0]) {
        let mut mask = 0x80;
        while mask >= 1 {
            let xor_tap = if crc & 0x80 != 0 { 0x85 } else { 0x00 };
            crc <<= 1;
            if byte & mask != 0 {
                crc |= 1;
            }
            crc ^= xor_tap;
            mask >>= 1
        }
    }
    crc
}

pub fn start_pak_switch(device: &mut device::Device, port: usize) {
    if device::events::get_event(device, device::events::EVENT_TYPE_PAK).is_some() {
        return;
    }

    let channel = &mut device.pif.channels[port];
    channel.change_pak = channel.pak_type;
    channel.pak_type = PakType::None;
    device::events::create_event(
        device,
        device::events::EVENT_TYPE_PAK,
        device.cpu.clock_rate / 2, // 500ms
    );
}

pub fn pak_switch_event(device: &mut device::Device) {
    for (i, channel) in device.pif.channels.iter_mut().enumerate() {
        if channel.change_pak != PakType::None {
            //stop rumble if it is on
            if let Some(netplay) = &device.netplay {
                if netplay.player_number == i {
                    device::ui::input::set_rumble(&device.ui, 0, 0);
                }
            } else {
                device::ui::input::set_rumble(&device.ui, i, 0);
            }

            let new_pak_type = match channel.change_pak {
                PakType::MemPak => PakType::RumblePak,
                PakType::RumblePak => {
                    if device.transferpaks[i].cart.rom.is_empty() {
                        PakType::MemPak
                    } else {
                        PakType::TransferPak
                    }
                }
                PakType::TransferPak => PakType::MemPak,
                PakType::None => unreachable!("no pak change pending"),
            };

            let name = match new_pak_type {
                PakType::MemPak => "MemPak",
                PakType::RumblePak => "RumblePak",
                PakType::TransferPak => "TransferPak",
                PakType::None => unreachable!(),
            };
            ui::video::onscreen_message(
                &format!("P{} {name} selected", i + 1),
                ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
            );
            channel.pak_type = new_pak_type;
            channel.change_pak = PakType::None;
        }
    }
}
