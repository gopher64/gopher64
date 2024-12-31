use crate::device;
use crate::ui;

pub const JCMD_STATUS: u8 = 0x00;
pub const JCMD_CONTROLLER_READ: u8 = 0x01;
pub const JCMD_PAK_READ: u8 = 0x02;
pub const JCMD_PAK_WRITE: u8 = 0x03;
//pub const JCMD_VRU_READ: u8 = 0x09;
//pub const JCMD_VRU_WRITE: u8 = 0x0A;
//pub const JCMD_VRU_READ_STATUS: u8 = 0x0B;
//pub const JCMD_VRU_WRITE_CONFIG: u8 = 0x0C;
//pub const JCMD_VRU_WRITE_INIT: u8 = 0x0D;
pub const JCMD_RESET: u8 = 0xff;

//pub const JDT_NONE: u16 = 0x0000;
pub const JDT_JOY_ABS_COUNTERS: u16 = 0x0001; /* joystick with absolute coordinates */
//pub const JDT_JOY_REL_COUNTERS: u16 = 0x0002; /* joystick with relative coordinates (= mouse) */
pub const JDT_JOY_PORT: u16 = 0x0004; /* has port for external paks */
//pub const JDT_VRU: u16 = 0x0100; /* VRU */
pub const PAK_CHUNK_SIZE: usize = 0x20;
pub const CONT_STATUS_PAK_NOT_PRESENT: u8 = 0;
pub const CONT_STATUS_PAK_PRESENT: u8 = 1;
pub const CONT_FLAVOR: u16 = JDT_JOY_ABS_COUNTERS | JDT_JOY_PORT;

#[derive(Copy, Clone)]
pub struct PakHandler {
    pub read: fn(&mut device::Device, usize, u16, usize, usize),
    pub write: fn(&mut device::Device, usize, u16, usize, usize),
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
            let input = ui::input::get(&mut device.ui, channel);
            device.pif.ram[offset..offset + 4].copy_from_slice(&input.to_ne_bytes());
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

pub fn pak_read_block(
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
        device.pif.ram[dcrc] = pak_data_crc(device, data, PAK_CHUNK_SIZE)
    } else {
        device.pif.ram[dcrc] = !pak_data_crc(device, data, PAK_CHUNK_SIZE)
    }
}

pub fn pak_write_block(
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
        device.pif.ram[dcrc] = pak_data_crc(device, data, PAK_CHUNK_SIZE)
    } else {
        device.pif.ram[dcrc] = !pak_data_crc(device, data, PAK_CHUNK_SIZE)
    }
}

pub fn pak_data_crc(device: &device::Device, data_offset: usize, size: usize) -> u8 {
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
