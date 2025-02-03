use crate::netplay;
use crate::ui;

#[derive(PartialEq)]
pub enum SaveTypes {
    Eeprom4k,
    Eeprom16k,
    Sram,
    Flash,
    Mempak,
    Romsave,
}

pub struct Paths {
    pub eep_file_path: std::path::PathBuf,
    pub sra_file_path: std::path::PathBuf,
    pub fla_file_path: std::path::PathBuf,
    pub pak_file_path: std::path::PathBuf,
    pub romsave_file_path: std::path::PathBuf,
}

// the bool indicates whether the save has been written to
// if that is the case, it will be flushed to the disk when the program closes
pub struct Saves {
    pub eeprom: (Vec<u8>, bool),
    pub sram: (Vec<u8>, bool),
    pub flash: (Vec<u8>, bool),
    pub mempak: (Vec<u8>, bool),
    pub sdcard: (Vec<u8>, bool),
    pub romsave: (std::collections::HashMap<u32, u8>, bool),
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
            4 => panic!("Unsupported save type: {:X}", save_type),
            5 => return vec![SaveTypes::Flash],
            6 => panic!("Unsupported save type: {:X}", save_type),
            _ => panic!("Unknown save type: {:X}", save_type),
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

pub fn init(ui: &mut ui::Ui, rom: &[u8]) {
    let id = ui.game_id.as_str();
    ui.save_type = get_save_type(rom, id);

    let base_path = ui.dirs.data_dir.join("saves");

    ui.paths.eep_file_path.clone_from(&base_path);
    ui.paths
        .eep_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".eep");

    ui.paths.sra_file_path.clone_from(&base_path);
    ui.paths
        .sra_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".sra");

    ui.paths.fla_file_path.clone_from(&base_path);
    ui.paths
        .fla_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".fla");

    ui.paths.pak_file_path.clone_from(&base_path);
    ui.paths
        .pak_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".mpk");

    ui.paths.romsave_file_path.clone_from(&base_path);
    ui.paths
        .romsave_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".romsave");
}

pub fn load_saves(ui: &mut ui::Ui, netplay: &mut Option<netplay::Netplay>) {
    ui.saves.sdcard.0 = std::fs::read("/var/home/loganmc10/Downloads/test.img").unwrap();

    if netplay.is_none() || netplay.as_ref().unwrap().player_number == 0 {
        let eep = std::fs::read(&mut ui.paths.eep_file_path);
        if eep.is_ok() {
            ui.saves.eeprom.0 = eep.unwrap();
        }
        let sra = std::fs::read(&mut ui.paths.sra_file_path);
        if sra.is_ok() {
            ui.saves.sram.0 = sra.unwrap();
        }
        let fla = std::fs::read(&mut ui.paths.fla_file_path);
        if fla.is_ok() {
            ui.saves.flash.0 = fla.unwrap();
        }
        let mempak = std::fs::read(&mut ui.paths.pak_file_path);
        if mempak.is_ok() {
            ui.saves.mempak.0 = mempak.unwrap();
        }
        let romsave = std::fs::read(&mut ui.paths.romsave_file_path);
        if romsave.is_ok() {
            ui.saves.romsave.0 = postcard::from_bytes(romsave.unwrap().as_ref()).unwrap();
        }
    }

    if netplay.is_some() {
        if netplay.as_ref().unwrap().player_number == 0 {
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "eep",
                &ui.saves.eeprom.0,
                ui.saves.eeprom.0.len(),
            );
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "sra",
                &ui.saves.sram.0,
                ui.saves.sram.0.len(),
            );
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "fla",
                &ui.saves.flash.0,
                ui.saves.flash.0.len(),
            );
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "mpk",
                &ui.saves.mempak.0,
                ui.saves.mempak.0.len(),
            );
            let mut romsave_bytes: Vec<u8> = vec![];
            if !ui.saves.romsave.0.is_empty() {
                romsave_bytes = postcard::to_stdvec(&ui.saves.romsave.0).unwrap();
            }
            netplay::send_save(
                netplay.as_mut().unwrap(),
                "rom",
                &romsave_bytes,
                romsave_bytes.len(),
            );
        } else {
            netplay::receive_save(netplay.as_mut().unwrap(), "eep", &mut ui.saves.eeprom.0);
            netplay::receive_save(netplay.as_mut().unwrap(), "sra", &mut ui.saves.sram.0);
            netplay::receive_save(netplay.as_mut().unwrap(), "fla", &mut ui.saves.flash.0);
            netplay::receive_save(netplay.as_mut().unwrap(), "mpk", &mut ui.saves.mempak.0);
            let mut romsave_bytes: Vec<u8> = vec![];
            netplay::receive_save(netplay.as_mut().unwrap(), "rom", &mut romsave_bytes);
            if !romsave_bytes.is_empty() {
                ui.saves.romsave.0 = postcard::from_bytes(&romsave_bytes).unwrap();
            }
        }
    }
}

fn write_rom_save(ui: &ui::Ui) {
    let data = postcard::to_stdvec(&ui.saves.romsave.0).unwrap();
    std::fs::write(ui.paths.romsave_file_path.clone(), data).unwrap();
}

pub fn write_saves(ui: &ui::Ui, netplay: &Option<netplay::Netplay>) {
    if netplay.is_none() || netplay.as_ref().unwrap().player_number == 0 {
        if ui.saves.eeprom.1 {
            write_save(ui, SaveTypes::Eeprom16k)
        }
        if ui.saves.sram.1 {
            write_save(ui, SaveTypes::Sram)
        }
        if ui.saves.flash.1 {
            write_save(ui, SaveTypes::Flash)
        }
        if ui.saves.mempak.1 {
            write_save(ui, SaveTypes::Mempak)
        }
        if ui.saves.romsave.1 {
            write_save(ui, SaveTypes::Romsave)
        }
    }
}

fn write_save(ui: &ui::Ui, save_type: SaveTypes) {
    let path: &std::path::Path;
    let data: &Vec<u8>;
    match save_type {
        SaveTypes::Eeprom4k | SaveTypes::Eeprom16k => {
            path = ui.paths.eep_file_path.as_ref();
            data = ui.saves.eeprom.0.as_ref();
        }
        SaveTypes::Sram => {
            path = ui.paths.sra_file_path.as_ref();
            data = ui.saves.sram.0.as_ref();
        }
        SaveTypes::Flash => {
            path = ui.paths.fla_file_path.as_ref();
            data = ui.saves.flash.0.as_ref();
        }
        SaveTypes::Mempak => {
            path = ui.paths.pak_file_path.as_ref();
            data = ui.saves.mempak.0.as_ref();
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
