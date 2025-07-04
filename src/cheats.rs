use crate::device;
use crate::ui;

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
pub struct DecodedCheat {
    code_type: u8,
    address: u32,
    data: u16,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cheats {
    pub cheats: Vec<Vec<DecodedCheat>>,
    pub boot: bool,
}

pub fn init(
    device: &mut device::Device,
    cheat_settings: std::collections::HashMap<String, Option<String>>,
) {
    let cheats =
        serde_json::from_slice::<ui::cheats::Cheats>(include_bytes!("../data/cheats.json"))
            .unwrap()
            .get(&ui::storage::get_game_crc(&device.cart.rom))
            .cloned()
            .unwrap_or_default();

    let re = regex::Regex::new(r"\?+").unwrap();
    for cheat_setting in cheat_settings.iter() {
        if let Some(cheat_data) = cheats.get(cheat_setting.0) {
            let mut option_value = None;
            if let Some(option) = cheat_setting.1 {
                if let Some(found_option_value) = cheat_data
                    .options
                    .as_ref()
                    .cloned()
                    .unwrap_or_default()
                    .get(option)
                {
                    option_value = Some(found_option_value.to_string());
                } else {
                    panic!("Cheat option: {option} not found");
                }
            }

            println!(
                "Cheat: {} Option: {}",
                cheat_setting.0,
                cheat_setting.1.clone().unwrap_or("none".to_string())
            );
            let mut decoded_cheat: Vec<DecodedCheat> = vec![];
            for code in cheat_data.data.iter() {
                let mut result = code.clone();
                if let Some(option_value) = option_value.as_ref() {
                    result = re.replace_all(code, option_value).into_owned();
                }
                let mut split = result.split_whitespace();
                let first_part = u32::from_str_radix(split.next().unwrap(), 16).unwrap();
                decoded_cheat.push(DecodedCheat {
                    code_type: (first_part >> 24) as u8,
                    address: first_part & 0x00FFFFFF,
                    data: u16::from_str_radix(split.next().unwrap(), 16).unwrap(),
                });
            }
            device.cheats.cheats.push(decoded_cheat);
        } else {
            println!("Could not find cheat: {}", cheat_setting.0);
        }
    }
}

fn write_byte(device: &mut device::Device, cheat_line: &DecodedCheat) {
    *device
        .rdram
        .mem
        .get_mut(cheat_line.address as usize ^ device.byte_swap)
        .unwrap_or(&mut 0) = cheat_line.data as u8;
}

fn write_half(device: &mut device::Device, cheat_line: &DecodedCheat) {
    *device
        .rdram
        .mem
        .get_mut(cheat_line.address as usize ^ device.byte_swap)
        .unwrap_or(&mut 0) = cheat_line.data as u8;
    *device
        .rdram
        .mem
        .get_mut((cheat_line.address + 1) as usize ^ device.byte_swap)
        .unwrap_or(&mut 0) = (cheat_line.data >> 8) as u8;
}

fn equal_byte(device: &mut device::Device, cheat_line: &DecodedCheat) -> bool {
    let current_value = *device
        .rdram
        .mem
        .get(cheat_line.address as usize ^ device.byte_swap)
        .unwrap_or(&0);
    current_value == (cheat_line.data as u8)
}

fn equal_half(device: &mut device::Device, cheat_line: &DecodedCheat) -> bool {
    let current_value1 = *device
        .rdram
        .mem
        .get(cheat_line.address as usize ^ device.byte_swap)
        .unwrap_or(&0);
    let current_value2 = *device
        .rdram
        .mem
        .get((cheat_line.address + 1) as usize ^ device.byte_swap)
        .unwrap_or(&0);
    current_value1 == (cheat_line.data as u8) && current_value2 == (cheat_line.data >> 8) as u8
}

pub fn execute_cheats(device: &mut device::Device) {
    let cheats = device.cheats.cheats.clone();
    for cheat in cheats.iter() {
        let mut valid = true;
        for cheat_line in cheat.iter() {
            match cheat_line.code_type {
                0x80 | 0xA0 => {
                    if valid {
                        write_byte(device, cheat_line);
                    }
                    valid = true;
                }
                0x81 | 0xA1 => {
                    if valid {
                        write_half(device, cheat_line);
                    }
                    valid = true;
                }
                0xF0 => {
                    if device.cheats.boot && valid {
                        write_byte(device, cheat_line);
                    }
                    valid = true;
                }
                0xF1 => {
                    if device.cheats.boot && valid {
                        write_half(device, cheat_line);
                    }
                    valid = true;
                }
                0xD0 => {
                    valid = equal_byte(device, cheat_line);
                }
                0xD1 => {
                    valid = equal_half(device, cheat_line);
                }
                0xD2 => {
                    valid = !equal_byte(device, cheat_line);
                }
                0xD3 => {
                    valid = !equal_half(device, cheat_line);
                }
                _ => panic!("Unknown cheat code type: {:X}", cheat_line.code_type),
            }
        }
    }
    device.cheats.boot = false;
}
