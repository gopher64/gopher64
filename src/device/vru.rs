use crate::device;
use std::collections::HashMap;

const JCMD_VRU_READ: u8 = 0x09;
const JCMD_VRU_WRITE: u8 = 0x0A;
const JCMD_VRU_READ_STATUS: u8 = 0x0B;
const JCMD_VRU_WRITE_CONFIG: u8 = 0x0C;
const JCMD_VRU_WRITE_INIT: u8 = 0x0D;

const VOICE_STATUS_READY: u8 = 0x00;
const VOICE_STATUS_START: u8 = 0x01;
//const VOICE_STATUS_CANCEL: u8 = 0x03;
const VOICE_STATUS_BUSY: u8 = 0x05;
//const VOICE_STATUS_END: u8 = 0x07;

const JDT_VRU: u16 = 0x0100; /* VRU */
const CONT_FLAVOR: u16 = JDT_VRU;

pub struct Vru {
    pub status: u8,
    pub voice_state: u8,
    pub load_offset: u8,
    pub voice_init: u8,
    pub word_buffer: [u16; 40],
    pub words: Vec<String>,
    pub talking: bool,
    pub word_mappings: HashMap<String, String>,
}

pub fn reset_vru(device: &mut device::Device) {
    device.vru.status = 0x00;
    if device.cart.rom[0x3E] == /* Japan */ 0x4A || device.cart.rom[0x3E] == /* Demo */ 0x00 {
        device.vru.voice_state = VOICE_STATUS_READY;
    } else {
        device.vru.voice_state = VOICE_STATUS_START;
    }
    device.vru.load_offset = 0;
    device.vru.voice_init = 1;
    device.vru.word_buffer = [0; 40];
}

fn set_status(device: &mut device::Device, channel: usize) {
    if device.vru.voice_init == 2 {
        /* words have been loaded, we can change the state from READY to START */
        device.vru.voice_state = VOICE_STATUS_START;
        device.vru.voice_init = 1;
    } else if device.vru.talking && (device.vru.voice_state == VOICE_STATUS_START) {
        /* On Densha de Go, if the player is talking for more than ~2.5 seconds, the input is ignored */
        device.vru.voice_state = VOICE_STATUS_BUSY;
        device.vru.status = 0; /* setting the status to 0 tells the game to check the voice_status */
    } else if !device.vru.talking && (device.vru.voice_state == VOICE_STATUS_BUSY) {
        device.vru.voice_state = VOICE_STATUS_READY;
        device.vru.status = 0; /* setting the status to 0 tells the game to check the voice_status */
    }

    device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = CONT_FLAVOR as u8;
    device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 1] = (CONT_FLAVOR >> 8) as u8;
    device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 2] = device.vru.status;
}

pub fn process(device: &mut device::Device, channel: usize) {
    let cmd = device.pif.ram[device.pif.channels[channel].tx_buf.unwrap()];

    match cmd {
        device::controller::JCMD_RESET => {
            reset_vru(device);

            set_status(device, channel);
        }
        device::controller::JCMD_STATUS => {
            set_status(device, channel);
        }
        device::controller::JCMD_CONTROLLER_READ => {}
        JCMD_VRU_READ_STATUS => {
            if device.vru.voice_init > 0 {
                device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] =
                    device.vru.voice_state;
            } else {
                device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = 0;
            }
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 1] = 0;
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 2] =
                device::controller::data_crc(
                    device,
                    device.pif.channels[channel].rx_buf.unwrap(),
                    2,
                );

            if device.vru.load_offset > 0 {
                let mut offset = 0;
                while device.vru.word_buffer[offset] == 0 && offset < 40 {
                    offset += 1;
                }
                if offset == 40 {
                    panic!("Empty JCMD_VRU_READ_STATUS.");
                } else if device.vru.word_buffer[offset] == 3 {
                    offset += 3;
                    let mut length = device.vru.word_buffer[offset];
                    if device.cart.rom[0x3E] == /* Japan */ 0x4A
                        || device.cart.rom[0x3E] == /* Demo */ 0x00
                    {
                        offset -= 1;
                        length = 0;
                        while device.vru.word_buffer[offset + length as usize] != 0 {
                            length += 1;
                        }
                        if device.cart.rom[0x3E] == /* Japan */ 0x4A {
                            let mut data = Vec::new();
                            for i in 0..length {
                                data.extend(
                                    device.vru.word_buffer[offset + i as usize].to_be_bytes(),
                                );
                            }
                            let (res, _enc, errors) = encoding_rs::SHIFT_JIS.decode(&data);
                            if errors {
                                panic!("Failed to decode Japanese word {:X?}", data);
                            } else {
                                device.vru.words.push(res.to_string());
                            }
                        } else {
                            panic!("Unknown VRU region")
                        }
                    } else {
                        offset += 1;

                        let mut data = String::new();
                        for i in 0..length {
                            data.push_str(&format!(
                                "{:04X}",
                                device.vru.word_buffer[offset + i as usize]
                            ))
                        }
                        let word = device.vru.word_mappings.get(&data);
                        if word.is_some() {
                            device.vru.words.push(word.unwrap().clone());
                        } else {
                            panic!("Unknown VRU word {}", data);
                        }
                    }
                } else {
                    panic!("Unknown command in JCMD_VRU_READ_STATUS.");
                }
                device.vru.load_offset = 0;
            }
            device.vru.status = 1;
        }
        JCMD_VRU_WRITE_CONFIG => {
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] =
                device::controller::data_crc(
                    device,
                    device.pif.channels[channel].tx_buf.unwrap() + 3,
                    4,
                );
            if device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] == 0x4E {
                device.vru.talking = true;
                device.vru.voice_init = 2;
                device::events::create_event(
                    device,
                    device::events::EventType::Vru,
                    device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize]
                        + (device.cpu.clock_rate * 2), // 2 seconds
                    vru_talking_event,
                )
            } else if device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] == 0xEF {
                device.vru.talking = false;
                device::events::remove_event(device, device::events::EventType::Vru);
            } else if device.pif.ram[device.pif.channels[channel].tx_buf.unwrap() + 3] == 0x2 {
                device.vru.voice_init = 0;
                device.vru.words.clear();
            }
            device.vru.status = 0; /* status is always set to 0 after a write */
        }
        JCMD_VRU_WRITE_INIT => {
            let offset = device.pif.channels[channel].tx_buf.unwrap() + 1;
            if u16::from_ne_bytes(device.pif.ram[offset..offset + 2].try_into().unwrap()) == 0 {
                device.vru.talking = false;
                device::events::remove_event(device, device::events::EventType::Vru);
            }
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = 0;
        }
        JCMD_VRU_READ => {
            panic! {"JCMD_VRU_READ"}
        }
        JCMD_VRU_WRITE => {
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] =
                device::controller::data_crc(
                    device,
                    device.pif.channels[channel].tx_buf.unwrap() + 3,
                    20,
                );
            if device.vru.load_offset == 0 {
                device.vru.word_buffer = [0; 40];
            }
            for i in 0..10 {
                let offset = device.pif.channels[channel].tx_buf.unwrap() + 3 + (i * 2);
                device.vru.word_buffer[device.vru.load_offset as usize] =
                    u16::from_ne_bytes(device.pif.ram[offset..offset + 2].try_into().unwrap());
                device.vru.load_offset += 1;
            }
            device.vru.status = 0; /* status is always set to 0 after a write */
        }
        _ => panic!("unknown VRU command {}", cmd),
    }
}

pub fn create_word_mappings(device: &mut device::Device) {
    device.vru.word_mappings = HashMap::from([
        (
            String::from("03A50024000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03A50045000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
    ]);
}

pub fn vru_talking_event(device: &mut device::Device) {
    device.vru.talking = false
}
