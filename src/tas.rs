use std::io::Read;

use crate::ui;

pub fn load_tas(tas_file: String) -> Vec<u32> {
    let path = std::path::Path::new(&tas_file);
    if path
        .extension()
        .unwrap_or_default()
        .eq_ignore_ascii_case("m64")
    {
        load_m64(tas_file)
    } else if path
        .extension()
        .unwrap_or_default()
        .eq_ignore_ascii_case("bk2")
    {
        let mut input_log = Vec::new();
        if let Ok(zip_file) = std::fs::File::open(tas_file)
            && let Ok(mut archive) = zip::ZipArchive::new(zip_file)
            && let Ok(mut file) = archive.by_name("Input Log.txt")
        {
            file.read_to_end(&mut input_log)
                .expect("could not read zip file");
        }
        if !input_log.is_empty() {
            load_bk2(String::from_utf8(input_log).unwrap())
        } else {
            eprintln!("could not read TAS file");
            Vec::new()
        }
    } else {
        eprintln!("could not load TAS file");
        Vec::new()
    }
}

fn load_bk2(input_log: String) -> Vec<u32> {
    let mut tas = Vec::new();
    for line in input_log.lines() {
        if line.starts_with("|") {
            let mut keys: u32 = 0;
            let data = line.split('|').nth(2).unwrap();
            let x_axis = data
                .split(',')
                .nth(0)
                .unwrap()
                .trim_start()
                .parse::<i8>()
                .unwrap();
            let y_axis = data
                .split(',')
                .nth(1)
                .unwrap()
                .trim_start()
                .parse::<i8>()
                .unwrap();
            let buttons = data.split(',').nth(2).unwrap();
            let u_dpad = buttons.chars().nth(4).unwrap() != '.';
            let d_dpad = buttons.chars().nth(5).unwrap() != '.';
            let l_dpad = buttons.chars().nth(6).unwrap() != '.';
            let r_dpad = buttons.chars().nth(7).unwrap() != '.';
            let start_button = buttons.chars().nth(8).unwrap() != '.';
            let z_trig = buttons.chars().nth(9).unwrap() != '.';
            let b_button = buttons.chars().nth(10).unwrap() != '.';
            let a_button = buttons.chars().nth(11).unwrap() != '.';
            let u_cbutton = buttons.chars().nth(12).unwrap() != '.';
            let d_cbutton = buttons.chars().nth(13).unwrap() != '.';
            let l_cbutton = buttons.chars().nth(14).unwrap() != '.';
            let r_cbutton = buttons.chars().nth(15).unwrap() != '.';
            let l_trig = buttons.chars().nth(16).unwrap() != '.';
            let r_trig = buttons.chars().nth(17).unwrap() != '.';
            keys |= (u_dpad as u32) << ui::input::U_DPAD;
            keys |= (d_dpad as u32) << ui::input::D_DPAD;
            keys |= (l_dpad as u32) << ui::input::L_DPAD;
            keys |= (r_dpad as u32) << ui::input::R_DPAD;
            keys |= (start_button as u32) << ui::input::START_BUTTON;
            keys |= (z_trig as u32) << ui::input::Z_TRIG;
            keys |= (b_button as u32) << ui::input::B_BUTTON;
            keys |= (a_button as u32) << ui::input::A_BUTTON;
            keys |= (u_cbutton as u32) << ui::input::U_CBUTTON;
            keys |= (d_cbutton as u32) << ui::input::D_CBUTTON;
            keys |= (l_cbutton as u32) << ui::input::L_CBUTTON;
            keys |= (r_cbutton as u32) << ui::input::R_CBUTTON;
            keys |= (l_trig as u32) << ui::input::L_TRIG;
            keys |= (r_trig as u32) << ui::input::R_TRIG;
            keys |= (x_axis as u8 as u32) << ui::input::X_AXIS_SHIFT;
            keys |= (y_axis as u8 as u32) << ui::input::Y_AXIS_SHIFT;
            tas.push(keys);
        }
    }
    println!("TAS file loaded successfully");
    tas
}

fn load_m64(tas_file: String) -> Vec<u32> {
    if let Ok(m64_file) = std::fs::read(tas_file) {
        let signature = u32::from_le_bytes(m64_file[0..4].try_into().unwrap());
        let version = u32::from_le_bytes(m64_file[4..8].try_into().unwrap());
        let num_controllers = m64_file[0x15];
        let start_type = u16::from_le_bytes(m64_file[0x1c..0x1e].try_into().unwrap());

        if signature == 0x1a34364d && version == 3 && num_controllers == 1 && start_type == 2 {
            println!("TAS file loaded successfully");
            m64_file[0x400..]
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
                .collect()
        } else {
            eprintln!("could not load m64 TAS file");
            Vec::new()
        }
    } else {
        eprintln!("could not read m64 TAS file");
        Vec::new()
    }
}
