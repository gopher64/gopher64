use crate::device;
use crate::ui;

struct DecodedCheat {
    code_type: u8,
    address: u32,
    data: u16,
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
    //let decoded_cheats: Vec<DecodedCheat> = vec![];
    for cheat_setting in cheat_settings.iter() {
        if let Some(cheat_data) = cheats.get(cheat_setting.0) {
            let mut option_value = "";
            if let Some(option) = cheat_setting.1 {
                if let Some(option_data) = cheat_data.options.as_ref() {
                    if let Some(found_option_value) = option_data.get(option) {
                        option_value = found_option_value;
                    } else {
                        panic!("Cheat option: {option} not found");
                    }
                } else {
                    panic!("No options for cheat: {}", cheat_setting.0);
                }
            }

            println!(
                "Cheat: {} Option: {}",
                cheat_setting.0,
                cheat_setting.1.clone().unwrap_or("none".to_string())
            );
            for code in cheat_data.data.iter() {
                let result = re.replace_all(code, option_value);
                let mut split = result.split_whitespace();
                let first_part = u32::from_str_radix(split.next().unwrap(), 16).unwrap();
                let decoded_cheat = DecodedCheat {
                    code_type: (first_part >> 24) as u8,
                    address: first_part & 0x00FFFFFF,
                    data: u16::from_str_radix(split.next().unwrap(), 16).unwrap(),
                };
                println!(
                    "Cheat code: {:02X} {:08X} {:04X}",
                    decoded_cheat.code_type, decoded_cheat.address, decoded_cheat.data
                );
            }
        }
    }
}
