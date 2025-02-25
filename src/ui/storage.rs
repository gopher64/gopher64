use crate::{device, netplay, ui};
use std::io::Read;
use std::io::Write;

#[derive(PartialEq)]
pub enum SaveTypes {
    Eeprom4k,
    Eeprom16k,
    Sram,
    Flash,
    Mempak,
    Sdcard,
    Romsave,
}

pub struct Paths {
    pub eep_file_path: std::path::PathBuf,
    pub sra_file_path: std::path::PathBuf,
    pub fla_file_path: std::path::PathBuf,
    pub pak_file_path: std::path::PathBuf,
    pub sdcard_file_path: std::path::PathBuf,
    pub romsave_file_path: std::path::PathBuf,
    pub savestate_file_path: std::path::PathBuf,
}

// the bool indicates whether the save has been written to
// if that is the case, it will be flushed to the disk when the program closes
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Save {
    pub data: Vec<u8>,
    pub written: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RomSave {
    pub data: std::collections::HashMap<u32, u8>,
    pub written: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Saves {
    pub eeprom: Save,
    pub sram: Save,
    pub flash: Save,
    pub mempak: Save,
    pub sdcard: Save,
    pub romsave: RomSave,
    pub write_to_disk: bool,
}

fn get_save_type(rom: &[u8], game_id: &str) -> Vec<SaveTypes> {
    let header_type = std::str::from_utf8(rom[0x3C..0x3E].try_into().unwrap());
    if header_type.is_ok() && header_type.unwrap() == "ED" {
        let save_type = rom[0x3F] >> 4;
        match save_type {
            0 => return vec![],
            1 => return vec![SaveTypes::Eeprom4k],
            2 => return vec![SaveTypes::Eeprom16k],
            3 => return vec![SaveTypes::Sram],
            4 => panic!("Unsupported save type: {}", save_type),
            5 => return vec![SaveTypes::Flash],
            6 => panic!("Unsupported save type: {}", save_type),
            _ => panic!("Unknown save type: {}", save_type),
        }
    }
    match game_id {
        "NB7" | // Banjo-Tooie [Banjo to Kazooie no Daiboken 2 (J)]
        "NGT" | // City Tour GrandPrix - Zen Nihon GT Senshuken
        "NFU" | // Conker's Bad Fur Day
        "NCW" | // Cruis'n World
        "NCZ" | // Custom Robo V2
        "ND6" | // Densha de Go! 64
        "NDO" | // Donkey Kong 64
        "ND2" | // Doraemon 2: Nobita to Hikari no Shinden
        "N3D" | // Doraemon 3: Nobita no Machi SOS!
        "NMX" | // Excitebike 64
        "NGC" | // GT 64: Championship Edition
        "NIM" | // Ide Yosuke no Mahjong Juku
        "NNB" | // Kobe Bryant in NBA Courtside
        "NMV" | // Mario Party 3
        "NM8" | // Mario Tennis
        "NEV" | // Neon Genesis Evangelion
        "NPP" | // Parlor! Pro 64: Pachinko Jikki Simulation Game
        "NUB" | // PD Ultraman Battle Collection 64
        "NPD" | // Perfect Dark
        "NRZ" | // Ridge Racer 64
        "NR7" | // Robot Poncots 64: 7tsu no Umi no Caramel
        "NEP" | // Star Wars Episode I: Racer
        "NYS"   // Yoshi's Story
        => {
            vec![SaveTypes::Eeprom16k]
        }
        "NCC" | // Command & Conquer
        "NDA" | // Derby Stallion 64
        "NAF" | // Doubutsu no Mori
        "NJF" | // Jet Force Gemini [Star Twins (J)]
        "NKJ" | // Ken Griffey Jr.'s Slugfest
        "NZS" | // Legend of Zelda: Majora's Mask [Zelda no Densetsu - Mujura no Kamen (J)]
        "NM6" | // Mega Man 64
        "NCK" | // NBA Courtside 2 featuring Kobe Bryant
        "NMQ" | // Paper Mario
        "NPN" | // Pokemon Puzzle League
        "NPF" | // Pokemon Snap [Pocket Monsters Snap (J)]
        "NPO" | // Pokemon Stadium
        "CP2" | // Pocket Monsters Stadium 2 (J)
        "NP3" | // Pokemon Stadium 2 [Pocket Monsters Stadium - Kin Gin (J)]
        "NRH" | // Rockman Dash - Hagane no Boukenshin (J)
        "NSQ" | // StarCraft 64
        "NT9" | // Tigger's Honey Hunt
        "NW4" | // WWF No Mercy
        "NDP"   // Dinosaur Planet (Unlicensed)
        =>{
            vec![SaveTypes::Flash]
        }
        "NPQ" // Powerpuff Girls: Chemical X Traction
        => {vec![]}
        _ => {
            vec![SaveTypes::Eeprom4k, SaveTypes::Sram]
        }
    }
}

pub fn get_game_name(rom: &[u8]) -> String {
    let mut game_name = "".to_owned();
    let header_value = std::str::from_utf8(&rom[0x20..0x34]);
    if header_value.is_ok() {
        let re = regex::Regex::new(r"[^a-zA-Z0-9_ -]").unwrap();
        game_name = re
            .replace_all(header_value.unwrap(), "")
            .trim()
            .replace('\0', "");
    }
    game_name
}

pub fn init(ui: &mut ui::Ui, rom: &[u8]) {
    ui.save_type = get_save_type(rom, &ui.game_id);

    let saves_path = ui.dirs.data_dir.join("saves");

    let states_path = ui.dirs.data_dir.join("states");

    let game_name = get_game_name(rom);

    let prefix = if game_name.is_empty() {
        &ui.game_id
    } else {
        &game_name
    };

    ui.paths.eep_file_path.clone_from(&saves_path);
    ui.paths
        .eep_file_path
        .push(prefix.to_owned() + "-" + &ui.game_hash + ".eep");

    ui.paths.sra_file_path.clone_from(&saves_path);
    ui.paths
        .sra_file_path
        .push(prefix.to_owned() + "-" + &ui.game_hash + ".sra");

    ui.paths.fla_file_path.clone_from(&saves_path);
    ui.paths
        .fla_file_path
        .push(prefix.to_owned() + "-" + &ui.game_hash + ".fla");

    ui.paths.pak_file_path.clone_from(&saves_path);
    ui.paths
        .pak_file_path
        .push(prefix.to_owned() + "-" + &ui.game_hash + ".mpk");

    ui.paths.sdcard_file_path.clone_from(&saves_path);
    ui.paths
        .sdcard_file_path
        .push(prefix.to_owned() + "-" + &ui.game_hash + ".img");

    ui.paths.romsave_file_path.clone_from(&saves_path);
    ui.paths
        .romsave_file_path
        .push(prefix.to_owned() + "-" + &ui.game_hash + ".romsave");

    ui.paths.savestate_file_path.clone_from(&states_path);
    ui.paths
        .savestate_file_path
        .push(prefix.to_owned() + "-" + &ui.game_hash + ".state");
}

pub fn load_saves(ui: &mut ui::Ui, netplay: &mut Option<netplay::Netplay>) {
    if netplay.is_none() || netplay.as_ref().unwrap().player_number == 0 {
        let eep = std::fs::read(&mut ui.paths.eep_file_path);
        if eep.is_ok() {
            ui.saves.eeprom.data = eep.unwrap();
        }
        let sra = std::fs::read(&mut ui.paths.sra_file_path);
        if sra.is_ok() {
            ui.saves.sram.data = sra.unwrap();
        }
        let fla = std::fs::read(&mut ui.paths.fla_file_path);
        if fla.is_ok() {
            ui.saves.flash.data = fla.unwrap();
        }
        let mempak = std::fs::read(&mut ui.paths.pak_file_path);
        if mempak.is_ok() {
            ui.saves.mempak.data = mempak.unwrap();
        }
        let sdcard = std::fs::read(&mut ui.paths.sdcard_file_path);
        if sdcard.is_ok() {
            ui.saves.sdcard.data = sdcard.unwrap();
        }
        let romsave = std::fs::read(&mut ui.paths.romsave_file_path);
        if romsave.is_ok() {
            ui.saves.romsave.data = postcard::from_bytes(romsave.unwrap().as_ref()).unwrap();
        }
    }

    if netplay.is_some() {
        if netplay.as_ref().unwrap().player_number == 0 {
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "eep",
                &ui.saves.eeprom.data,
                ui.saves.eeprom.data.len(),
            );
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "sra",
                &ui.saves.sram.data,
                ui.saves.sram.data.len(),
            );
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "fla",
                &ui.saves.flash.data,
                ui.saves.flash.data.len(),
            );
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "mpk",
                &ui.saves.mempak.data,
                ui.saves.mempak.data.len(),
            );

            let mut compressed_sd = Vec::new();
            if !ui.saves.sdcard.data.is_empty() {
                compressed_sd = compress_file(&[(&ui.saves.sdcard.data, "save")]);
            }
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "img",
                &compressed_sd,
                compressed_sd.len(),
            );

            let mut compressed_romsave = Vec::new();
            if !ui.saves.romsave.data.is_empty() {
                compressed_romsave = compress_file(&[(
                    &postcard::to_stdvec(&ui.saves.romsave.data).unwrap(),
                    "save",
                )]);
            }
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "rom",
                &compressed_romsave,
                compressed_romsave.len(),
            );
        } else {
            netplay::receive_save(netplay.as_mut().unwrap(), "eep", &mut ui.saves.eeprom.data);
            netplay::receive_save(netplay.as_mut().unwrap(), "sra", &mut ui.saves.sram.data);
            netplay::receive_save(netplay.as_mut().unwrap(), "fla", &mut ui.saves.flash.data);
            netplay::receive_save(netplay.as_mut().unwrap(), "mpk", &mut ui.saves.mempak.data);

            let mut compressed_sd = Vec::new();
            netplay::receive_save(netplay.as_mut().unwrap(), "img", &mut compressed_sd);
            if !compressed_sd.is_empty() {
                ui.saves.sdcard.data = decompress_file(&compressed_sd, "save");
            }

            let mut compressed_romsave = Vec::new();
            netplay::receive_save(netplay.as_mut().unwrap(), "rom", &mut compressed_romsave);
            if !compressed_romsave.is_empty() {
                let romsave_bytes = decompress_file(&compressed_romsave, "save");
                ui.saves.romsave.data = postcard::from_bytes(&romsave_bytes).unwrap();
            }
        }
    }
}

pub fn decompress_file(input: &[u8], name: &str) -> Vec<u8> {
    let mut decompressed_file = Vec::new();
    {
        let mut reader = zip::ZipArchive::new(std::io::Cursor::new(input)).unwrap();
        let mut file = reader.by_name(name).unwrap();
        file.read_to_end(&mut decompressed_file).unwrap();
    }
    decompressed_file
}

pub fn compress_file(data: &[(&[u8], &str)]) -> Vec<u8> {
    let mut compressed_file = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut compressed_file));
        for item in data {
            writer
                .start_file(
                    item.1,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Zstd),
                )
                .unwrap();
            writer.write_all(item.0).unwrap();
        }
    }
    compressed_file
}

fn write_rom_save(ui: &ui::Ui) {
    let data = postcard::to_stdvec(&ui.saves.romsave.data).unwrap();
    std::fs::write(ui.paths.romsave_file_path.clone(), data).unwrap();
}

fn writeback_sdcard(device: &mut device::Device) {
    let mut length = 0;
    let mut save_data: &[u8] = &[];
    if device.ui.saves.eeprom.written {
        length = device.ui.saves.eeprom.data.len() / 512;
        save_data = device.ui.saves.eeprom.data.as_ref();
    } else if device.ui.saves.sram.written {
        length = device.ui.saves.sram.data.len() / 512;
        save_data = device.ui.saves.sram.data.as_ref();
    } else if device.ui.saves.flash.written {
        length = device.ui.saves.flash.data.len() / 512;
        save_data = device.ui.saves.flash.data.as_ref();
    }

    for i in 0..length {
        let offset = device.cart.sc64.writeback_sector[i] as usize * 512;
        device.ui.saves.sdcard.data[offset..offset + 512]
            .copy_from_slice(&save_data[i * 512..(i + 1) * 512]);
    }
    device.ui.saves.sdcard.written = true;
}

pub fn write_saves(device: &mut device::Device) {
    if device.netplay.is_none() || device.netplay.as_ref().unwrap().player_number == 0 {
        if device.ui.saves.write_to_disk {
            if device.ui.saves.eeprom.written {
                write_save(&device.ui, SaveTypes::Eeprom16k)
            }
            if device.ui.saves.sram.written {
                write_save(&device.ui, SaveTypes::Sram)
            }
            if device.ui.saves.flash.written {
                write_save(&device.ui, SaveTypes::Flash)
            }
            if device.ui.saves.romsave.written {
                write_save(&device.ui, SaveTypes::Romsave)
            }
        } else {
            writeback_sdcard(device)
        }
        if device.ui.saves.mempak.written {
            write_save(&device.ui, SaveTypes::Mempak)
        }
        if device.ui.saves.sdcard.written {
            write_save(&device.ui, SaveTypes::Sdcard)
        }
    }
}

fn write_save(ui: &ui::Ui, save_type: SaveTypes) {
    let path: &std::path::Path;
    let data: &Vec<u8>;
    match save_type {
        SaveTypes::Eeprom4k | SaveTypes::Eeprom16k => {
            path = ui.paths.eep_file_path.as_ref();
            data = ui.saves.eeprom.data.as_ref();
        }
        SaveTypes::Sram => {
            path = ui.paths.sra_file_path.as_ref();
            data = ui.saves.sram.data.as_ref();
        }
        SaveTypes::Flash => {
            path = ui.paths.fla_file_path.as_ref();
            data = ui.saves.flash.data.as_ref();
        }
        SaveTypes::Mempak => {
            path = ui.paths.pak_file_path.as_ref();
            data = ui.saves.mempak.data.as_ref();
        }
        SaveTypes::Sdcard => {
            path = ui.paths.sdcard_file_path.as_ref();
            data = ui.saves.sdcard.data.as_ref();
        }
        SaveTypes::Romsave => {
            write_rom_save(ui);
            return;
        }
    }
    let result = std::fs::write(path, data);
    if result.is_err() {
        panic!(
            "could not save {} {}",
            path.display(),
            result.err().unwrap()
        )
    }
}
