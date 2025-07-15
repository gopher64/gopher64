use crate::device;
use crate::ui;
use crate::ui::gui::AppWindow;
use crate::ui::gui::ErrorDialog;
use slint::ComponentHandle;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CheatData {
    note: String,
    pub data: Vec<String>,
    pub options: Option<std::collections::BTreeMap<String, String>>,
}

pub type Cheats = std::collections::BTreeMap<String, std::collections::BTreeMap<String, CheatData>>;

pub fn cheats_window(app: &AppWindow) {
    let weak = app.as_weak();
    app.on_cheats_select_rom_clicked(move |rom_dir| {
        let select_rom = rfd::AsyncFileDialog::new()
            .set_title("Select ROM")
            .add_filter("ROM files", &ui::gui::N64_EXTENSIONS)
            .set_directory(rom_dir)
            .pick_file();
        let weak = weak.clone();
        tokio::spawn(async move {
            if let Some(file) = select_rom.await
                && let Some(rom_contents) = device::get_rom_contents(file.path())
            {
                let game_crc = ui::storage::get_game_crc(&rom_contents);
                let game_name = ui::storage::get_game_name(&rom_contents);
                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_cheat_game_name(game_name.into());
                })
                .unwrap();
                let cheats: Cheats =
                    serde_json::from_slice(include_bytes!("../../data/cheats.json")).unwrap();
                if let Some(cheat) = cheats.get(&game_crc) {
                    let cheat = cheat.clone();
                    weak.upgrade_in_event_loop(move |handle| {
                        let cheat_settings = ui::config::Cheats::new();
                        let game_cheats = cheat_settings.cheats.get(&game_crc).cloned();
                        handle.set_cheat_game_crc(game_crc.into());
                        let cheats_vec = slint::VecModel::default();
                        for item in cheat.iter() {
                            let mut cheat_enabled = false;
                            if let Some(game_cheat) = game_cheats.as_ref()
                                && game_cheat.contains_key(item.0)
                            {
                                cheat_enabled = true;
                            }
                            let options_vec = slint::VecModel::default();
                            if let Some(options) = item.1.options.as_ref() {
                                for option in options.iter() {
                                    let mut option_enabled = false;
                                    if let Some(game_cheat) = game_cheats.as_ref()
                                        && game_cheat.contains_key(item.0)
                                        && let Some(opt) = game_cheat.get(item.0)
                                        && let Some(opt) = opt
                                        && opt == option.0
                                    {
                                        option_enabled = true;
                                    }
                                    options_vec.push((option_enabled, option.0.into()));
                                }
                            }
                            let options = slint::ModelRc::from(std::rc::Rc::new(options_vec));
                            cheats_vec.push((
                                item.0.clone().into(),
                                cheat_enabled,
                                item.1.clone().note.into(),
                                options,
                            ));
                        }
                        #[allow(clippy::type_complexity)]
                        let cheats_model: std::rc::Rc<
                            slint::VecModel<(
                                slint::SharedString,
                                bool,
                                slint::SharedString,
                                slint::ModelRc<(bool, slint::SharedString)>,
                            )>,
                        > = std::rc::Rc::new(cheats_vec);
                        handle.set_cheats(slint::ModelRc::from(cheats_model));
                    })
                    .unwrap();
                } else {
                    clear_cheats(&weak, false);
                }
            } else {
                clear_cheats(&weak, true);
                weak.upgrade_in_event_loop(move |_handle| {
                    let message_dialog = ErrorDialog::new().unwrap();
                    let weak_dialog = message_dialog.as_weak();
                    message_dialog.on_close_clicked(move || {
                        weak_dialog.unwrap().window().hide().unwrap();
                    });
                    message_dialog.set_text("Could not read ROM".into());
                    message_dialog.show().unwrap();
                })
                .unwrap();
            }
        });
    });

    let weak = app.as_weak();
    app.on_cheats_clear_clicked(move || {
        let mut cheats = ui::config::Cheats::new();
        cheats.cheats.clear();
        clear_cheats(&weak, true);
    });

    app.on_cheat_toggled(move |game_crc, cheat_name, option, enabled| {
        let mut cheats = ui::config::Cheats::new();
        let mut game_cheats = cheats
            .cheats
            .remove(&game_crc.to_string())
            .unwrap_or_default();
        if enabled {
            let cheat_option = if option.is_empty() {
                None
            } else {
                Some(option.into())
            };
            game_cheats.insert(cheat_name.into(), cheat_option);
        } else {
            game_cheats.remove(&cheat_name.to_string());
        }
        cheats.cheats.insert(game_crc.into(), game_cheats);
    });
}

fn clear_cheats(weak: &slint::Weak<AppWindow>, clear_name: bool) {
    weak.upgrade_in_event_loop(move |handle| {
        let cheats_vec = slint::VecModel::default();
        #[allow(clippy::type_complexity)]
        let cheats_model: std::rc::Rc<
            slint::VecModel<(
                slint::SharedString,
                bool,
                slint::SharedString,
                slint::ModelRc<(bool, slint::SharedString)>,
            )>,
        > = std::rc::Rc::new(cheats_vec);
        handle.set_cheats(slint::ModelRc::from(cheats_model));
        if clear_name {
            handle.set_cheat_game_name("".into());
        }
    })
    .unwrap();
}
