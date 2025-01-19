use crate::device;
use crate::ui;
use eframe::egui;

pub struct Netplay {
    session_name: String,
    password: String,
    player_name: String,
    server: (String, String),
    servers: std::collections::HashMap<String, String>,
    server_receiver: Option<tokio::sync::mpsc::Receiver<std::collections::HashMap<String, String>>>,
}

pub struct GopherEguiApp {
    config_dir: std::path::PathBuf,
    cache_dir: std::path::PathBuf,
    data_dir: std::path::PathBuf,
    configure_profile: bool,
    netplay_create: bool,
    netplay_join: bool,
    netplay_wait: bool,
    profile_name: String,
    controllers: Vec<String>,
    selected_controller: [i32; 4],
    selected_profile: [String; 4],
    input_profiles: Vec<String>,
    controller_enabled: [bool; 4],
    upscale: bool,
    integer_scaling: bool,
    fullscreen: bool,
    emulate_vru: bool,
    dinput: bool,
    show_vru_dialog: bool,
    vru_window_receiver: Option<tokio::sync::mpsc::Receiver<Vec<String>>>,
    vru_word_notifier: Option<tokio::sync::mpsc::Sender<String>>,
    vru_word_list: Vec<String>,
    netplay: Netplay,
}

struct SaveConfig {
    selected_controller: [i32; 4],
    selected_profile: [String; 4],
    controller_enabled: [bool; 4],
    upscale: bool,
    integer_scaling: bool,
    fullscreen: bool,
    emulate_vru: bool,
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
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config_dir: std::path::PathBuf,
        cache_dir: std::path::PathBuf,
        data_dir: std::path::PathBuf,
    ) -> GopherEguiApp {
        add_japanese_font(&cc.egui_ctx);
        let game_ui = ui::Ui::new(config_dir.clone());
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
            cache_dir: cache_dir.clone(),
            config_dir: config_dir.clone(),
            data_dir: data_dir.clone(),
            configure_profile: false,
            netplay_create: false,
            netplay_join: false,
            netplay_wait: false,
            profile_name: "".to_string(),
            selected_profile: game_ui.config.input.input_profile_binding.clone(),
            selected_controller,
            controllers: get_controllers(&game_ui),
            input_profiles: get_input_profiles(&game_ui),
            controller_enabled: game_ui.config.input.controller_enabled,
            upscale: game_ui.config.video.upscale,
            integer_scaling: game_ui.config.video.integer_scaling,
            fullscreen: game_ui.config.video.fullscreen,
            emulate_vru: game_ui.config.input.emulate_vru,
            show_vru_dialog: false,
            dinput: false,
            vru_window_receiver: None,
            vru_word_notifier: None,
            vru_word_list: Vec::new(),
            netplay: Netplay {
                server_receiver: None,
                session_name: "".to_string(),
                password: "".to_string(),
                player_name: "".to_string(),
                server: ("".to_string(), "".to_string()),
                servers: std::collections::HashMap::new(),
            },
        }
    }
}

fn save_config(game_ui: &mut ui::Ui, save_config_items: SaveConfig) {
    let joystick_subsystem = game_ui.joystick_subsystem.as_ref().unwrap();
    for (pos, item) in save_config_items.selected_controller.iter().enumerate() {
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

    game_ui.config.input.input_profile_binding = save_config_items.selected_profile;
    game_ui.config.input.controller_enabled = save_config_items.controller_enabled;

    game_ui.config.video.upscale = save_config_items.upscale;
    game_ui.config.video.integer_scaling = save_config_items.integer_scaling;
    game_ui.config.video.fullscreen = save_config_items.fullscreen;
    game_ui.config.input.emulate_vru = save_config_items.emulate_vru;
}

impl Drop for GopherEguiApp {
    fn drop(&mut self) {
        let mut game_ui = ui::Ui::new(self.config_dir.clone());
        let save_config_items = SaveConfig {
            selected_controller: self.selected_controller,
            selected_profile: self.selected_profile.clone(),
            controller_enabled: self.controller_enabled,
            upscale: self.upscale,
            integer_scaling: self.integer_scaling,
            fullscreen: self.fullscreen,
            emulate_vru: self.emulate_vru,
        };
        save_config(&mut game_ui, save_config_items);
    }
}

impl eframe::App for GopherEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.netplay_create {
            egui::Window::new("Create Netplay Session").show(ctx, |ui| {
                egui::Grid::new("button_grid").show(ui, |ui| {
                    let profile_name_label = ui.label("Profile Name:");
                    let mut size = ui.spacing().interact_size;
                    size.x = 200.0;
                    ui.add_sized(size, |ui: &mut egui::Ui| {
                        ui.text_edit_singleline(&mut self.netplay.session_name)
                            .labelled_by(profile_name_label.id)
                    });

                    ui.end_row();

                    let password_label = ui.label("Password (Optional):");

                    ui.text_edit_singleline(&mut self.netplay.password)
                        .labelled_by(password_label.id);

                    ui.end_row();

                    ui.label("ROM");
                    if ui.button("Open ROM").clicked() {
                        // Spawn dialog on main thread
                        let task = rfd::AsyncFileDialog::new().pick_file();
                        tokio::spawn(async {
                            let file = task.await;

                            if let Some(file) = file {
                                let _rom_contents = device::get_rom_contents(file.path());
                            }
                        });
                    }

                    ui.end_row();

                    let player_name_label = ui.label("Player Name:");

                    ui.text_edit_singleline(&mut self.netplay.player_name)
                        .labelled_by(player_name_label.id);

                    ui.end_row();

                    ui.label("Server:");

                    if self.netplay.servers.is_empty() {
                        if self.netplay.server_receiver.is_none() {
                            let (tx, rx) = tokio::sync::mpsc::channel(1);
                            self.netplay.server_receiver = Some(rx);

                            tokio::spawn(async move {
                                if let Ok(response) =
                                    reqwest::get("https://m64p.s3.amazonaws.com/servers.json").await
                                {
                                    if let Ok(servers) = response
                                        .json::<std::collections::HashMap<String, String>>()
                                        .await
                                    {
                                        let _ = tx.send(servers).await;
                                    }
                                }
                            });
                        } else {
                            let result = self.netplay.server_receiver.as_mut().unwrap().try_recv();
                            if result.is_ok() {
                                self.netplay.servers = result.unwrap();
                                self.netplay.server_receiver = None;
                                let first_server = self.netplay.servers.iter().next().unwrap();
                                self.netplay.server =
                                    (first_server.0.clone(), first_server.1.clone());
                            }
                        }
                    }

                    egui::ComboBox::from_id_salt("server-combobox")
                        .selected_text(self.netplay.server.0.to_string())
                        .show_ui(ui, |ui| {
                            for server in self.netplay.servers.iter() {
                                ui.selectable_value(
                                    &mut self.netplay.server,
                                    (server.0.clone(), server.1.clone()),
                                    server.0,
                                );
                            }
                        });

                    ui.end_row();

                    ui.label("Your ping:");

                    ui.label("value");

                    ui.end_row();

                    if ui.button("Create Session").clicked() {
                        self.netplay_create = false;
                        self.netplay_wait = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.netplay_create = false
                        };
                    })
                });
            });
        }

        if self.netplay_join {
            egui::Window::new("Join Netplay Session").show(ctx, |ui| {
                if ui.button("Close").clicked() {
                    self.netplay_join = false
                };
            });
        }

        if self.configure_profile {
            egui::Window::new("Configure Input Profile")
                // .open(&mut self.configure_profile)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let name_label = ui.label("Profile Name:");
                        ui.text_edit_singleline(&mut self.profile_name)
                            .labelled_by(name_label.id);
                    });
                    ui.checkbox(&mut self.dinput, "Use DirectInput");
                    ui.horizontal(|ui| {
                        if ui.button("Configure Profile").clicked() {
                            let profile_name = self.profile_name.clone();
                            let config_dir = self.config_dir.clone();
                            let dinput = self.dinput;
                            tokio::spawn(async move {
                                let mut game_ui = ui::Ui::new(config_dir);
                                ui::input::configure_input_profile(
                                    &mut game_ui,
                                    profile_name,
                                    dinput,
                                );
                            });
                            self.configure_profile = false;
                            if !self.profile_name.is_empty()
                                && !self.input_profiles.contains(&self.profile_name)
                            {
                                self.input_profiles.push(self.profile_name.clone())
                            }
                        };
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.configure_profile = false
                            };
                        })
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.configure_profile {
                ui.disable()
            }
            egui::Grid::new("button_grid")
                .min_col_width(200.0)
                .show(ui, |ui| {
                    if ui.button("Open ROM").clicked() {
                        // Spawn dialog on main thread
                        let task = rfd::AsyncFileDialog::new().pick_file();
                        let selected_controller = self.selected_controller;
                        let selected_profile = self.selected_profile.clone();
                        let controller_enabled = self.controller_enabled;
                        let upscale = self.upscale;
                        let integer_scaling = self.integer_scaling;
                        let fullscreen = self.fullscreen;
                        let emulate_vru = self.emulate_vru;
                        let config_dir = self.config_dir.clone();
                        let cache_dir = self.cache_dir.clone();
                        let data_dir = self.data_dir.clone();

                        let (vru_window_notifier, vru_window_receiver): (
                            tokio::sync::mpsc::Sender<Vec<String>>,
                            tokio::sync::mpsc::Receiver<Vec<String>>,
                        ) = tokio::sync::mpsc::channel(1);

                        let (vru_word_notifier, vru_word_receiver): (
                            tokio::sync::mpsc::Sender<String>,
                            tokio::sync::mpsc::Receiver<String>,
                        ) = tokio::sync::mpsc::channel(1);

                        if emulate_vru {
                            self.vru_window_receiver = Some(vru_window_receiver);
                            self.vru_word_notifier = Some(vru_word_notifier);
                        }

                        let gui_ctx = ctx.clone();
                        tokio::spawn(async move {
                            let file = task.await;

                            if let Some(file) = file {
                                let running_file = cache_dir.join("game_running");
                                if running_file.exists() {
                                    return;
                                }
                                let result = std::fs::File::create(running_file.clone());
                                if result.is_err() {
                                    panic!(
                                        "could not create running file: {}",
                                        result.err().unwrap()
                                    )
                                }
                                let mut device = device::Device::new(config_dir);

                                let save_config_items = SaveConfig {
                                    selected_controller,
                                    selected_profile,
                                    controller_enabled,
                                    upscale,
                                    integer_scaling,
                                    fullscreen,
                                    emulate_vru,
                                };
                                save_config(&mut device.ui, save_config_items);

                                if emulate_vru {
                                    device.vru.window_notifier = Some(vru_window_notifier);
                                    device.vru.word_receiver = Some(vru_word_receiver);
                                    device.vru.gui_ctx = Some(gui_ctx);
                                }
                                device::run_game(
                                    std::path::Path::new(file.path()),
                                    data_dir,
                                    &mut device,
                                    fullscreen,
                                );
                                let result = std::fs::remove_file(running_file.clone());
                                if result.is_err() {
                                    panic!(
                                        "could not remove running file: {}",
                                        result.err().unwrap()
                                    )
                                }
                            }
                        });
                    }

                    if ui.button("Netplay: Create Session").clicked()
                        && !self.cache_dir.join("game_running").exists()
                    {
                        self.netplay_create = true;
                    }

                    ui.end_row();

                    if ui.button("Configure Input Profile").clicked()
                        && !self.cache_dir.join("game_running").exists()
                    {
                        self.configure_profile = true;
                    }

                    if ui.button("Netplay: Join Session").clicked()
                        && !self.cache_dir.join("game_running").exists()
                    {
                        self.netplay_join = true;
                    }
                });

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
            ui.checkbox(&mut self.integer_scaling, "Integer Scaling");
            ui.checkbox(&mut self.fullscreen, "Fullscreen (Esc closes game)");
            ui.add_space(32.0);
            ui.checkbox(
                &mut self.emulate_vru,
                "Emulate VRU (connects VRU to controller port 4)",
            );
            ui.add_space(32.0);
            ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
        });

        if self.emulate_vru && self.vru_window_receiver.is_some() {
            let result = self.vru_window_receiver.as_mut().unwrap().try_recv();
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
                                if ui.button((*v).to_string()).clicked() {
                                    self.vru_word_notifier
                                        .as_ref()
                                        .unwrap()
                                        .try_send(v.clone())
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
                            .try_send(String::from(""))
                            .unwrap();
                        self.show_vru_dialog = false;
                    }
                },
            );
        }
    }
}

fn add_japanese_font(ctx: &egui::Context) {
    ctx.add_font(eframe::epaint::text::FontInsert::new(
        "japanese_font",
        egui::FontData::from_static(include_bytes!("../../data/NotoSansJP-Regular.ttf")),
        vec![
            eframe::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
            eframe::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Monospace,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
        ],
    ));
}
