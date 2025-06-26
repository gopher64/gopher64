use crate::{device, ui};
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Vru {
    pub status: u8,
    pub voice_state: u8,
    pub load_offset: u8,
    pub voice_init: u8,
    #[serde(with = "serde_big_array::BigArray")]
    pub word_buffer: [u16; 40],
    pub words: Vec<String>,
    pub talking: bool,
    pub word_mappings: HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VruWindow {
    #[serde(skip)]
    pub window_notifier: Option<tokio::sync::mpsc::Sender<Option<Vec<String>>>>,
    #[serde(skip)]
    pub word_receiver: Option<tokio::sync::mpsc::Receiver<String>>,
}

pub fn init(device: &mut device::Device) {
    reset_vru(device);
    create_word_mappings(device);
}

fn reset_vru(device: &mut device::Device) {
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
                        let mut data = Vec::new();
                        for i in 0..length {
                            data.extend(device.vru.word_buffer[offset + i as usize].to_be_bytes());
                        }

                        let (res, _enc, errors) = encoding_rs::SHIFT_JIS.decode(&data);
                        if errors {
                            panic!("Failed to decode Japanese word {data:X?}");
                        } else {
                            device.vru.words.push(res.to_string());
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
                        if let Some(result) = word {
                            device.vru.words.push(result.clone());
                        } else {
                            panic!("Unknown VRU word {data}");
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
                    device::events::EVENT_TYPE_VRU,
                    device.cpu.clock_rate * 2, // 2 seconds
                )
            } else if device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] == 0xEF {
                device.vru.talking = false;
                device::events::remove_event(device, device::events::EVENT_TYPE_VRU);
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
                device::events::remove_event(device, device::events::EVENT_TYPE_VRU);
            }
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = 0;
        }
        JCMD_VRU_READ => {
            let index = if device.vru_window.window_notifier.is_some() {
                ui::vru::prompt_for_match(
                    &device.vru.words,
                    device.vru_window.window_notifier.as_ref().unwrap(),
                    device.vru_window.word_receiver.as_mut().unwrap(),
                )
            } else {
                0x7FFF
            };
            let num_results = if index == 0x7FFF { 0 } else { 1 };
            let data: HashMap<usize, u16> = HashMap::from([
                (0, 0x8000),
                (2, 0x0F00),
                (4, 0),           // error flags
                (6, num_results), // number of results
                (8, 0xBB8),       // mic level
                (10, 0xBB8),      // voice level
                (12, 0x8004),     // voice length
                (14, index),      // match 1
                (16, 0),          // match 1 errors
                (18, 0x7FFF),     // match 2
                (20, 0),          // match 2 errors
                (22, 0x7FFF),     // match 3
                (24, 0),          // match 3 errors
                (26, 0x7FFF),     // match 4
                (28, 0),          // match 4 errors
                (30, 0x7FFF),     // match 5
                (32, 0),          // match 5 errors
                (34, 0x0040),
            ]);
            for (key, value) in data {
                let offset = device.pif.channels[channel].rx_buf.unwrap() + key;
                device.pif.ram[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
            }

            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 36] =
                device::controller::data_crc(
                    device,
                    device.pif.channels[channel].rx_buf.unwrap(),
                    36,
                );

            device.vru.voice_state = VOICE_STATUS_START;
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
        _ => panic!("unknown VRU command {cmd}"),
    }
}

pub fn vru_talking_event(device: &mut device::Device) {
    device.vru.talking = false
}

fn create_word_mappings(device: &mut device::Device) {
    device.vru.word_mappings = HashMap::from([
        (
            String::from("03A50024000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03A50045000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03A50024000303C900450003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03A8018F000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03B101B0000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03A80066000303CF00A80003036000F900EA"),
            String::from("pikachu"),
        ),
        (
            String::from("03A50024000303CF00A80003035D001200F900EA"),
            String::from("pikachu"),
        ),
        (String::from("040801740024"), String::from("hey!")),
        (
            String::from("03CF00C603360405000F0234"),
            String::from("come-here"),
        ),
        (
            String::from("0369004803FC0318018F"),
            String::from("this-way"),
        ),
        (
            String::from("039C010B000603900006037B01B0"),
            String::from("good-bye"),
        ),
        (
            String::from("039900A80006037B01B0"),
            String::from("good-bye"),
        ),
        (
            String::from("03F900090309009F02E2018F000303BD0234"),
            String::from("see-you-later"),
        ),
        (
            String::from("037B01B00006037B01B0"),
            String::from("bye-bye"),
        ),
        (
            String::from("03FC000303C30255000303C6"),
            String::from("start"),
        ),
        (
            String::from("02E2006903FC000303B402E2018F"),
            String::from("lets-play"),
        ),
        (String::from("040B00C002EB0213"), String::from("hello")),
        (
            String::from("043B0213000303AB00A5034803FC006903FC00A503270024"),
            String::from("open-sesame"),
        ),
        (
            String::from("043B0213000303A80063034803FC006903FC00A503270024"),
            String::from("open-sesame"),
        ),
        (
            String::from("039F01FE00990318018F"),
            String::from("go-away"),
        ),
        (
            String::from("039C010B00060390033302D603390042035A"),
            String::from("good-morning"),
        ),
        (
            String::from("039900A5033302D603390042035A"),
            String::from("good-morning"),
        ),
        (
            String::from("042F01AD033603FC023A0024"),
            String::from("im-sorry"),
        ),
        (
            String::from("042F01AD033603FC012602F10024"),
            String::from("im-sorry"),
        ),
        (String::from("03FC023A0024"), String::from("sorry")),
        (String::from("03FC012602F10024"), String::from("sorry")),
        (
            String::from("042F01B00003036000ED03DB030C00EA"),
            String::from("i-choose-you"),
        ),
        (
            String::from("04080174030C00EA000303A50024000303CF00A80003036000EA"),
            String::from("hey-you-pikachu"),
        ),
        (
            String::from("03A50024000303CF00A8000303A50024000303CF00A80003036000EA"),
            String::from("pika-pikachu"),
        ),
        (
            String::from("03A50024000303CF00A8000303A50024000303CF00A8000303A50024"),
            String::from("pika-pika-pi"),
        ),
        (
            String::from("03A50024000303A50024000303CF00A80003036000EA"),
            String::from("pi-pikachu"),
        ),
        (
            String::from("03A50024000303A50024000303CF00A8000303A50024"),
            String::from("pi-pika-pi"),
        ),
        (
            String::from("03A50024000303D8000303CF00C9"),
            String::from("pikka"),
        ),
        (String::from("03A50024000303CF00A8"), String::from("pika")),
        (
            String::from("03A50024000303CF00A8000303A50024000303CF00A8"),
            String::from("pikapika"),
        ),
        (
            String::from("03A503030024000303CF00A8"),
            String::from("pi-ka"),
        ),
        (
            String::from("03A50024000303CF00C9000303CF00C9"),
            String::from("pi-ka-ka-"),
        ),
        (
            String::from("03A50024000303CF00A8000303CF00A8000303A50024"),
            String::from("pi-kakapi-"),
        ),
        (
            String::from("037E02F10042035A036C0087000303C604050297"),
            String::from("bring-that-here"),
        ),
        (
            String::from("039F0213000603960066000303B70045000303C6"),
            String::from("go-get-it"),
        ),
        (
            String::from("0393004803E70045000303C6000303C000E703270024"),
            String::from("give-it-to-me"),
        ),
        (
            String::from("0393004203270027036C0087000303C6"),
            String::from("gimme-that"),
        ),
        (
            String::from("03F600C603480006038702370402014D000303D8"),
            String::from("thundershock"),
        ),
        (
            String::from("03F600C603480006038702340006037B020A02EE000303C6"),
            String::from("thunderbolt"),
        ),
        (
            String::from("03F600C60348000603870234"),
            String::from("thunder"),
        ),
        (String::from("03F600C6033F0234"), String::from("thunder")),
        (
            String::from("0432009F02E20066000303D8000303C602F10045000303D802F40087000303C6"),
            String::from("electric-rat"),
        ),
        (
            String::from("0426001B02E20066000303D8000303C602F10045000303D802F40087000303C6"),
            String::from("electric-rat"),
        ),
        (
            String::from("0432009F02E20066000303C602F10045000303D802F40087000303C6"),
            String::from("electric-rat"),
        ),
        (
            String::from("042F01A40408018F000303B7030C00EA"),
            String::from("i-hate-you"),
        ),
        (
            String::from("03A202F4018F000303C600060366014D0006037E"),
            String::from("great-job"),
        ),
        (
            String::from("036C0087000303C3031B00AB03DE000603A202F4018F000303C6"),
            String::from("that-was-great"),
        ),
        (
            String::from("0309023703FC0213000303C9030C00EA000303C6"),
            String::from("youre-so-cute"),
        ),
        (
            String::from("03BD00A202F1004803ED0045000303D8"),
            String::from("terrific"),
        ),
        (
            String::from("037500C9000303BD023703F002F10024"),
            String::from("butterfree"),
        ),
        (
            String::from("043200C603360006037E02F4005D02E500A8"),
            String::from("ambrella"),
        ),
        (
            String::from("037B01CE035A0006037B01CE035A"),
            String::from("boing-boing"),
        ),
        (
            String::from(
                "043801290348000303C000ED036C00A5033C0066000303D803FC000303C603FC000303BA018F00060366",
            ),
            String::from("on-to-the-next-stage!"),
        ),
        (
            String::from("042F0087000303AB009F02EE"),
            String::from("apple"),
        ),
        (
            String::from("043200A8000303AB009F02EE"),
            String::from("apple"),
        ),
        (
            String::from("0426000C0087000303B402EE"),
            String::from("apple"),
        ),
        (
            String::from("03A202F10021033C0087000303AB009F02EE"),
            String::from("green-apple"),
        ),
        (
            String::from("0372018F000303D8000303BA0087000303AB009F02EE"),
            String::from("baked-apple"),
        ),
        (
            String::from("039F020A02EE0006038700A5033C0087000303AB009F02EE"),
            String::from("golden-apple"),
        ),
        (String::from("03A5002400030360"), String::from("peach")),
        (
            String::from("03FC000303C602FD012C00060372029D0024"),
            String::from("strawberry"),
        ),
        (
            String::from("03FC000303BD0225012C00060372029D0024"),
            String::from("strawberry"),
        ),
        (
            String::from("02F4008A03DE00060372029D0024"),
            String::from("raspberry"),
        ),
        (
            String::from("02F4008A03FC0006037E02F10024"),
            String::from("raspberry"),
        ),
        (
            String::from("037500A5033C0084033F00A8"),
            String::from("banana"),
        ),
        (
            String::from("036F0042033C0084033F00C9"),
            String::from("banana"),
        ),
        (
            String::from("037500C6033C0084033F00A8"),
            String::from("banana"),
        ),
        (
            String::from("03CC0087000303C6000303BA018602EE"),
            String::from("cattail"),
        ),
        (
            String::from("032101A702EE00060390040B02340006037E"),
            String::from("wild-herb"),
        ),
        (String::from("03D502D60348"), String::from("corn")),
        (
            String::from("03B1014D000303B4000303D502D60348"),
            String::from("popcorn"),
        ),
        (
            String::from("03AB00C60336000303B4000303C900420348"),
            String::from("pumpkin"),
        ),
        (
            String::from("03AB00C60336000303C900420348"),
            String::from("pumpkin"),
        ),
        (
            String::from("03AB00C6035A000303CF00A50348"),
            String::from("pumpkin"),
        ),
        (
            String::from("03C00273033F00A8000303B4"),
            String::from("turnip"),
        ),
        (
            String::from("03C0027303390045000303B4"),
            String::from("turnip"),
        ),
        (
            String::from("03CC00870006036F004500060366"),
            String::from("cabbage"),
        ),
        (
            String::from("03CC008102F700A8000303C6"),
            String::from("carrot"),
        ),
        (String::from("03CC02A300A8000303C6"), String::from("carrot")),
        (
            String::from("043200C60339030900A50348"),
            String::from("onion"),
        ),
        (String::from("043200A5033F00A50348"), String::from("onion")),
        (
            String::from("043501080339030900A50348"),
            String::from("onion"),
        ),
        (
            String::from("03FC031500240006037500C9000303BA018F0006038D0213"),
            String::from("sweet-potato"),
        ),
        (
            String::from("03FC03150024000303C6000303AB00A8000303C3016E0006038D0213"),
            String::from("sweet-potato"),
        ),
        (
            String::from("03FC03150024000303B4000303BA018F0006038700A8"),
            String::from("sweet-potato"),
        ),
        (
            String::from(
                "0411014D000303C603FC03150024000303C6000303AB00C9000303BA018F0006038D0213",
            ),
            String::from("hot-sweet-potato"),
        ),
        (
            String::from("03FC032A005D02DF002703F002FA00EA000303C6"),
            String::from("smelly-fruit"),
        ),
        (
            String::from("03FC000303B70045000303C9002703F002FA00EA000303C6"),
            String::from("sticky-fruit"),
        ),
        (
            String::from("02FD021603FC000303BD00A80006039003F002FA00EA000303C6"),
            String::from("roasted-fruit"),
        ),
        (
            String::from("0321012302EE033F00A8000303C6"),
            String::from("walnut"),
        ),
        (
            String::from("03D802F40087000303D50321012302EE033F00A8000303C6"),
            String::from("cracked-walnut"),
        ),
        (
            String::from("03FC000303B101B0000303C900270402005D02EE"),
            String::from("spiky-shell"),
        ),
        (
            String::from("03FC000303B101AD033900270402005D02EE"),
            String::from("spiny-shell"),
        ),
        (
            String::from("0360006903FC033F00A8000303C6"),
            String::from("chestnut"),
        ),
        (
            String::from("036F002400030360033F00C9000303C6"),
            String::from("beach-nut"),
        ),
        (
            String::from("03D50213000303CF00A5033F00C9000303C6"),
            String::from("coconut"),
        ),
        (
            String::from("042C018F000303D502D60348"),
            String::from("acorn"),
        ),
        (
            String::from("03C3021603FC000303BD00A800060384018F000303D502D60348"),
            String::from("toasted-acorn"),
        ),
        (
            String::from("03F002EB01950042034E018F000303D502D60348"),
            String::from("flying-acorn"),
        ),
        (
            String::from("042C018F000303D502D60348000303C3014D000303B4"),
            String::from("acorn-top"),
        ),
        (
            String::from("036600C603360006037B0216040202FA00E70336"),
            String::from("jumbo-shroom"),
        ),
        (
            String::from("02FD021603FC000303BD00A8000603900006036600A503360006037B0213"),
            String::from("roasted-jumbo"),
        ),
        (
            String::from("039002F1002103270027040202FA00E70336"),
            String::from("dreamy-shroom"),
        ),
        (
            String::from("03C602F700CC03F0009F02EE000303C3014D000303B4"),
            String::from("truffle-top"),
        ),
        (
            String::from("037E02E800ED040202FA00E70336"),
            String::from("blue-shroom"),
        ),
        (
            String::from("0411014D000303C6040202FA00E70336"),
            String::from("hot-shroom"),
        ),
        (
            String::from("02F4006600060390040202FA00E70336"),
            String::from("red-shroom"),
        ),
        (
            String::from("03FC00C603390027040202FA00E70336"),
            String::from("sunny-shroom"),
        ),
        (
            String::from("032D00CC040202FA00E70336"),
            String::from("mushroom"),
        ),
        (
            String::from("0411014D000303C6032D00CC040202FA00E70336"),
            String::from("hot-mushroom"),
        ),
        (
            String::from("02EB01B0000303C60006037500C002EE0006037E"),
            String::from("light-bulb"),
        ),
        (
            String::from("02EB01B0000303C600060378010202EE0006037E"),
            String::from("light-bulb"),
        ),
        (
            String::from("03CF00C9000303B4000303CC018F000303D8"),
            String::from("cupcake"),
        ),
        (String::from("03840174002703DB0024"), String::from("daisy")),
        (
            String::from("03A50045000303C000E70339030900A8"),
            String::from("petunia"),
        ),
        (
            String::from("03C000E102E500A8000303B4"),
            String::from("tulip"),
        ),
        (
            String::from("03C000E102DF0045000303B4"),
            String::from("tulip"),
        ),
        (
            String::from("03840084034800060387009F02EB019B00A50348"),
            String::from("dandelion"),
        ),
        (
            String::from("03840084034800060381001B02EB019B00A50348"),
            String::from("dandelion"),
        ),
        (
            String::from("037E02E800EA00060372005D02EE"),
            String::from("bluebell"),
        ),
        (
            String::from("03FC000303CF00C60348000303D80006037E02EB015003FC00A50336"),
            String::from("skunk-blossom"),
        ),
        (
            String::from("03FC00C6034803F002EB01DD0234"),
            String::from("sunflower"),
        ),
        (String::from("02DF003C02DF0024"), String::from("lily")),
        (
            String::from("02FD021603DE0006037500C900060390"),
            String::from("rose-bud"),
        ),
        (
            String::from("02F400660006039002FD021603DE"),
            String::from("red-rose"),
        ),
        (
            String::from("0375009F02E800E70348"),
            String::from("balloon"),
        ),
        (
            String::from("03B10213000303CC018F00060381004803FC000303D8"),
            String::from("poke-disc"),
        ),
        (
            String::from("036F0024000303600006037B012302EE"),
            String::from("beach-ball"),
        ),
        (
            String::from("0321012C000303BD02190042035A000303CC00840348"),
            String::from("watering-can"),
        ),
        (
            String::from("0321012C000303BD02190042035A0006036600C9000603A2"),
            String::from("watering-jug"),
        ),
        (
            String::from("0306005D02EB02130006037B012302EE"),
            String::from("yellow-ball"),
        ),
        (
            String::from("03600255000303D5020A02EE"),
            String::from("charcoal"),
        ),
        (
            String::from("02DF002703F004110315004803FC009F02EE"),
            String::from("leaf-whistle"),
        ),
        (
            String::from("041102520333014A03390045000303CF00A8"),
            String::from("harmonica"),
        ),
        (
            String::from("03C602F700C60336000303A80066"),
            String::from("trumpet"),
        ),
        (
            String::from("03C602FA01080336000303AB00A8000303C6"),
            String::from("trumpet"),
        ),
        (
            String::from("037500A5033C0084033F00A8000303A5000F009F02EE"),
            String::from("banana-peel"),
        ),
        (
            String::from("032A018F0006039900AB03F002100348"),
            String::from("megaphone"),
        ),
        (
            String::from("039F020A02EE00060390000303D501CE0348"),
            String::from("gold-coin"),
        ),
        (
            String::from("0360014D000303CF009F02E500A8000303C6000303D501CE0348"),
            String::from("chocolate-coin"),
        ),
        (
            String::from("0402014D000303D802E500A8000303C6000303D501CE0348"),
            String::from("chocolate-coin"),
        ),
        (
            String::from("03F9003C02EE03EA0234000303D501CE0348"),
            String::from("silver-coin"),
        ),
        (
            String::from("03D5014D000303AB0234000303D501CE0348"),
            String::from("copper-coin"),
        ),
        (String::from("02FA00EA0006036F0024"), String::from("ruby")),
        (
            String::from("03FC008A03F001AA0300"),
            String::from("sapphire"),
        ),
        (
            String::from("03FC008A03F0019B0234"),
            String::from("sapphire"),
        ),
        (
            String::from("03C30213000303A8008A03DE"),
            String::from("topaz"),
        ),
        (
            String::from("02F40066000603900333025500060375009F02EE"),
            String::from("red-marble"),
        ),
        (
            String::from("037E02E800E70333025500060375009F02EE"),
            String::from("blue-marble"),
        ),
        (
            String::from("0306005D02EB02100333025500060375009F02EE"),
            String::from("yellow-marble"),
        ),
        (
            String::from("02FA00EA0006036F001E02F10042035A"),
            String::from("ruby-ring"),
        ),
        (
            String::from("037B014D00060387009F02EE000303CC0087000303B402F10042035A"),
            String::from("bottle-cap-ring"),
        ),
        (
            String::from("03B101AA02F700A8000303C603FC02D900060390"),
            String::from("pirate-sword"),
        ),
        (
            String::from("03B101AA02F10045000303C603FC02D900060390"),
            String::from("pirate-sword"),
        ),
        (
            String::from("03C301D403FC0126030000060390"),
            String::from("toy-sword"),
        ),
        (
            String::from("037502340006039003F00069036C0234"),
            String::from("bird-feather"),
        ),
        (String::from("037B020A02EE000303C6"), String::from("bolt")),
        (
            String::from("032A0087000603A203390045000303C6"),
            String::from("magnet"),
        ),
        (
            String::from("032A018F000603A203390045000303C6"),
            String::from("magnet"),
        ),
        (
            String::from("0333025500060375009F02EE"),
            String::from("marble"),
        ),
        (String::from("03D802F4018F000303C6"), String::from("crate")),
        (
            String::from("03D50255000603900006037B02D9000603900006037B014D000303D803FC"),
            String::from("cardboard-box"),
        ),
        (
            String::from("03CF00A503360405000F0234"),
            String::from("come-here"),
        ),
        (
            String::from("043B021603EA022804050297"),
            String::from("over-here"),
        ),
        (String::from("040B009F02EB0213"), String::from("hello")),
        (
            String::from("039C010B00060390034501950024000303C6"),
            String::from("good-night"),
        ),
        (
            String::from("039900A5034501B0000303C6"),
            String::from("good-night"),
        ),
        (
            String::from("03F6018C035A000303C9030C00EA"),
            String::from("thank-you"),
        ),
        (
            String::from("038A00ED03FC00C6033603F30042035A"),
            String::from("do-something"),
        ),
        (
            String::from("02DF004803FC00A50348000303BD00A503270024"),
            String::from("listen-to-me"),
        ),
        (
            String::from("03A500090024000303CF00A8"),
            String::from("pi-ka"),
        ),
        (
            String::from("039F0213000603930045000303BD00A8000303C6"),
            String::from("go-get-it"),
        ),
        (
            String::from("036C0087000303C603FC03F001AD0348"),
            String::from("thats-fine"),
        ),
        (String::from("043B0213000303CC01740024"), String::from("ok")),
        (String::from("04020276"), String::from("sure")),
        (String::from("02FD01B0000303C6"), String::from("right")),
        (
            String::from("02E2006903FC0006038A00ED036C0087000303C6"),
            String::from("lets-do-that"),
        ),
        (
            String::from("0345014D000303C6036C0087000303C3031B00C60348"),
            String::from("not-that-one"),
        ),
        (
            String::from("036C0087000303C603DE02FD0129035A"),
            String::from("thats-wrong"),
        ),
        (
            String::from("0372008700060390000303B10213000303CC018C0333014A0348"),
            String::from("bad-pokemon"),
        ),
        (
            String::from("038D02100348000303C60006038A00ED036C0087000303C6"),
            String::from("dont-do-that"),
        ),
        (
            String::from("042900420348000303CF021C0066000303D8000303C6"),
            String::from("incorrect"),
        ),
        (
            String::from("03CF00C9000303BD00A8000303C301F2000303C6"),
            String::from("cut-it-out"),
        ),
        (String::from("03450213"), String::from("no")),
        (
            String::from("0318018F0006038700A80006039F0213"),
            String::from("way-to-go"),
        ),
        (String::from("03F002EB01DD0234"), String::from("flower")),
        (
            String::from("038400840348000303C603FC"),
            String::from("dance"),
        ),
        (
            String::from("03F602FD01F80045000303C6"),
            String::from("throw-it"),
        ),
        (
            String::from("03C3015003F90045000303C6"),
            String::from("toss-it"),
        ),
        (
            String::from("030C00EA000303CC008403390024000303C6036C0087000303C6"),
            String::from("you-can-eat-that"),
        ),
        (
            String::from("03BA019203FC000303B70045000303C6"),
            String::from("taste-it"),
        ),
        (
            String::from("041101F503DB0045000303C6000303BA019203FC000303C6"),
            String::from("hows-it-taste?"),
        ),
        (
            String::from("041101F503DB0045000303C603FC032A005D02EE"),
            String::from("hows-it-smell?"),
        ),
        (
            String::from("041101F503DB0045000303C603FC01EF034800060390"),
            String::from("hows-it-sound?"),
        ),
        (
            String::from("0411031B00A8000303C60006038700AB03DB0045000303C60006038A00EA"),
            String::from("what-does-it-do?"),
        ),
        (
            String::from("03B402E201830411031B00A8000303C6"),
            String::from("play-what?"),
        ),
        (
            String::from("03C602FD01950045000303C6"),
            String::from("try-it"),
        ),
        (
            String::from("03B402E201740045000303C6"),
            String::from("play-it"),
        ),
        (
            String::from("039002FD014D000303A50045000303C6"),
            String::from("drop-it"),
        ),
        (
            String::from("03A80066000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (
            String::from("030C00EA000303CC00840348000303BA019203FC000303B70045000303C6"),
            String::from("you-can-taste-it"),
        ),
        (
            String::from("041101F503DB0045000303C603FC01EF0348"),
            String::from("hows-it-sound?"),
        ),
        (
            String::from("03AE010B000303C6036C0087000303C6000603720087000303D8"),
            String::from("put-that-back"),
        ),
        (
            String::from("03FC000303BA018003150048036C03270024"),
            String::from("stay-with-me"),
        ),
        (
            String::from("03FC000303BA01770087000303C6033301A4041101F503FC"),
            String::from("stay-at-my-house"),
        ),
        (
            String::from("03F90009030C00E102E2018F000303BD0234"),
            String::from("see-you-later"),
        ),
        (String::from("03F90009030C00EA"), String::from("see-you")),
        (String::from("037B01B0"), String::from("bye")),
        (
            String::from("0411031B00A8000303BD0219030900A80006038A00CF0042035A"),
            String::from("what-are-you-doing?"),
        ),
        (
            String::from("0411031B00A8000303C603FC00C9000303B4"),
            String::from("whats-up?"),
        ),
        (
            String::from("0411031B00A8000303C603FC00C9000303B4036C02B8"),
            String::from("whats-up-there?"),
        ),
        (
            String::from("042F0087000303AB009F02EE"),
            String::from("apple"),
        ),
        (
            String::from("033C006903EA0231033301AD034800060390"),
            String::from("never-mind"),
        ),
        (
            String::from("039C010B00060390034501B0000303C6"),
            String::from("good-night"),
        ),
        (
            String::from("03F60084035A000303C9030C00EA"),
            String::from("thank-you"),
        ),
        (
            String::from("04290045000303C603DE02F700C603390042035100990318018F"),
            String::from("its-running-away"),
        ),
        (
            String::from("03F002EB01DD02340006037500C900060390"),
            String::from("flower-bud"),
        ),
        (
            String::from("03BA008700060393030902190045000303C6"),
            String::from("tag-youre-it"),
        ),
        (
            String::from("039F01FE00BA0318018F"),
            String::from("go-away"),
        ),
        (
            String::from("02E8010B000303D5021603EA0237036C02B8"),
            String::from("look-over-there"),
        ),
        (
            String::from("04290045000303C603FC021603EA0237036C02B8"),
            String::from("its-over-there"),
        ),
        (String::from("02DF003C02DF0024"), String::from("lily")),
        (
            String::from("03FC00C6034803F002EB01DD0234"),
            String::from("sunflower"),
        ),
        (
            String::from("03FC000303CF00C60348000303D80006037E02EB015003FC014A0336"),
            String::from("skunk-blossom"),
        ),
        (
            String::from("03FC000303C602FD016E00060372029D0024"),
            String::from("strawberry"),
        ),
        (
            String::from("02F4008A03FC000303B402F10024"),
            String::from("raspberry"),
        ),
        (
            String::from("0438012C0006038100480402"),
            String::from("oddish"),
        ),
        (String::from("03A202E800E70336"), String::from("gloom")),
        (
            String::from("03EA01A702EE000303B402E800E70336"),
            String::from("vileplume"),
        ),
        (
            String::from("042F01AD033603FC02BE0024"),
            String::from("im-sorry"),
        ),
        (String::from("03FC02BE0024"), String::from("sorry")),
        (String::from("03CC0084033F00A50348"), String::from("cannon")),
        (
            String::from("03810045000603930045000303C301F2000303C6"),
            String::from("dig-it-out"),
        ),
        (
            String::from("03810045000603A2036C017A0234"),
            String::from("dig-there"),
        ),
        (String::from("03810045000603A2"), String::from("dig")),
        (String::from("03AE010202EE"), String::from("pull")),
        (String::from("033F00C9000303C6"), String::from("nut")),
        (
            String::from("03780108033C0084033F00C9"),
            String::from("banana"),
        ),
        (
            String::from("03C602F4006903E4023400030360006903FC000303C6"),
            String::from("treasure-chest"),
        ),
        (
            String::from("03C602F4006903E40234"),
            String::from("treasure"),
        ),
        (
            String::from("0372029D002400060390000303C602F4006903E40234"),
            String::from("buried-treasure"),
        ),
        (String::from("0408005D02EB0213"), String::from("hello")),
        (
            String::from("042F01A702E500CC03E7030C00EA"),
            String::from("i-love-you"),
        ),
        (
            String::from("04080084035701290348"),
            String::from("hang-on"),
        ),
        (
            String::from("03C602F40066000603660234"),
            String::from("treasure"),
        ),
        (
            String::from("03960066000303C602EB015003FC000303C6"),
            String::from("get-lost"),
        ),
        (
            String::from("03A50024000303CF00BD040B00A8"),
            String::from("pikka"),
        ),
        (
            String::from("03A500090024000303CF00C9"),
            String::from("pi-ka"),
        ),
        (
            String::from("0375009C041101AD034800060381030C00EA"),
            String::from("behind-you"),
        ),
        (
            String::from("036F0018041101AD03480006036600EA"),
            String::from("behind-you"),
        ),
        (
            String::from("041101EF032A006303390024"),
            String::from("how-many?"),
        ),
        (
            String::from("041101EF032A0063033900150258036C02B8"),
            String::from("how-many-are-there?"),
        ),
        (
            String::from("03B1020A02DF001503180087000603A2"),
            String::from("poliwag"),
        ),
        (
            String::from("03B1014402DF001503180087000603A2"),
            String::from("poliwag"),
        ),
        (
            String::from("03B1020A02DF0015031B022B02EE"),
            String::from("poliwhirl"),
        ),
        (
            String::from("03FC000303D5031B0234000303BD009F02EE"),
            String::from("squirtle"),
        ),
        (
            String::from("0396008A03FC000303C602DF0024"),
            String::from("gastly"),
        ),
        (
            String::from("041101290348000303BD0234"),
            String::from("haunter"),
        ),
        (String::from("02FD0129035A"), String::from("wrong")),
        (
            String::from("036C0087000303C603DE02FD0129035A"),
            String::from("thats-wrong"),
        ),
        (String::from("03D503150045000303C6"), String::from("quit")),
        (
            String::from("03CF00C9000303B700450006038D01F2000303C6"),
            String::from("cut-it-out"),
        ),
        (String::from("03CF00A503270297"), String::from("c-mere")),
        (
            String::from("03CF00A5033604050297"),
            String::from("come-here"),
        ),
        (
            String::from("02E8010B000303D5021603EA022804050297"),
            String::from("look-over-here"),
        ),
        (
            String::from("02E8010B000303D501F2000303C6"),
            String::from("look-out"),
        ),
        (
            String::from("0321012C0003036001F2000303C6"),
            String::from("watch-out"),
        ),
        (String::from("036C02B8"), String::from("there")),
        (
            String::from("03FC03150042035A034501F2"),
            String::from("swing-now"),
        ),
        (String::from("03FC03150042035A"), String::from("swing")),
        (
            String::from("039F020D02FD01B0000303C6"),
            String::from("go-right"),
        ),
        (String::from("02FD01B0000303C6"), String::from("right")),
        (
            String::from("03F00258036C022E02FD01B0000303C6"),
            String::from("farther-right"),
        ),
        (
            String::from("039F020A02E2006903F0000303C6"),
            String::from("go-left"),
        ),
        (String::from("02E2006903F0000303C6"), String::from("left")),
        (
            String::from("03F00258036C022B02E2006903F0000303C6"),
            String::from("farther-left"),
        ),
        (
            String::from("03FC000303C3014D000303B4"),
            String::from("stop"),
        ),
        (
            String::from("03FC000303C3014D000303B4034501F2"),
            String::from("stop-now"),
        ),
        (
            String::from("03FC000303C3014D000303B4036C02B8"),
            String::from("stop-there"),
        ),
        (
            String::from("03FC000303A50042033F00A202FD01EF034800060390"),
            String::from("spin-around"),
        ),
        (
            String::from("02FD012903570318018F"),
            String::from("wrong-way"),
        ),
        (
            String::from("03BD0231033F00A202FD01EF034800060390"),
            String::from("turn-around"),
        ),
        (
            String::from("03720087000303CF00C9000303B4"),
            String::from("back-up"),
        ),
        (
            String::from("03C90024000303B40006039F01F80042035A"),
            String::from("keep-going"),
        ),
        (
            String::from("03FC000303C602F4018F00060387009C0408006600060390"),
            String::from("straight-ahead"),
        ),
        (
            String::from("037500C002EE0006037500AB03FC02D9"),
            String::from("bulbasaur"),
        ),
        (
            String::from("03E70021033F00AB03FC02D9"),
            String::from("venusaur"),
        ),
        (
            String::from("03FC02EB0213000303B10213000303D8"),
            String::from("slowpoke"),
        ),
        (
            String::from("03FC000303D5031B0234000303BD009F02EE"),
            String::from("squirtle"),
        ),
        (
            String::from("03600252032A00840348000603870234"),
            String::from("charmander"),
        ),
        (
            String::from("03C903060087000303BD0234000303A50024"),
            String::from("caterpie"),
        ),
        (
            String::from("02EB014D000303B402F700AB03FC"),
            String::from("lapras"),
        ),
        (
            String::from("02E20087000303B402F700AB03FC"),
            String::from("lapras"),
        ),
        (
            String::from("03C302130006039900A8000303A50024"),
            String::from("togepi"),
        ),
        (
            String::from("03C30213000603930045000303A500090024"),
            String::from("togepi"),
        ),
        (
            String::from("03A500420339030F012C0006038700A8"),
            String::from("pinata"),
        ),
        (
            String::from("03A500210339030F014D000303BD00A8"),
            String::from("pinata"),
        ),
        (
            String::from("02DF004803F900420348000303BD00A503270024"),
            String::from("listen-to-me"),
        ),
        (String::from("03F002FA00EA000303C6"), String::from("fruit")),
        (
            String::from("03450213000303C6000303A8008700060390"),
            String::from("notepad"),
        ),
        (
            String::from("042900420348000603A202F1002400060381000F00A50348000303C603FC"),
            String::from("ingredients"),
        ),
        (
            String::from("02F4006903FC00A8000303A50024"),
            String::from("recipe"),
        ),
        (
            String::from("02F4006903F90045000303A50024"),
            String::from("recipe"),
        ),
        (
            String::from("0411014D000303C603FC03150024000303AB00C9000303BA018F0006038D0213"),
            String::from("hot-sweet-potato"),
        ),
        (
            String::from("0411014D000303C603FC03150024000303B4000303BA018F0006038D0213"),
            String::from("hot-sweet-potato"),
        ),
        (String::from("03D200E102EE"), String::from("cool")),
        (String::from("03F001AD0348"), String::from("fine")),
        (
            String::from("0309023703FC0213000303C9030C00EA000303C6"),
            String::from("youre-so-cute"),
        ),
        (
            String::from("0393004803E70045000303C6000303C000E703270024"),
            String::from("give-it-to-me"),
        ),
        (
            String::from("0393004203270027036C0087000303C6"),
            String::from("gimme-that"),
        ),
        (
            String::from("03F602FD01F80045000303C6"),
            String::from("throw-it"),
        ),
        (
            String::from("03FC000303C000EA000303A5004500060390"),
            String::from("stupid"),
        ),
        (
            String::from("02E20066000303C603FC000303B402E2018F"),
            String::from("lets-play"),
        ),
        (
            String::from("039C0108033302D603390042035A"),
            String::from("good-morning"),
        ),
        (
            String::from("0318018F000303CF00C9000303B4"),
            String::from("wake-up"),
        ),
        (
            String::from("03960066000303BD00C9000303B4"),
            String::from("get-up"),
        ),
        (
            String::from("04260024000303CF00A5034803DE"),
            String::from("ekans"),
        ),
        (
            String::from("042C0066000303CF00A5034803DE"),
            String::from("ekans"),
        ),
        (
            String::from("037E02F10042034B0045000303C604050297"),
            String::from("bring-it-here"),
        ),
        (
            String::from("030C00EA000303CF00A503390024000303C6036C0087000303C6"),
            String::from("you-can-eat-that"),
        ),
        (
            String::from("03BA019203FC000303BD00A8000303C6"),
            String::from("taste-it"),
        ),
        (
            String::from("04260024000303B70045000303C6"),
            String::from("eat-it"),
        ),
        (
            String::from("032A0087000603A2033F00A8000303C6"),
            String::from("magnet"),
        ),
        (String::from("03D501CE0348"), String::from("coin")),
        (String::from("036600D5009F02EE"), String::from("jewel")),
        (
            String::from("0333025500060375009F02EE"),
            String::from("marble"),
        ),
        (
            String::from("03B101AA02F10045000303C603FC01"),
            String::from("pirate-sword"),
        ),
        (
            String::from("039C0108034501B0000303C6"),
            String::from("good-night"),
        ),
        (
            String::from("03D50315004803DE000303C301AD0336"),
            String::from("quiz-time"),
        ),
        (String::from("03B7002703E70024"), String::from("tv")),
        (
            String::from("042F01B0000303D503150045000303C6"),
            String::from("i-quit"),
        ),
        (
            String::from("042F01AD03360006038700C60348"),
            String::from("im-done"),
        ),
        (
            String::from("030C00EA000303CF00A50348000303B402E2018F"),
            String::from("you-can-play"),
        ),
        (
            String::from("03150024000303CF00A50348000303B402E2018F"),
            String::from("we-can-play"),
        ),
        (String::from("0372008700060390"), String::from("bad")),
        (
            String::from("0345014D000303C90045000303C3012F03F0"),
            String::from("knock-it-off"),
        ),
        (
            String::from("03450213000303B402E201740042035A"),
            String::from("no-playing"),
        ),
        (
            String::from("03FC000303C3014D000303B4000303B402E201740042035A"),
            String::from("stop-playing"),
        ),
        (
            String::from("042F01AD03360345014D000303C6000303B402E201740042035A"),
            String::from("im-not-playing"),
        ),
        (String::from("03D503150045000303C6"), String::from("quit")),
        (
            String::from("03FC000303C3014D000303B4036C0087000303C6"),
            String::from("stop-that"),
        ),
        (
            String::from("03D802DF002103390045000303BD00C9000303B4"),
            String::from("clean-it-up"),
        ),
        (
            String::from("03AE010B000303B70045000303BD00990318018F"),
            String::from("put-it-away"),
        ),
        (
            String::from("03D802DF0021033F00C9000303B4"),
            String::from("clean-up"),
        ),
        (
            String::from("03D802DF00210348036C0087000303BD00C9000303B4"),
            String::from("clean-that-up"),
        ),
        (String::from("03B70048040200EA"), String::from("tissue")),
        (
            String::from("037500C002EE0006037500AB03FC02D9"),
            String::from("bulbasaur"),
        ),
        (
            String::from("042F01B303E7002703FC02D9"),
            String::from("ivysaur"),
        ),
        (
            String::from("042F01B303EA00AB03FC02D9"),
            String::from("ivysaur"),
        ),
        (
            String::from("03E70021033F00AB03FC02D9"),
            String::from("venusaur"),
        ),
        (
            String::from("03600252032A00840348000603870234"),
            String::from("charmander"),
        ),
        (
            String::from("036002520327001B02DF0009030900A50348"),
            String::from("charmeleon"),
        ),
        (
            String::from("0360025803DE025500060390"),
            String::from("charizard"),
        ),
        (
            String::from("0360024000AB03DE025500060390"),
            String::from("charizard"),
        ),
        (
            String::from("03FC000303D5031B0234000303BD009F02EE"),
            String::from("squirtle"),
        ),
        (
            String::from("03210255000303BD0234000303BD009F02EE"),
            String::from("wartortle"),
        ),
        (
            String::from("03210255000303C302D9000303BD009F02EE"),
            String::from("wartortle"),
        ),
        (
            String::from("037E02E2008A03FC000303C301D403FC"),
            String::from("blastoise"),
        ),
        (
            String::from("03CC0087000303BD0234000303A50024"),
            String::from("caterpie"),
        ),
        (
            String::from("032A0066000303BD00A8000303B1014D00060390"),
            String::from("metapod"),
        ),
        (
            String::from("037500C9000303BD023703F002F10024"),
            String::from("butterfree"),
        ),
        (
            String::from("0315002400060387009F02EE"),
            String::from("weedle"),
        ),
        (
            String::from("03D2010B000303D200E7033F00A8"),
            String::from("kakuna"),
        ),
        (
            String::from("03C90045000303D200E7033F00A8"),
            String::from("kakuna"),
        ),
        (
            String::from("036F00240006039002F1003C02EE"),
            String::from("beedrill"),
        ),
        (
            String::from("036F00240006039002F7009F02EE"),
            String::from("beedrill"),
        ),
        (
            String::from("03A5004500060366014D000303C6"),
            String::from("pidgeot"),
        ),
        (
            String::from("02F40087000303BD00A8000303BD00A8"),
            String::from("rattata"),
        ),
        (
            String::from("02F700A8000303BA00870006038700A8"),
            String::from("rattata"),
        ),
        (
            String::from("02F40087000303BA00870006038700A8"),
            String::from("rattata"),
        ),
        (
            String::from("02F40087000303B70045000303CC018F000303C6"),
            String::from("raticate"),
        ),
        (
            String::from("03FC000303A502880213"),
            String::from("spearow"),
        ),
        (String::from("03ED02880213"), String::from("fearow")),
        (
            String::from("04260024000303CF00A5034803DE"),
            String::from("ekans"),
        ),
        (
            String::from("042C0066000303CF00A5034803DE"),
            String::from("ekans"),
        ),
        (
            String::from("043802550006037B014D000303D8"),
            String::from("arbok"),
        ),
        (
            String::from("03A50024000303CF00A80003036000EA"),
            String::from("pikachu"),
        ),
        (String::from("02FD01B00003036000EA"), String::from("raichu")),
        (
            String::from("03FC00840348040202FA00EA"),
            String::from("sandshrew"),
        ),
        (
            String::from("03FC0084034803FC02E2008A0402"),
            String::from("sandslash"),
        ),
        (
            String::from("0339004500060387021F00A5034803ED0021032A018602EE"),
            String::from("nidoran-female"),
        ),
        (
            String::from("033900450006038D020D02F700A5034803ED0021032A017A009F02EE"),
            String::from("nidoran-female"),
        ),
        (
            String::from("033900450006038702190021033F00A8"),
            String::from("nidorina"),
        ),
        (
            String::from("033900240006038D0213000303D5031500210348"),
            String::from("nidoqueen"),
        ),
        (
            String::from("033900450006038D020D02F700A50348032A018602EE"),
            String::from("nidoran-male"),
        ),
        (
            String::from("0339004500060387021F00A50348032A017A009F02EE"),
            String::from("nidoran-male"),
        ),
        (
            String::from("033900450006038D02BE002103450213"),
            String::from("nidorino"),
        ),
        (
            String::from("033900240006038D020D02F1002103450213"),
            String::from("nidorino"),
        ),
        (
            String::from("033900240006038D0213000303C90042035A"),
            String::from("nidoking"),
        ),
        (
            String::from("03D802E2006903F0008102F10024"),
            String::from("clefairy"),
        ),
        (
            String::from("03D802E500AB03F0008102F10024"),
            String::from("clefairy"),
        ),
        (
            String::from("03D802E2006903F0018F00060375009F02EE"),
            String::from("clefable"),
        ),
        (
            String::from("03D802E500AB03F0018F00060375009F02EE"),
            String::from("clefable"),
        ),
        (
            String::from("03EA009F02EE000303A50045000303D803FC"),
            String::from("vulpix"),
        ),
        (
            String::from("034501AD0348000303BA018602EE03DE"),
            String::from("ninetales"),
        ),
        (
            String::from("03630045000603A202DF0024000303AB00CC03F0"),
            String::from("jigglypuff"),
        ),
        (
            String::from("0363004500060399009F02DF0024000303AB00CC03F0"),
            String::from("jigglypuff"),
        ),
        (
            String::from("03150045000603A202DF0024000303BD00CC03F0"),
            String::from("wigglytuff"),
        ),
        (
            String::from("03DE00EA000603720087000303C6"),
            String::from("zubat"),
        ),
        (
            String::from("039F020A02EE000603720087000303C6"),
            String::from("golbat"),
        ),
        (
            String::from("0438012C0006038100480402"),
            String::from("oddish"),
        ),
        (String::from("03A202E800E70336"), String::from("gloom")),
        (
            String::from("03EA01A702EE000303B402E800E70336"),
            String::from("vileplume"),
        ),
        (String::from("03B1024000AB03FC"), String::from("paras")),
        (String::from("03A802A300AB03FC"), String::from("paras")),
        (
            String::from("03B1024000AB03FC0066000303D8000303C6"),
            String::from("parasect"),
        ),
        (
            String::from("03A802A300AB03FC0066000303D8000303C6"),
            String::from("parasect"),
        ),
        (
            String::from("03EA006303450210033C0087000303C6"),
            String::from("venonat"),
        ),
        (
            String::from("03EA0063034502100333012F03F6"),
            String::from("venomoth"),
        ),
        (
            String::from("03810045000603A202E500A8000303C6"),
            String::from("diglett"),
        ),
        (String::from("0327001501F503F6"), String::from("meowth")),
        (
            String::from("03AB023703E400A50348"),
            String::from("persian"),
        ),
        (
            String::from("03FC01B00006038700C9000303D8"),
            String::from("psyduck"),
        ),
        (
            String::from("039C010202EE0006038700C9000303D8"),
            String::from("golduck"),
        ),
        (
            String::from("032A0084035A000303C90024"),
            String::from("mankey"),
        ),
        (
            String::from("03B402FD01AD032A018F000303B4"),
            String::from("primeape"),
        ),
        (
            String::from("03A202FD020A02EB01B3036C"),
            String::from("growlithe"),
        ),
        (
            String::from("03A202FD01E902DF0048036C"),
            String::from("growlithe"),
        ),
        (
            String::from("04380255000303CC018C034501AD0348"),
            String::from("arcanine"),
        ),
        (
            String::from("04380255000303CF00A5034501AD0348"),
            String::from("arcanine"),
        ),
        (
            String::from("03B1020A02DF001503180087000603A2"),
            String::from("poliwag"),
        ),
        (
            String::from("03B1020A02DF0015031B022B02EE"),
            String::from("poliwhirl"),
        ),
        (
            String::from("03B1020A02DF001E02F4008A03F6"),
            String::from("poliwrath"),
        ),
        (
            String::from("03B1014402DF001E02F4008A03F6"),
            String::from("poliwrath"),
        ),
        (
            String::from("042F00870006037E02F700A8"),
            String::from("abra"),
        ),
        (
            String::from("03CF00A80006038400870006037E02F700A8"),
            String::from("kadabra"),
        ),
        (
            String::from("042F007E02E500A8000303CF00AB03DE00840336"),
            String::from("alakazam"),
        ),
        (
            String::from("032D00A800030360014D000303B4"),
            String::from("machop"),
        ),
        (
            String::from("032D00A8000303600213000303D8"),
            String::from("machoke"),
        ),
        (
            String::from("032D00A80003036000840336000303B4"),
            String::from("machamp"),
        ),
        (
            String::from("0372005D02EE03FC000303B402FD01F2000303C6"),
            String::from("bellsprout"),
        ),
        (
            String::from("03BA0063033F00A8000303CF00C002EE"),
            String::from("tentacool"),
        ),
        (
            String::from("03BA0063033F00A8000303D802FA00E102EE"),
            String::from("tentacruel"),
        ),
        (
            String::from("03630009030F02130006038A00EA00060390"),
            String::from("geodude"),
        ),
        (
            String::from("03A202F4008A03EA009F02E50234"),
            String::from("graveler"),
        ),
        (
            String::from("03A202F4008A03EA02E50234"),
            String::from("graveler"),
        ),
        (String::from("039F020A02E500A50336"), String::from("golem")),
        (String::from("039F014402EE0336"), String::from("golem")),
        (
            String::from("03B1021003390024000303BD00A8"),
            String::from("ponyta"),
        ),
        (
            String::from("03B1021003390024000303C3016E"),
            String::from("ponyta"),
        ),
        (
            String::from("02F40087000303A5004500060384008A0402"),
            String::from("rapidash"),
        ),
        (
            String::from("03FC02EB0213000303B10213000303D8"),
            String::from("slowpoke"),
        ),
        (
            String::from("03FC02EB02130006037E02FD0213"),
            String::from("slowbro"),
        ),
        (
            String::from("032A0087000603A2033F00A5033301B0000303C6"),
            String::from("magnemite"),
        ),
        (
            String::from("032A0087000603A2033F00A8000303C3014A0348"),
            String::from("magneton"),
        ),
        (String::from("0008"), String::from("farfetch")),
        (
            String::from("038D02130006039002F100150213"),
            String::from("dodrio"),
        ),
        (String::from("03F9001B02EE"), String::from("seel")),
        (
            String::from("038A00EA0006039F014A035A"),
            String::from("dewgong"),
        ),
        (
            String::from("038A00EA0006039F014A0348"),
            String::from("dewgong"),
        ),
        (String::from("03A202FD01AD032D0234"), String::from("grimer")),
        (String::from("032D00C9000303D8"), String::from("muk")),
        (
            String::from("0402005D02EE000603870234"),
            String::from("shellder"),
        ),
        (
            String::from("03D802EB01D403FC000303BD0234"),
            String::from("cloyster"),
        ),
        (
            String::from("0396008A03FC000303C602DF00"),
            String::from("gastly"),
        ),
        (String::from("0396008A03FC02DF0024"), String::from("gastly")),
        (
            String::from("041101290348000303BD0234"),
            String::from("haunter"),
        ),
        (
            String::from("03960084035A0006039F0255"),
            String::from("gengar"),
        ),
        (
            String::from("0396006303480006039F0255"),
            String::from("gengar"),
        ),
        (
            String::from("0438014A03390045000303D803FC"),
            String::from("onix"),
        ),
        (
            String::from("039002FD01F503DB0024"),
            String::from("drowzee"),
        ),
        (
            String::from("039002FD021603DB0024"),
            String::from("drowzee"),
        ),
        (
            String::from("04050045000303B403450213"),
            String::from("hypno"),
        ),
        (
            String::from("03D802F400870006036F00"),
            String::from("krabby"),
        ),
        (
            String::from("03C90042035A02E50234"),
            String::from("kingler"),
        ),
        (
            String::from("03EA020A02EE000303C302D90006037E"),
            String::from("voltorb"),
        ),
        (
            String::from("0426001B02E20066000303D8000303C602FD021300060390"),
            String::from("electrode"),
        ),
        (
            String::from("0432009F02E20066000303D8000303C602FD021300060390"),
            String::from("electrode"),
        ),
        (
            String::from("042C0066000603A203DE000303C9030C00EA000303C6"),
            String::from("exeggcute"),
        ),
        (
            String::from("042C0066000603A203DE0066000303C9030C00EA000303C302D9"),
            String::from("exeggutor"),
        ),
        (
            String::from("042C0066000603A203FC00A8000303C9030C00EA000603870234"),
            String::from("exeggutor"),
        ),
        (
            String::from("03C9030C00EA0006037B02100348"),
            String::from("cubone"),
        ),
        (
            String::from("032A02A9020403180087000303D8"),
            String::from("marowak"),
        ),
        (
            String::from("03330246020403180087000303D8"),
            String::from("marowak"),
        ),
        (
            String::from("04050045000303C603330210034802DF0024"),
            String::from("hitmonlee"),
        ),
        (
            String::from("04050045000303C60333014A034802DF0024"),
            String::from("hitmonlee"),
        ),
        (
            String::from("04050045000303C60333021003480003036000840348"),
            String::from("hitmonchan"),
        ),
        (
            String::from("02DF0045000303C90024000303BD00A5035A"),
            String::from("lickitung"),
        ),
        (
            String::from("02DF0045000303C90045000303BD00A5035A"),
            String::from("lickitung"),
        ),
        (
            String::from("02DF0045000303C90045000303C00108035A"),
            String::from("lickitung"),
        ),
        (
            String::from("03D5015003ED0042035A"),
            String::from("koffing"),
        ),
        (
            String::from("0315002703DB0042035A"),
            String::from("weezing"),
        ),
        (
            String::from("02FD01A4041102D60348"),
            String::from("rhyhorn"),
        ),
        (
            String::from("02FD01B00006038D01290348"),
            String::from("rhydon"),
        ),
        (
            String::from("03600084034803F90024"),
            String::from("chansey"),
        ),
        (
            String::from("03BA0084034800060366005D02E500A8"),
            String::from("tangela"),
        ),
        (
            String::from("03BA018C034800060396005D02E500A8"),
            String::from("tangela"),
        ),
        (
            String::from("03CC018C035A0006039900AB03FC000303D5014A0348"),
            String::from("kangaskhan"),
        ),
        (String::from("041102DC03F90024"), String::from("horsea")),
        (
            String::from("03F900240006039002F700A8"),
            String::from("seadra"),
        ),
        (
            String::from("039F020A02EE0006038100210348"),
            String::from("goldeen"),
        ),
        (
            String::from("03F90024000303C90042035A"),
            String::from("seaking"),
        ),
        (
            String::from("03FC000303C3023A030C00EA"),
            String::from("staryu"),
        ),
        (
            String::from("03FC000303C3025203270024"),
            String::from("starmie"),
        ),
        (
            String::from("0327004803FC000303BD0231033301AD0336"),
            String::from("mr.mime"),
        ),
        (String::from("03FC01B3036C0234"), String::from("scyther")),
        (
            String::from("03630042035A000303D803FC"),
            String::from("jynx"),
        ),
        (
            String::from("0426001B02E20066000303D8000303BD00A80006037500CC03DE"),
            String::from("electabuzz"),
        ),
        (
            String::from("0432009F02E20066000303D8000303BD00A80006037500CC03DE"),
            String::from("electabuzz"),
        ),
        (
            String::from("032A0087000603A203330255"),
            String::from("magmar"),
        ),
        (String::from("03A50042034803FC0234"), String::from("pinsir")),
        (String::from("03C3020D02FD012F03FC"), String::from("tauros")),
        (String::from("03C30246021603FC"), String::from("tauros")),
        (
            String::from("032A0087000603630045000303D50255000303B4"),
            String::from("magikarp"),
        ),
        (
            String::from("039602A300A80006038D015003FC"),
            String::from("gyarados"),
        ),
        (
            String::from("039602A300A80006038D021603FC"),
            String::from("gyarados"),
        ),
        (
            String::from("02EB014D000303B402F700AB03FC"),
            String::from("lapras"),
        ),
        (
            String::from("02E20087000303B402F700AB03FC"),
            String::from("lapras"),
        ),
        (String::from("03810045000303C30213"), String::from("ditto")),
        (String::from("0426002703E70024"), String::from("eevee")),
        (
            String::from("03EA018F000303B102BE0015014A0348"),
            String::from("vaporeon"),
        ),
        (
            String::from("03EA00A8000303B102BE000F00A50348"),
            String::from("vaporeon"),
        ),
        (
            String::from("0366020A02EE000303B70015014A0348"),
            String::from("jolteon"),
        ),
        (
            String::from("03F002E2029D0015014A0348"),
            String::from("flareon"),
        ),
        (
            String::from("03B1020D02F100240006039F014A0348"),
            String::from("porygon"),
        ),
        (
            String::from("03B1020D02F100240006039900A50348"),
            String::from("porygon"),
        ),
        (
            String::from("043B0210032D00C6034501B0000303C6"),
            String::from("omanyte"),
        ),
        (
            String::from("04380129032D00A5034501B0000303C6"),
            String::from("omanyte"),
        ),
        (
            String::from("043B0210032D00CC03FC000303C30255"),
            String::from("omastar"),
        ),
        (
            String::from("04380129032D00AB03FC000303C30255"),
            String::from("omastar"),
        ),
        (
            String::from("03CF00C90006037800EA0006038D0213"),
            String::from("kabuto"),
        ),
        (
            String::from("03D5012C0006037800EA000303C3014D000303B403FC"),
            String::from("kabutops"),
        ),
        (
            String::from("03D5012C0006037500A8000303C3014D000303B403FC"),
            String::from("kabutops"),
        ),
        (
            String::from("03D5012C0006037500A8000303C3015003FC"),
            String::from("kabutops"),
        ),
        (
            String::from("042C02A90213000603840087000303D8000303BD00C002EE"),
            String::from("aerodactyl"),
        ),
        (
            String::from("03FC034502D002E20087000303D803FC"),
            String::from("snorlax"),
        ),
        (
            String::from("04380255000303B70045000303D200E703450213"),
            String::from("articuno"),
        ),
        (
            String::from("03DE0087000303B40006038D015003FC"),
            String::from("zapdos"),
        ),
        (
            String::from("03DE0087000303B40006038D021603FC"),
            String::from("zapdos"),
        ),
        (
            String::from("0333020A02EE000303C602F4006903FC"),
            String::from("moltres"),
        ),
        (
            String::from("039002F700A8000303B7002103390024"),
            String::from("dratini"),
        ),
        (
            String::from("039002F400870006039900A5033C02B8"),
            String::from("dragonair"),
        ),
        (
            String::from("039002F400870006039900A5034501B0000303C6"),
            String::from("dragonite"),
        ),
        (
            String::from("0327030C00EA000303C000EA"),
            String::from("mewtwo"),
        ),
        (String::from("0327030C00EA"), String::from("mew")),
        (
            String::from("03C3014D000303AB00AB03EA000603630045000603A202DF0024000303AB00AB03F0"),
            String::from("top-of-jigglypuff"),
        ),
        (
            String::from("03C302130006039900A8000303A50024"),
            String::from("togepi"),
        ),
        (
            String::from("03C302130006039C010B000303A50024"),
            String::from("togepi"),
        ),
        (
            String::from("03C30213000603930045000303A500090024"),
            String::from("togepi"),
        ),
        (
            String::from("03B1014402DF001503180087000603A2"),
            String::from("poliwag"),
        ),
        (
            String::from("03150024000303A50042034800060372005D02EE"),
            String::from("weepinbell"),
        ),
        (
            String::from("03E70045000303D8000303C602F1002400060372005D02EE"),
            String::from("victreebel"),
        ),
        (
            String::from("03BA0063033F00A8000303D200E102EE"),
            String::from("tentacool"),
        ),
        (
            String::from("038700C9000603A2000303C602F100150213"),
            String::from("dugtrio"),
        ),
        (String::from("03A50045000603630024"), String::from("pidgey")),
        (
            String::from("038D02130006038A00DB0213"),
            String::from("doduo"),
        ),
        (
            String::from("03A50045000603660213000303C30213"),
            String::from("pidgeotto"),
        ),
        (
            String::from("03CF00C9000303D200E7033F00A8"),
            String::from("kakuna"),
        ),
        (
            String::from("03BA00630348000303BD00A8000303D200E102EE"),
            String::from("tentacool"),
        ),
        (
            String::from("03CC018C035A00060396008A03FC000303D5014A0348"),
            String::from("kangaskhan"),
        ),
        (
            String::from("043B0210032D00AB03FC000303C30255"),
            String::from("omastar"),
        ),
        (
            String::from("03CF00A80006037800EA000303C3014D000303B403FC"),
            String::from("kabutops"),
        ),
        (
            String::from("036C0087000303C603FC02FD01B0000303C6"),
            String::from("thats-right"),
        ),
        (
            String::from("036C0087000303C603FC0006039C010B00060390"),
            String::from("thats-good"),
        ),
        (
            String::from("036C0087000303C3031B00C60348"),
            String::from("that-one"),
        ),
        (
            String::from("036C008A03FC03F001AD0348"),
            String::from("thats-fine"),
        ),
        (
            String::from("036C0087000303C603DE03F001AD0348"),
            String::from("thats-fine"),
        ),
        (
            String::from("03CF021C0066000303D8000303C6"),
            String::from("correct"),
        ),
        (
            String::from("03CF00A202F40066000303D8000303C6"),
            String::from("correct"),
        ),
        (
            String::from("036F0042035A0006039F0213"),
            String::from("bingo"),
        ),
        (
            String::from("03D5012302DF0045000303C6"),
            String::from("call-it"),
        ),
        (
            String::from("02E2006903FC000303B402E2018F"),
            String::from("lets-play"),
        ),
        (String::from("02FD0129035A"), String::from("wrong")),
        (
            String::from("0372008700060390000303A50024000303CF00A80003036000EA"),
            String::from("bad-pikachu"),
        ),
        (String::from("0327004803FC"), String::from("miss")),
        (
            String::from("038D02100348000303C6000303D5012302DF0045000303C6"),
            String::from("dont-call-it"),
        ),
        (String::from("041101F503FC"), String::from("house")),
        (String::from("041102100336"), String::from("home")),
        (
            String::from("03EA00A202F1004500060381000F00C60348"),
            String::from("viridian"),
        ),
        (
            String::from("03EA00A202F1004500060381000F00C6034803F002C400AB03FC000303C6"),
            String::from("viridian-forest"),
        ),
        (String::from("043B0213000303CF0234"), String::from("ochre")),
        (
            String::from("043B0213000303CF0225031E010B0006039003DE"),
            String::from("ochre-woods"),
        ),
        (
            String::from("043B0213000303CF023703ED004803FF0042035A0411020A02EE"),
            String::from("ochre-fishing-hole"),
        ),
        (
            String::from("03FC000303B402F10042035A02DF002703F0"),
            String::from("springleaf"),
        ),
        (
            String::from("03FC000303B402F10042035A02DF002703F003ED000F00C002EE00060390"),
            String::from("springleaf-field"),
        ),
        (
            String::from("0438014402E500AB03EA01AD0348"),
            String::from("olivine"),
        ),
        (
            String::from("0438014402E500AB03EA01AD034802E2018F000303D8"),
            String::from("olivine-lake"),
        ),
        (
            String::from("0438014402E500AB03EA01AD034803ED004803FF0042035A0411020A02EE"),
            String::from("olivine-fishing-hole"),
        ),
        (
            String::from("03D502130006037B012302EE000303C6"),
            String::from("cobalt"),
        ),
        (
            String::from("03D502130006037B012302EE000303C301A702E500C6034800060390"),
            String::from("cobalt-island"),
        ),
        (
            String::from("03D502130006037E03ED004803FF0042035A0411020A02EE"),
            String::from("cobalt-fishing-hole"),
        ),
        (String::from("03D5021603FC000303C6"), String::from("coast")),
        (
            String::from("03D502130006037B012302EE000303C6000303D5021603FC000303C6"),
            String::from("cobalt-coast"),
        ),
        (
            String::from("03FC000303A80069040200C002EE000303C602F4018C03390042035A"),
            String::from("special-training"),
        ),
        (
            String::from("03BD0231033F00A8000303B4"),
            String::from("turnip"),
        ),
        (String::from("03BD02310348000303B4"), String::from("turnip")),
        (
            String::from("03FC03150024000303C6000303AB00A8000303BA018F0006038D0213"),
            String::from("sweet-potato"),
        ),
        (
            String::from(
                "0411014D000303C603FC03150024000303C6000303AE010B000303BA018F0006038D0213",
            ),
            String::from("hot-sweet-potato"),
        ),
        (
            String::from("03D501470300000603900006037B02D9000603900006037B014D000303D803FC"),
            String::from("cardboard-box"),
        ),
        (
            String::from("04080084034B00420348036C02B8"),
            String::from("hang-in-there"),
        ),
        (String::from("03AE010202EE"), String::from("pull")),
        (
            String::from("02F1001B02DF0045000303B700420348"),
            String::from("reel-it-in"),
        ),
        (String::from("034501F2"), String::from("now")),
        (
            String::from("03CC00870003035D0045000303C6"),
            String::from("catch-it"),
        ),
        (
            String::from("03960066000303B70045000303C6"),
            String::from("get-it"),
        ),
        (
            String::from("03AE010202DF0045000303C6"),
            String::from("pull-it"),
        ),
        (
            String::from("03AE010202EE04110255000603870234"),
            String::from("pull-harder"),
        ),
        (
            String::from("02E20066000303B70045000303C60006039F0213"),
            String::from("let-it-go"),
        ),
        (
            String::from("02F1003C02DF002703F90045000303C6"),
            String::from("release-it"),
        ),
        (String::from("02E500C6034800030360"), String::from("lunch")),
        (String::from("03FC033C0087000303D8"), String::from("snack")),
        (
            String::from("02E2006903F90024000303C6"),
            String::from("lets-eat"),
        ),
        (
            String::from("03CF00C9000303B4000303CC018F000303D8"),
            String::from("cupcake"),
        ),
        (
            String::from("042F019B00C6034800060387023703FC000303BA0084034800060390"),
            String::from("i-understand"),
        ),
        (
            String::from("042F01B0000603930045000303B70045000303C6"),
            String::from("i-get-it"),
        ),
        (String::from("0411032101B0"), String::from("why?")),
        (
            String::from("041101F2000303CF00C60336"),
            String::from("how-come?"),
        ),
        (
            String::from("039C010B0006037B01B0"),
            String::from("good-bye"),
        ),
        (
            String::from("03BA018F000303D8000303CC02B8"),
            String::from("take-care"),
        ),
        (
            String::from("042F019B009F02EE0327004803F9030C00EA"),
            String::from("ill-miss-you"),
        ),
        (
            String::from("0438016502EE0327004803F9030C00EA"),
            String::from("ill-miss-you"),
        ),
        (
            String::from("03FC000303C3014D000303B4036C0087000303C6"),
            String::from("stop-that"),
        ),
        (String::from("03D503150045000303C6"), String::from("quit")),
        (
            String::from("03CF00C9000303B70045000303C301F2000303C6"),
            String::from("cut-it-out"),
        ),
        (
            String::from("03CF00A503330129034802E20066000303C603DE0006039F0213"),
            String::from("c-mon-lets-go"),
        ),
        (
            String::from("02E20066000303C603DE0006039F0213"),
            String::from("lets-go"),
        ),
        (String::from("031B0225012F03F0"), String::from("we-re-off")),
        (
            String::from("03BA008700060393030902190045000303C6"),
            String::from("tag-youre-it"),
        ),
        (
            String::from("03CF00C9000303B70045000303C301F2000303C6"),
            String::from("cut-it-out"),
        ),
        (
            String::from("0411031B00A8000303C603FC02FD0129035A"),
            String::from("whats-wrong?"),
        ),
        (
            String::from("0411031B00A8000303B7004803F90045000303C6"),
            String::from("what-is-it?"),
        ),
        (
            String::from("03CF00C603330129034802E20066000303C603DE0006039F0213"),
            String::from("c-mon-lets-go!"),
        ),
        (
            String::from("031502970006039F01F80042035A034501F2"),
            String::from("were-going-now"),
        ),
        (
            String::from("02E20066000303C603FC000303600066000303C90045000303C301F2000303C6"),
            String::from("lets-check-it-out"),
        ),
        (
            String::from("03F600C6033F02370402014D000303D8"),
            String::from("thundershock"),
        ),
        (
            String::from("03F600C6033F02340006037B020A02EE000303C6"),
            String::from("thunderbolt"),
        ),
        (
            String::from("043801290339030300420348"),
            String::from("onion"),
        ),
        (
            String::from("03FC02DF0024000303B4000303C301B0000303C6"),
            String::from("sleep-tight"),
        ),
        (
            String::from("03F90009030C00EA000303C000E7033302460213"),
            String::from("see-you-tomorrow"),
        ),
        (
            String::from("03F90009030C00CF00420348036C00C6033302D603390042035A"),
            String::from("see-you-in-the-morning"),
        ),
        (
            String::from("039F0213000303BD00C6033302460213"),
            String::from("go-tomorrow"),
        ),
        (String::from("02FD014D000303D8"), String::from("rock")),
        (
            String::from("03F9004803DE023703FC"),
            String::from("scissors"),
        ),
        (String::from("03A8018F000303AB0234"), String::from("paper")),
        (
            String::from("0381002A000303B10255000303C6"),
            String::from("depart"),
        ),
        (
            String::from("02F10006038100EA03DE00030384002A000303B10255000303C6"),
            String::from("reduced-depart"),
        ),
        (
            String::from("03D5012F035D0063034800030381002A000303B10255000303C6"),
            String::from("caution-depart"),
        ),
        (
            String::from("02DF03FC03BD023A000603D8000303B700060381002A000303B10255000303C6"),
            String::from("restricted-depart"),
        ),
        (
            String::from("03B1008A03FC0042035A"),
            String::from("passing"),
        ),
        (
            String::from("03FC000303C3014D000303AB0042035A"),
            String::from("stopping"),
        ),
        (
            String::from("031E02DC03390042035A"),
            String::from("warning"),
        ),
        (
            String::from("03AB02FA021603F9000900060390"),
            String::from("proceed"),
        ),
        (
            String::from("02F10006038100EA03DE00060390"),
            String::from("reduced"),
        ),
        (
            String::from("03D5012F035D00630348"),
            String::from("caution"),
        ),
        (
            String::from("02DF03FC03BD023A000603D8000303B700060390"),
            String::from("restricted"),
        ),
        (
            String::from("042C0066000603A203FC000303AB021C005A03DE"),
            String::from("express"),
        ),
        (
            String::from("02E50042032A002A000303C6"),
            String::from("limit"),
        ),
        (
            String::from("03450213000602E50042032A002A000303C6"),
            String::from("no-limit"),
        ),
        (String::from("03B7005A0348"), String::from("ten")),
        (
            String::from("03E7002A03F6000303B700090348"),
            String::from("fifteen"),
        ),
        (
            String::from("03C30318004B0339038A0024"),
            String::from("twenty"),
        ),
        (
            String::from("03C30318004B0339038A002403EA019E03EA"),
            String::from("twenty-five"),
        ),
        (String::from("036C02AC03BA0027"), String::from("thirty")),
        (
            String::from("036C02AC03BA002703EA019E03EA"),
            String::from("thirty-five"),
        ),
        (String::from("03EA02D6000603900048"), String::from("forty")),
        (
            String::from("03EA02D600060390004803EA019E03EA"),
            String::from("forty-five"),
        ),
        (
            String::from("03F3005A03F0000303B70042"),
            String::from("fifty"),
        ),
        (
            String::from("03F3005A03F0000303B7004203EA019E03EA"),
            String::from("fifty-five"),
        ),
        (
            String::from("03F9002A000303D803FC000303C00009"),
            String::from("sixty"),
        ),
        (
            String::from("03F9002A000303D803FC000303C0000903EA019E03EA"),
            String::from("sixty-five"),
        ),
        (
            String::from("03FC004B036C00630348000303B70009"),
            String::from("seventy"),
        ),
        (
            String::from("03FC004B036C00630348000303B7000903EA019E03EA"),
            String::from("seventy-five"),
        ),
        (String::from("042C018C03B70009"), String::from("eighty")),
        (
            String::from("042C018C03B7000903EA019E03EA"),
            String::from("eighty-five"),
        ),
        (
            String::from("034501A103480006038A0048"),
            String::from("ninety"),
        ),
        (
            String::from("034501A103480006038A004803EA019E03EA"),
            String::from("ninety-five"),
        ),
        (
            String::from("031B00C60348040B00BD0348038702F100060390"),
            String::from("one-hundred"),
        ),
        (
            String::from("031B00C60348021303F001AD03E7"),
            String::from("one-oh-five"),
        ),
        (
            String::from("031B00C60348000303B7005A0348"),
            String::from("one-ten"),
        ),
        (
            String::from("031B00C60348000303E7002A03F6000303B700090348"),
            String::from("one-fifteen"),
        ),
        (
            String::from("031B00C60348000303C303180348000303B70024"),
            String::from("one-twenty"),
        ),
        (
            String::from("031B00C60348000303C303180348000303B7002403EA019E03EA"),
            String::from("one-twenty-five"),
        ),
        (
            String::from("82B582E382C182CF82C282B582F182B182A4"),
            String::from("depart"),
        ),
        (
            String::from("82B582E382C182CF82C282B082F182BB82AD"),
            String::from("reduced-depart"),
        ),
        (
            String::from("82B582E382C182CF82C282BF82E382A482A2"),
            String::from("caution-depart"),
        ),
        (
            String::from("82B582E382C182CF82C282AF82A282A982A2"),
            String::from("restricted-depart"),
        ),
    ]);
}
