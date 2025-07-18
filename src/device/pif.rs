mod rom;
use crate::device;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Pif {
    #[serde(with = "serde_big_array::BigArray")]
    pub rom: [u8; 1984],
    #[serde(with = "serde_big_array::BigArray")]
    pub ram: [u8; 64],
    pub channels: [PifChannel; 5],
}

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct PifChannel {
    pub tx: Option<usize>,
    pub tx_buf: Option<usize>,
    pub rx: Option<usize>,
    pub rx_buf: Option<usize>,
    #[serde(skip)]
    pub process: Option<fn(&mut device::Device, usize)>,
    pub pak_handler: Option<device::controller::PakHandler>,
    pub change_pak: device::controller::PakType,
}

pub const PIF_RAM_SIZE: usize = 64;
const PIF_CHANNELS_COUNT: usize = 5;
const PIF_RAM_OFFSET: usize = 0x7C0;
const PIF_MASK: usize = 0xFFFF;
const CHL_LEN: usize = 0x20;

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

    device.si.dma_dir = device::si::DmaDir::Write;
    device::events::create_event(device, device::events::EVENT_TYPE_SI, 3200); //based on https://github.com/rasky/n64-systembench
    device.si.regs[device::si::SI_STATUS_REG as usize] |=
        device::si::SI_STATUS_DMA_BUSY | device::si::SI_STATUS_IO_BUSY
}

fn process_channel(device: &mut device::Device, channel: usize) -> usize {
    /* don't process channel if it has been disabled */
    if device.pif.channels[channel].tx.is_none() {
        return 0;
    }

    /* reset Tx/Rx just in case */
    device.pif.ram[device.pif.channels[channel].tx.unwrap()] &= 0x3f;
    device.pif.ram[device.pif.channels[channel].rx.unwrap()] &= 0x3f;

    /* set NoResponse if no device is connected */
    if device.pif.channels[channel].process.is_none() {
        device.pif.ram[device.pif.channels[channel].rx.unwrap()] |= 0x80;
        return 0;
    }

    /* do device processing */
    let process_handler = device.pif.channels[channel].process.unwrap();
    process_handler(device, channel);
    1
}

pub fn update_pif_ram(device: &mut device::Device) -> u64 {
    let mut active_channels = 0;
    for k in 0..PIF_CHANNELS_COUNT {
        active_channels += process_channel(device, k)
    }

    (24000 + (active_channels * 30000)) as u64
}

fn disable_pif_channel(channel: &mut PifChannel) {
    channel.tx = None;
    channel.rx = None;
    channel.tx_buf = None;
    channel.rx_buf = None;
}

fn setup_pif_channel(device: &mut device::Device, channel: usize, buf: usize) -> usize {
    let tx = device.pif.ram[buf] & 0x3f;
    let rx = device.pif.ram[buf + 1] & 0x3f;

    /* XXX: check out of bounds accesses */

    device.pif.channels[channel].tx = Some(buf);
    device.pif.channels[channel].rx = Some(buf + 1);
    device.pif.channels[channel].tx_buf = Some(buf + 2);
    device.pif.channels[channel].rx_buf = Some(buf + 2 + tx as usize);

    (2 + tx + rx) as usize
}

fn setup_channels_format(device: &mut device::Device) {
    let mut i: usize = 0;
    let mut k: usize = 0;
    while i < PIF_RAM_SIZE && k < PIF_CHANNELS_COUNT {
        match device.pif.ram[i] {
            0x00 => {
                /* skip channel */
                disable_pif_channel(&mut device.pif.channels[k]);
                k += 1;
                i += 1;
            }

            0xff => {
                /* dummy data */
                i += 1;
            }

            0xfe => {
                /* end of channel setup - remaining channels are disabled */
                while k < PIF_CHANNELS_COUNT {
                    disable_pif_channel(&mut device.pif.channels[k]);
                    k += 1;
                }
            }

            0xfd => {
                /* channel reset - send reset command and discard the results */
                disable_pif_channel(&mut device.pif.channels[k]); // not sure about this
                k += 1;
                i += 1;
            }

            _ => {
                /* setup channel */

                /* HACK?: some games sends bogus PIF commands while accessing controller paks
                 * Yoshi Story, Top Gear Rally 2, Indiana Jones, ...
                 * When encountering such commands, we skip this bogus byte.
                 */
                if (i + 1 < PIF_RAM_SIZE) && (device.pif.ram[i + 1] == 0xfe) {
                    i += 1;
                    continue;
                }

                if (i + 2) >= PIF_RAM_SIZE {
                    i = PIF_RAM_SIZE;
                    continue;
                }

                i += setup_pif_channel(device, k, i);
                k += 1;
            }
        }
    }
}

pub fn process_ram(device: &mut device::Device) {
    let mut clrmask = 0x00;
    let command = device.pif.ram[0x3F];
    if command & 0x01 != 0 {
        // Configure joybus protocol
        setup_channels_format(device);
        clrmask |= 0x01
    }
    if command & 0x02 != 0 {
        // Challenge / response for protection
        /* disable channel processing when doing CIC challenge */
        for k in 0..PIF_CHANNELS_COUNT {
            disable_pif_channel(&mut device.pif.channels[k]);
        }

        /* CIC Challenge */
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
        device.pif.ram[0x3F] = 0x80
    }
    device.pif.ram[0x3F] &= !clrmask
}

pub fn connect_pif_channels(device: &mut device::Device) {
    for i in 0..4 {
        if device.netplay.is_none() {
            if device.ui.config.input.controller_enabled[i] {
                device.pif.channels[i].process = Some(device::controller::process);
            }
        } else if device.netplay.as_ref().unwrap().player_data[i].reg_id != 0 {
            device.pif.channels[i].process = Some(device::controller::process);
        }
    }
    if device.ui.config.input.emulate_vru && device.netplay.is_none() {
        device.pif.channels[3].process = Some(device::controller::vru::process);
    }
    device.pif.channels[4].process = Some(device::cart::process)
}

fn get_default_handler(device: &device::Device) -> device::controller::PakHandler {
    if device.ui.game_id == "NCT" {
        // Chameleon Twist does not support the mempak
        device::controller::PakHandler {
            read: device::controller::rumble::read,
            write: device::controller::rumble::write,
            pak_type: device::controller::PakType::RumblePak,
        }
    } else {
        device::controller::PakHandler {
            read: device::controller::mempak::read,
            write: device::controller::mempak::write,
            pak_type: device::controller::PakType::MemPak,
        }
    }
}

pub fn init(device: &mut device::Device) {
    if device.cart.pal {
        device.pif.rom = rom::PAL_PIF_ROM;
    } else {
        device.pif.rom = rom::NTSC_PIF_ROM;
    }
    device.pif.ram[0x26] = device.cart.cic_seed;
    device.pif.ram[0x27] = device.cart.cic_seed;

    let default_handler = get_default_handler(device);
    let tpak_handler = device::controller::PakHandler {
        read: device::controller::transferpak::read,
        write: device::controller::transferpak::write,
        pak_type: device::controller::PakType::TransferPak,
    };

    connect_pif_channels(device);

    for i in 0..4 {
        if device.netplay.is_none() {
            if device.ui.config.input.controller_enabled[i] {
                if device.transferpaks[i].cart.rom.is_empty() {
                    device.pif.channels[i].pak_handler = Some(default_handler);
                } else {
                    device.pif.channels[i].pak_handler = Some(tpak_handler);
                }
            }
        } else if device.netplay.as_ref().unwrap().player_data[i].reg_id != 0 {
            device.pif.channels[i].pak_handler = Some(default_handler);
        }
    }
    if device.ui.config.input.emulate_vru && device.netplay.is_none() {
        device.pif.channels[3].pak_handler = None;
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
