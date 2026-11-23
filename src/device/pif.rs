mod rom;
use crate::device;

pub const PIF_RAM_SIZE: usize = 64;
const PIF_CHANNELS_COUNT: usize = 5;
const PIF_RAM_OFFSET: usize = 0x7C0;
const PIF_MASK: usize = 0x7FF;
const CHL_LEN: usize = 0x20;
const PIF_COMMAND: usize = PIF_RAM_SIZE - 1;
const PIF_CONFIGURE_JOYBUS: u8 = 0x01;
const JOYBUS_LENGTH_MASK: u8 = 0x3f;
const JOYBUS_TX_RESET: u8 = 0x40;
const JOYBUS_TX_SKIP: u8 = 0x80;

// Approximate n64-systembench read timings in CPU cycles, including the Joybus
// exchange but not the firmware time to parse a newly written frame.
const JOYBUS_READ_BASE: u64 = 22_400;
const JOYBUS_CHANNEL_SETUP: u64 = 18_800;
const JOYBUS_BYTE: u64 = 3_125;
const JOYBUS_REPLY_TIMEOUT: u64 = 8_000;

pub enum JoybusError {
    Timeout = 0x40,
    NoResponse = 0x80,
}

pub enum JoybusAction {
    None,
    SwitchPak,
}

type JoybusHandler =
    fn(&mut device::Device, usize, &[u8], &mut [u8]) -> Result<JoybusAction, JoybusError>;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Pif {
    #[serde(with = "serde_big_array::BigArray")]
    pub rom: [u8; 1984],
    #[serde(with = "serde_big_array::BigArray")]
    pub ram: [u8; PIF_RAM_SIZE],
    pub channels: [PifChannel; PIF_CHANNELS_COUNT],
}

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct PifChannel {
    pub header: Option<usize>,
    #[serde(skip)]
    pub process: Option<JoybusHandler>,
    pub pak_type: device::controller::PakType,
    pub change_pak: device::controller::PakType,
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 3000); //based on https://github.com/rasky/n64-systembench

    let mut masked_address = address as usize & PIF_MASK;
    if masked_address < PIF_RAM_OFFSET {
        u32::from_be_bytes(
            device.pif.rom[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        )
    } else {
        masked_address -= PIF_RAM_OFFSET;
        u32::from_be_bytes(
            device.pif.ram[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        )
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let mut masked_address = address as usize & PIF_MASK;
    if masked_address < PIF_RAM_OFFSET {
        panic!("write to pif rom")
    }
    masked_address -= PIF_RAM_OFFSET;
    let mut data = u32::from_be_bytes(
        device.pif.ram[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    );
    device::memory::masked_write_32(&mut data, value, mask);
    device.pif.ram[masked_address..masked_address + 4].copy_from_slice(&data.to_be_bytes());

    device::si::io_write(device);
}

pub fn update_pif_ram(device: &mut device::Device) -> u64 {
    let mut cycles = JOYBUS_READ_BASE;
    let mut pak_switch_port = None;
    // Give handlers only their packet's payloads, separate from device state.
    let mut ram = device.pif.ram;
    // PIF firmware executes channel 4 first, even when reused packets overlap.
    for (port, channel) in device.pif.channels.into_iter().enumerate().rev() {
        let Some(header) = channel.header else {
            continue;
        };
        let packet = &mut ram[header..PIF_COMMAND];
        let [tx_header, rx_header, payload @ ..] = packet else {
            continue;
        };
        // A reset sends no payload. The physical reset pulse is not emulated.
        if *tx_header & (JOYBUS_TX_SKIP | JOYBUS_TX_RESET) != 0 {
            continue;
        }
        // A frame can be reused with different header bytes, so check lengths again.
        let (tx_len, rx_len) = packet_lengths(*tx_header, *rx_header);
        if tx_len + rx_len > payload.len() {
            *rx_header = rx_len as u8 | JoybusError::Timeout as u8;
            continue;
        }
        if tx_len == 0 {
            continue;
        }
        let (tx, rx) = payload[..tx_len + rx_len].split_at_mut(tx_len);
        let result = match channel.process {
            Some(process) => process(device, port, tx, rx),
            None => Err(JoybusError::NoResponse),
        };
        if matches!(result, Ok(JoybusAction::SwitchPak)) {
            // A shared Pak shortcut goes to the lowest eligible port, regardless of wire order.
            pak_switch_port = Some(port.min(pak_switch_port.unwrap_or(port)));
        }
        cycles += JOYBUS_CHANNEL_SETUP + tx_len as u64 * JOYBUS_BYTE;
        cycles += match result {
            Ok(_) => rx_len as u64 * JOYBUS_BYTE,
            // Handlers report errors before producing any reply bytes.
            Err(_) => JOYBUS_REPLY_TIMEOUT,
        };
        *rx_header = rx_len as u8 | result.err().map_or(0, |error| error as u8);
    }
    device.pif.ram = ram;
    if let Some(port) = pak_switch_port {
        device::controller::start_pak_switch(device, port);
    }
    cycles
}

pub fn write_reply(rx: &mut [u8], reply: &[u8]) {
    // The guest can intentionally receive fewer bytes than the device sends.
    let len = rx.len().min(reply.len());
    rx[..len].copy_from_slice(&reply[..len]);
}

pub fn dma_write_target(pif: &mut Pif, address: u32) -> Option<(usize, &mut [u8])> {
    let offset = address as usize & PIF_MASK & !3;
    // RAM is at the end of the mirrored PIF window; wrapping past it reaches ROM.
    let start = offset.max(PIF_RAM_OFFSET);
    let end = (offset + PIF_RAM_SIZE).min(PIF_MASK + 1);
    if start >= end {
        return None;
    }
    Some((
        start - offset,
        &mut pif.ram[start - PIF_RAM_OFFSET..end - PIF_RAM_OFFSET],
    ))
}

fn packet_lengths(tx: u8, rx: u8) -> (usize, usize) {
    (
        usize::from(tx & JOYBUS_LENGTH_MASK),
        usize::from(rx & JOYBUS_LENGTH_MASK),
    )
}

fn clear_joybus_layout(pif: &mut Pif) {
    for channel in &mut pif.channels {
        channel.header = None;
    }
}

fn setup_channels_format(pif: &mut Pif) {
    clear_joybus_layout(pif);
    let frame = &pif.ram[..PIF_COMMAND];
    let mut offset = 0;
    for channel in &mut pif.channels {
        while frame.get(offset) == Some(&0xff) {
            offset += 1;
        }
        match frame.get(offset) {
            Some(0x00 | 0xfd) => offset += 1,
            Some(0xfe) | None => break,
            Some(&tx) => {
                let Some(&rx) = frame.get(offset + 1) else {
                    break;
                };
                let (tx_len, rx_len) = packet_lengths(tx, rx);
                let end = offset + 2 + tx_len + rx_len;
                if end > frame.len() {
                    break;
                }
                // Flagged skip and reset packets still use their declared lengths.
                channel.header = Some(offset);
                offset = end;
            }
        }
    }
}

pub fn process_ram(device: &mut device::Device) {
    let mut clrmask = 0x00;
    let command = device.pif.ram[PIF_COMMAND];
    if command & PIF_CONFIGURE_JOYBUS != 0 {
        // Configure joybus protocol
        clrmask |= PIF_CONFIGURE_JOYBUS;
        setup_channels_format(&mut device.pif);
    }
    if command & 0x02 != 0 {
        // Challenge / response for protection
        clear_joybus_layout(&mut device.pif);
        process_cic_challenge(device);
        clrmask |= 0x02;
    }
    if command & 0x08 != 0 {
        // Terminate boot process
        clrmask |= 0x08;
    }
    if command & 0x10 != 0 {
        // ROM lockout
        for i in device.pif.rom.iter_mut() {
            *i = 0
        }
    }
    if command & 0x20 != 0 {
        // Acquire checksum
        device.pif.ram[PIF_COMMAND] = 0x80
    }
    device.pif.ram[PIF_COMMAND] &= !clrmask;
}

fn controller_connected(device: &device::Device, port: usize) -> bool {
    if let Some(netplay) = &device.netplay {
        netplay.connected[port]
    } else if let Some(playback) = &device.bk2_playback {
        playback.connected(port)
    } else {
        device.ui.config.input.controller_enabled[port]
    }
}

pub fn vru_enabled(device: &device::Device) -> bool {
    device.ui.config.input.emulate_vru && device.netplay.is_none() && device.bk2_playback.is_none()
}

pub fn connect_pif_channels(device: &mut device::Device) {
    for i in 0..4 {
        if controller_connected(device, i) {
            device.pif.channels[i].process = Some(device::controller::process);
        }
    }
    if vru_enabled(device) {
        device.pif.channels[3].process = Some(device::controller::vru::process);
    }
    device.pif.channels[4].process = Some(device::cart::process)
}

pub fn reset_pif(device: &mut device::Device, is_nmi_reset: bool) {
    if device.cart.pal {
        device.pif.rom = rom::PAL_PIF_ROM;
    } else {
        device.pif.rom = rom::NTSC_PIF_ROM;
    }

    let reset_type = u32::from(is_nmi_reset);
    let rom_type = 0u32;
    let s7 = 0u32;
    let cic = u32::from(device.cart.cic_seed);
    let word = (rom_type << 19) | (s7 << 18) | (reset_type << 17) | (cic << 8) | 0x3f;
    device.pif.ram[0x24..0x28].copy_from_slice(&word.to_be_bytes());
}

pub fn init(device: &mut device::Device) {
    use device::controller::PakType;

    reset_pif(device, false);
    let default_pak = if device.ui.game_id == "NCT" {
        // Chameleon Twist does not support the mempak
        PakType::RumblePak
    } else {
        PakType::MemPak
    };

    connect_pif_channels(device);

    for i in 0..4 {
        if controller_connected(device, i) {
            device.pif.channels[i].pak_type =
                if device.netplay.is_some() || device.transferpaks[i].cart.rom.is_empty() {
                    default_pak
                } else {
                    PakType::TransferPak
                };
        }
    }
    if vru_enabled(device) {
        device.pif.channels[3].pak_type = PakType::None;
    }
}

fn process_cic_challenge(device: &mut device::Device) {
    let mut challenge: [u8; 30] = [0; 30];
    let mut response: [u8; 30] = [0; 30];

    /* format the 'challenge' message into 30 nibbles for X-Scale's CIC code */
    for i in 0..15 {
        challenge[i * 2] = (device.pif.ram[0x30 + i] >> 4) & 0x0f;
        challenge[i * 2 + 1] = device.pif.ram[0x30 + i] & 0x0f;
    }

    /* calculate the proper response for the given challenge (X-Scale's algorithm) */
    n64_cic_nus_6105(challenge, &mut response, CHL_LEN - 2);
    device.pif.ram[0x2e] = 0;
    device.pif.ram[0x2f] = 0;

    /* re-format the 'response' into a byte stream */
    for i in 0..15 {
        device.pif.ram[0x30 + i] = (response[i * 2] << 4) + response[i * 2 + 1];
    }
}

fn n64_cic_nus_6105(chl: [u8; 30], rsp: &mut [u8; 30], len: usize) {
    let lut0: [u8; 0x10] = [
        0x4, 0x7, 0xA, 0x7, 0xE, 0x5, 0xE, 0x1, 0xC, 0xF, 0x8, 0xF, 0x6, 0x3, 0x6, 0x9,
    ];
    let lut1: [u8; 0x10] = [
        0x4, 0x1, 0xA, 0x7, 0xE, 0x5, 0xE, 0x1, 0xC, 0x9, 0x8, 0x5, 0x6, 0x3, 0xC, 0x9,
    ];

    let mut key = 0xB;
    let mut lut = lut0.as_ref();
    for i in 0..len {
        rsp[i] = (key + 5 * chl[i]) & 0xF;
        key = lut[rsp[i] as usize];
        let sgn = (rsp[i] >> 3) & 0x1;
        let mut mag;
        if sgn == 1 {
            mag = !rsp[i]
        } else {
            mag = rsp[i]
        }
        mag &= 0x7;
        let mut modd;
        if mag % 3 == 1 {
            modd = sgn
        } else {
            modd = 1 - sgn;
        }
        if lut == lut1 && (rsp[i] == 0x1 || rsp[i] == 0x9) {
            modd = 1;
        }
        if lut == lut1 && (rsp[i] == 0xB || rsp[i] == 0xE) {
            modd = 0;
        }
        if modd == 1 {
            lut = &lut1;
        } else {
            lut = &lut0;
        }
    }
}
