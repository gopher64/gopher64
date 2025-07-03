use crate::device;
use crate::ui;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DecodedCheat {
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
            device.cheats.push(decoded_cheat);
        } else {
            println!("Could not find cheat: {}", cheat_setting.0);
        }
    }
}
