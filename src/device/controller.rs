use crate::{device, netplay, savestates, ui};

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

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct PakHandler {
    #[serde(skip, default = "savestates::default_pak_handler")]
    pub read: fn(&mut device::Device, usize, u16, usize, usize),
    #[serde(skip, default = "savestates::default_pak_handler")]
    pub write: fn(&mut device::Device, usize, u16, usize, usize),
    pub pak_type: PakType,
}

pub fn process(device: &mut device::Device, channel: usize) {
    let cmd = device.pif.ram[device.pif.channels[channel].tx_buf.unwrap()];

    match cmd {
        JCMD_RESET | JCMD_STATUS => {
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = CONT_FLAVOR as u8;
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 1] =
                (CONT_FLAVOR >> 8) as u8;
            if device.pif.channels[channel].pak_handler.is_none() {
                device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 2] =
                    CONT_STATUS_PAK_NOT_PRESENT;
            } else {
                device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 2] =
                    CONT_STATUS_PAK_PRESENT;
            }
        }
        JCMD_CONTROLLER_READ => {
            let offset = device.pif.channels[channel].rx_buf.unwrap();
            let input = if device.netplay.is_none() {
                ui::input::get(&mut device.ui, channel)
            } else {
                if device.netplay.as_ref().unwrap().player_number as usize == channel {
                    let local_input = ui::input::get(&mut device.ui, 0);
                    netplay::send_input(device.netplay.as_ref().unwrap(), local_input);
                }

                netplay::get_input(device, channel)
            };

            device.pif.ram[offset..offset + 4].copy_from_slice(&input.0.to_ne_bytes());
            if input.1 {
                // pak change button pressed
                if device::events::get_event(device, device::events::EVENT_TYPE_PAK).is_none() {
                    device.pif.channels[channel].change_pak =
                        device.pif.channels[channel].pak_handler.unwrap().pak_type;
                    device.pif.channels[channel].pak_handler = None;
                    device::events::create_event(
                        device,
                        device::events::EVENT_TYPE_PAK,
                        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize]
                            + (device.cpu.clock_rate), // 1 second
                    )
                }
            }
        }
        JCMD_PAK_READ => pak_read_block(
            device,
            device.pif.channels[channel].tx_buf.unwrap() + 1,
            device.pif.channels[channel].rx_buf.unwrap(),
            device.pif.channels[channel].rx_buf.unwrap() + 32,
            channel,
        ),
        JCMD_PAK_WRITE => pak_write_block(
            device,
            device.pif.channels[channel].tx_buf.unwrap() + 1,
            device.pif.channels[channel].tx_buf.unwrap() + 3,
            device.pif.channels[channel].rx_buf.unwrap(),
            channel,
        ),
        _ => println!("unknown controller command {}", cmd),
    }
}

fn pak_read_block(
    device: &mut device::Device,
    addr_acrc: usize,
    data: usize,
    dcrc: usize,
    channel: usize,
) {
    let address =
        ((device.pif.ram[addr_acrc] as u16) << 8) | (device.pif.ram[addr_acrc + 1] & 0xe0) as u16;
    let handler = device.pif.channels[channel].pak_handler;

    if handler.is_some() {
        (handler.unwrap().read)(device, channel, address, data, PAK_CHUNK_SIZE);
        device.pif.ram[dcrc] = data_crc(device, data, PAK_CHUNK_SIZE)
    } else {
        device.pif.ram[dcrc] = !data_crc(device, data, PAK_CHUNK_SIZE)
    }
}

fn pak_write_block(
    device: &mut device::Device,
    addr_acrc: usize,
    data: usize,
    dcrc: usize,
    channel: usize,
) {
    let address =
        ((device.pif.ram[addr_acrc] as u16) << 8) | (device.pif.ram[addr_acrc + 1] & 0xe0) as u16;
    let handler = device.pif.channels[channel].pak_handler;

    if handler.is_some() {
        (handler.unwrap().write)(device, channel, address, data, PAK_CHUNK_SIZE);
        device.pif.ram[dcrc] = data_crc(device, data, PAK_CHUNK_SIZE)
    } else {
        device.pif.ram[dcrc] = !data_crc(device, data, PAK_CHUNK_SIZE)
    }
}

fn data_crc(device: &device::Device, data_offset: usize, size: usize) -> u8 {
    let mut i = 0;
    let mut crc = 0;

    while i <= size {
        let mut mask = 0x80;
        while mask >= 1 {
            let xor_tap = if crc & 0x80 != 0 { 0x85 } else { 0x00 };
            crc <<= 1;
            if i != size && (device.pif.ram[data_offset + i] & mask) != 0 {
                crc |= 1;
            }
            crc ^= xor_tap;
            mask >>= 1
        }
        i += 1;
    }
    crc
}

pub fn pak_switch_event(device: &mut device::Device) {
    for (i, channel) in device.pif.channels.iter_mut().enumerate() {
        if channel.change_pak != PakType::None {
            //stop rumble if it is on
            if device.netplay.is_none() {
                device::ui::input::set_rumble(&mut device.ui, i, 0);
            } else if device.netplay.as_ref().unwrap().player_number as usize == i {
                device::ui::input::set_rumble(&mut device.ui, 0, 0);
            }

            let new_pak_type = match channel.change_pak {
                PakType::MemPak => PakType::RumblePak,
                PakType::RumblePak => PakType::TransferPak,
                PakType::TransferPak => PakType::MemPak,
                _ => {
                    panic!("Invalid pak type");
                }
            };

            if new_pak_type == PakType::MemPak {
                channel.pak_handler = Some(device::controller::PakHandler {
                    read: device::controller::mempak::read,
                    write: device::controller::mempak::write,
                    pak_type: new_pak_type,
                });
            } else if new_pak_type == PakType::RumblePak {
                channel.pak_handler = Some(device::controller::PakHandler {
                    read: device::controller::rumble::read,
                    write: device::controller::rumble::write,
                    pak_type: new_pak_type,
                });
            } else if new_pak_type == PakType::TransferPak {
                channel.pak_handler = Some(device::controller::PakHandler {
                    read: device::controller::transferpak::read,
                    write: device::controller::transferpak::write,
                    pak_type: new_pak_type,
                });
            }
            ui::audio::play_pak_switch(&mut device.ui, new_pak_type);
            channel.change_pak = PakType::None;
        }
    }
}
