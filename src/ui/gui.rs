use crate::device;
use crate::ui;
use eframe::egui;

pub struct GopherEguiApp {
    configure_profile: bool,
    profile_name: String,
    controllers: Vec<String>,
    selected_controller: [i32; 4],
    selected_profile: [String; 4],
    input_profiles: Vec<String>,
    controller_enabled: [bool; 4],
    upscale: bool,
    emulate_vru: bool,
    show_vru_dialog: bool,
    vru_window_receiver: Option<std::sync::mpsc::Receiver<Vec<String>>>,
    vru_word_notifier: Option<std::sync::mpsc::Sender<String>>,
    vru_word_list: Vec<String>,
}

fn get_input_profiles(game_ui: &ui::Ui) -> Vec<String> {
    let mut profiles = vec![];
    for key in game_ui.config.input.input_profiles.keys() {
        profiles.push((*key).clone())
    }
    profiles
}

fn get_controllers(game_ui: &ui::Ui) -> Vec<String> {
    let mut controllers: Vec<String> = vec![];

    let joystick_subsystem = game_ui.joystick_subsystem.as_ref().unwrap();
    let num_joysticks = joystick_subsystem.num_joysticks().unwrap();
    for i in 0..num_joysticks {
        controllers.push(joystick_subsystem.name_for_index(i).unwrap());
    }
    controllers
}

impl GopherEguiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> GopherEguiApp {
        add_japanese_font(&cc.egui_ctx);

        let game_ui = ui::Ui::new();
        let joystick_subsystem = game_ui.joystick_subsystem.as_ref().unwrap();
        let num_joysticks = joystick_subsystem.num_joysticks().unwrap();
        let mut guids: Vec<String> = vec![];
        for i in 0..num_joysticks {
            guids.push(joystick_subsystem.device_guid(i).unwrap().to_string());
        }
        let mut selected_controller = [-1, -1, -1, -1];
        for (pos, item) in game_ui
            .config
            .input
            .controller_assignment
            .iter()
            .enumerate()
        {
            if item.is_some() {
                for (guid_pos, guid) in guids.iter().enumerate() {
                    if item.as_deref().unwrap() == *guid {
                        selected_controller[pos] = guid_pos as i32;
                        break;
                    }
                }
            }
        }
        GopherEguiApp {
            configure_profile: false,
            profile_name: "".to_string(),
            selected_profile: game_ui.config.input.input_profile_binding.clone(),
            selected_controller,
            controllers: get_controllers(&game_ui),
            input_profiles: get_input_profiles(&game_ui),
            controller_enabled: game_ui.config.input.controller_enabled,
            upscale: game_ui.config.video.upscale,
            emulate_vru: game_ui.config.input.emulate_vru,
            show_vru_dialog: false,
            vru_window_receiver: None,
            vru_word_notifier: None,
            vru_word_list: Vec::new(),
        }
    }
}

fn save_config(
    game_ui: &mut ui::Ui,
    selected_controller: [i32; 4],
    selected_profile: [String; 4],
    controller_enabled: [bool; 4],
    upscale: bool,
    emulate_vru: bool,
) {
    let joystick_subsystem = game_ui.joystick_subsystem.as_ref().unwrap();
    for (pos, item) in selected_controller.iter().enumerate() {
        if *item != -1 {
            game_ui.config.input.controller_assignment[pos] = Some(
                joystick_subsystem
                    .device_guid(*item as u32)
                    .unwrap()
                    .to_string(),
            );
        } else {
            game_ui.config.input.controller_assignment[pos] = None
        }
    }

    game_ui.config.input.input_profile_binding = selected_profile;
    game_ui.config.input.controller_enabled = controller_enabled;

    game_ui.config.video.upscale = upscale;
    game_ui.config.input.emulate_vru = emulate_vru;
}

impl Drop for GopherEguiApp {
    fn drop(&mut self) {
        let mut game_ui = ui::Ui::new();
        save_config(
            &mut game_ui,
            self.selected_controller,
            self.selected_profile.clone(),
            self.controller_enabled,
            self.upscale,
            self.emulate_vru,
        );
    }
}

impl eframe::App for GopherEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.configure_profile {
            egui::Window::new("Configure Input Profile")
                // .open(&mut self.configure_profile)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let name_label = ui.label("Profile Name: ");
                        ui.text_edit_singleline(&mut self.profile_name)
                            .labelled_by(name_label.id);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Configure Profile").clicked() {
                            let profile_name = self.profile_name.clone();
                            execute(async {
                                let mut game_ui = ui::Ui::new();
                                ui::input::configure_input_profile(&mut game_ui, profile_name);
                            });
                            self.configure_profile = false;
                            if !self.profile_name.is_empty()
                                && !self.input_profiles.contains(&self.profile_name)
                            {
                                self.input_profiles.push(self.profile_name.clone())
                            }
                        };
                        if ui.button("Close").clicked() {
                            self.configure_profile = false
                        };
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.configure_profile {
                ui.disable()
            }

            if ui.button("Open ROM").clicked() {
                // Spawn dialog on main thread
                let task = rfd::AsyncFileDialog::new().pick_file();
                let selected_controller = self.selected_controller;
                let selected_profile = self.selected_profile.clone();
                let controller_enabled = self.controller_enabled;
                let upscale = self.upscale;
                let emulate_vru = self.emulate_vru;

                let (vru_window_notifier, vru_window_receiver): (
                    std::sync::mpsc::Sender<Vec<String>>,
                    std::sync::mpsc::Receiver<Vec<String>>,
                ) = std::sync::mpsc::channel();
                self.vru_window_receiver = Some(vru_window_receiver);

                let (vru_word_notifier, vru_word_receiver): (
                    std::sync::mpsc::Sender<String>,
                    std::sync::mpsc::Receiver<String>,
                ) = std::sync::mpsc::channel();
                self.vru_word_notifier = Some(vru_word_notifier);

                let gui_ctx = ctx.clone();
                execute(async move {
                    let file = task.await;

                    if let Some(file) = file {
                        let running_file = dirs::cache_dir()
                            .unwrap()
                            .join("gopher64")
                            .join("game_running");
                        if running_file.exists() {
                            return;
                        }
                        let _ = std::fs::File::create(running_file.clone());
                        let mut device = device::Device::new();
                        save_config(
                            &mut device.ui,
                            selected_controller,
                            selected_profile,
                            controller_enabled,
                            upscale,
                            emulate_vru,
                        );
                        device.vru.window_notifier = Some(vru_window_notifier);
                        device.vru.word_receiver = Some(vru_word_receiver);
                        device.vru.gui_ctx = Some(gui_ctx);
                        device::run_game(std::path::Path::new(file.path()), &mut device, false);
                        let _ = std::fs::remove_file(running_file.clone());
                    }
                });
            }

            if ui.button("Configure Input Profile").clicked() {
                let running_file = dirs::cache_dir()
                    .unwrap()
                    .join("gopher64")
                    .join("game_running");
                if running_file.exists() {
                    return;
                }
                self.configure_profile = true;
            }

            ui.add_space(32.0);
            ui.label("Controller Config:");
            egui::Grid::new("controller_config").show(ui, |ui| {
                ui.label("Port");
                ui.label("Enabled");
                ui.label("Profile");
                ui.label("Controller");
                ui.end_row();
                for i in 0..4 {
                    ui.label(format!("{}", i + 1));
                    ui.checkbox(&mut self.controller_enabled[i], "");

                    egui::ComboBox::from_id_salt(format!("profile-combo-{}", i))
                        .selected_text(self.selected_profile[i].clone())
                        .show_ui(ui, |ui| {
                            for j in 0..self.input_profiles.len() {
                                ui.selectable_value(
                                    &mut self.selected_profile[i],
                                    self.input_profiles[j].clone(),
                                    self.input_profiles[j].clone(),
                                );
                            }
                        });

                    let controller_text = if self.selected_controller[i] == -1 {
                        "None".to_string()
                    } else {
                        self.controllers[self.selected_controller[i] as usize].clone()
                    };
                    egui::ComboBox::from_id_salt(format!("controller-combo-{}", i))
                        .selected_text(controller_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_controller[i],
                                -1,
                                "None".to_string(),
                            );
                            for j in 0..self.controllers.len() {
                                ui.selectable_value(
                                    &mut self.selected_controller[i],
                                    j as i32,
                                    self.controllers[j].clone(),
                                );
                            }
                        });
                    ui.end_row();
                }
            });
            ui.add_space(32.0);
            ui.checkbox(&mut self.upscale, "High-Res Graphics");
            ui.checkbox(
                &mut self.emulate_vru,
                "Emulate VRU (connects VRU to controller port 4)",
            );
            ui.add_space(32.0);
            ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
        });

        if self.vru_window_receiver.is_some() {
            let result = self.vru_window_receiver.as_ref().unwrap().try_recv();
            if result.is_ok() {
                self.show_vru_dialog = true;
                self.vru_word_list = result.unwrap();
            }
        }

        if self.show_vru_dialog {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("vru_dialog"),
                egui::ViewportBuilder::default()
                    .with_title("What would you like to say?")
                    .with_always_on_top(),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::Grid::new("vru_words").show(ui, |ui| {
                            for (i, v) in self.vru_word_list.iter().enumerate() {
                                if i % 5 == 0 {
                                    ui.end_row();
                                }
                                if ui.button(format!("{}", *v)).clicked() {
                                    self.vru_word_notifier
                                        .as_ref()
                                        .unwrap()
                                        .send(v.clone())
                                        .unwrap();
                                    self.show_vru_dialog = false;
                                }
                            }
                        });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        self.vru_word_notifier
                            .as_ref()
                            .unwrap()
                            .send(String::from(""))
                            .unwrap();
                        self.show_vru_dialog = false;
                    }
                },
            );
        }
    }
}

fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}

fn add_japanese_font(ctx: &egui::Context) {
    ctx.add_font(epaint::text::FontInsert::new(
        "japanese_font",
        egui::FontData::from_static(include_bytes!("../../data/NotoSansJP-Regular.ttf")),
        vec![
            epaint::text::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
            epaint::text::InsertFontFamily {
                family: egui::FontFamily::Monospace,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
        ],
    ));
}
