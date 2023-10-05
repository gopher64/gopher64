use crate::ui;

pub enum SaveTypes {
    Eeprom,
}

pub struct Paths {
    pub eep_file_path: std::path::PathBuf,
}

pub struct Saves {
    pub eeprom: Vec<u8>,
}

pub fn init(ui: &mut ui::Ui, game_name: &str) {
    let mut base_path = dirs::data_dir().unwrap();
    base_path.push("gopher64");
    base_path.push("saves");

    let result = std::fs::create_dir_all(base_path.clone());
    if result.is_err() {
        panic!("could not create save dir")
    }

    ui.paths.eep_file_path = base_path;
    ui.paths.eep_file_path.push(game_name.to_owned() + ".eep");
}

pub fn load_saves(ui: &mut ui::Ui) {
    let eep = std::fs::read(&mut ui.paths.eep_file_path);
    if eep.is_ok() {
        ui.saves.eeprom = eep.unwrap();
    }
}

pub fn write_save(ui: &mut ui::Ui, save_type: SaveTypes) {
    let path: &std::path::Path;
    let data: &Vec<u8>;
    match save_type {
        SaveTypes::Eeprom => {
            path = ui.paths.eep_file_path.as_ref();
            data = ui.saves.eeprom.as_ref();
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
