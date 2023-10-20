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

pub struct Saves {
    pub eeprom: (Vec<u8>, bool),
    pub sram: (Vec<u8>, bool),
    pub flash: (Vec<u8>, bool),
    pub mempak: (Vec<u8>, bool),
    pub romsave: (serde_json::Map<String, serde_json::Value>, bool),
}

pub fn get_save_type(game_id: &str) -> Vec<SaveTypes> {
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
            return vec![SaveTypes::Eeprom16k]
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
            return vec![SaveTypes::Flash]
        }
        "NPQ" // Powerpuff Girls: Chemical X Traction
        => {return vec![];}
        _ => {
            return vec![SaveTypes::Eeprom4k, SaveTypes::Sram]
        }
    }
}

pub fn init(ui: &mut ui::Ui) {
    let id = ui.game_id.as_str();
    ui.save_type = get_save_type(id);

    let base_path = dirs::data_dir().unwrap().join("gopher64").join("saves");

    let result = std::fs::create_dir_all(base_path.clone());
    if result.is_err() {
        panic!("could not create save dir")
    }

    ui.paths.eep_file_path = base_path.clone();
    ui.paths
        .eep_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".eep");

    ui.paths.sra_file_path = base_path.clone();
    ui.paths
        .sra_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".sra");

    ui.paths.fla_file_path = base_path.clone();
    ui.paths
        .fla_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".fla");

    ui.paths.pak_file_path = base_path.clone();
    ui.paths
        .pak_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".mpk");

    ui.paths.romsave_file_path = base_path.clone();
    ui.paths
        .romsave_file_path
        .push(ui.game_id.to_owned() + "-" + &ui.game_hash + ".romsave");
}

pub fn load_saves(ui: &mut ui::Ui) {
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
        ui.saves.romsave.0 = serde_json::from_slice(romsave.unwrap().as_ref()).unwrap();
    }
}

pub fn write_rom_save(ui: &mut ui::Ui) {
    let f = std::fs::File::create(ui.paths.romsave_file_path.clone()).unwrap();
    serde_json::to_writer(f, &ui.saves.romsave).unwrap();
}

pub fn write_saves(ui: &mut ui::Ui) {
    if ui.saves.eeprom.1 {
        write_save(ui, SaveTypes::Eeprom4k)
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

pub fn write_save(ui: &mut ui::Ui, save_type: SaveTypes) {
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
