use crate::device;
use crate::netplay;
use crate::ui;
use eframe::egui;
pub mod gui_netplay;

pub struct GopherEguiApp {
    config_dir: std::path::PathBuf,
    cache_dir: std::path::PathBuf,
    data_dir: std::path::PathBuf,
    configure_profile: bool,
    profile_name: String,
    controller_names: Vec<String>,
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
    netplay_error_receiver: Option<tokio::sync::mpsc::Receiver<String>>,
    vru_word_notifier: Option<tokio::sync::mpsc::Sender<String>>,
    vru_word_list: Vec<String>,
    pub netplay: gui_netplay::GuiNetplay,
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

pub fn get_controller_names(game_ui: &ui::Ui) -> Vec<String> {
    let mut controllers: Vec<String> = vec![];

    for offset in 0..game_ui.num_joysticks as isize {
        let name = unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::joystick::SDL_GetJoystickNameForID(
                *(game_ui.joysticks.offset(offset)),
            ))
        };
        controllers.push(name.to_string_lossy().to_string());
    }

    controllers
}

pub fn get_controller_paths(game_ui: &ui::Ui) -> Vec<String> {
    let mut controller_paths: Vec<String> = vec![];

    for offset in 0..game_ui.num_joysticks as isize {
        let path = unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::joystick::SDL_GetJoystickPathForID(
                *(game_ui.joysticks.offset(offset)),
            ))
            .to_string_lossy()
            .to_string()
        };
        controller_paths.push(path);
    }

    controller_paths
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
        let controller_paths = get_controller_paths(&game_ui);

        let mut selected_controller = [-1, -1, -1, -1];
        for (pos, item) in game_ui
            .config
            .input
            .controller_assignment
            .iter()
            .enumerate()
        {
            if item.is_some() {
                for (path_pos, path) in controller_paths.iter().enumerate() {
                    if item.as_deref().unwrap() == *path {
                        selected_controller[pos] = path_pos as i32;
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
            profile_name: "".to_string(),
            selected_profile: game_ui.config.input.input_profile_binding.clone(),
            selected_controller,
            controller_names: get_controller_names(&game_ui),
            input_profiles: get_input_profiles(&game_ui),
            controller_enabled: game_ui.config.input.controller_enabled,
            upscale: game_ui.config.video.upscale,
            integer_scaling: game_ui.config.video.integer_scaling,
            fullscreen: game_ui.config.video.fullscreen,
            emulate_vru: game_ui.config.input.emulate_vru,
            show_vru_dialog: false,
            dinput: false,
            netplay_error_receiver: None,
            vru_window_receiver: None,
            vru_word_notifier: None,
            vru_word_list: Vec::new(),
            netplay: Default::default(),
        }
    }
}

fn save_config(game_ui: &mut ui::Ui, save_config_items: SaveConfig) {
    let controller_paths = get_controller_paths(game_ui);
    for (pos, item) in save_config_items.selected_controller.iter().enumerate() {
        if *item != -1 {
            game_ui.config.input.controller_assignment[pos] =
                Some(controller_paths[*item as usize].clone());
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

fn configure_profile(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Configure Input Profile")
        // .open(&mut self.configure_profile)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let name_label = ui.label("Profile Name:");
                ui.text_edit_singleline(&mut app.profile_name)
                    .labelled_by(name_label.id);
            });
            ui.checkbox(&mut app.dinput, "Use DirectInput");
            ui.horizontal(|ui| {
                if ui.button("Configure Profile").clicked() {
                    let profile_name = app.profile_name.clone();
                    let config_dir = app.config_dir.clone();
                    let dinput = app.dinput;
                    tokio::spawn(async move {
                        let mut game_ui = ui::Ui::new(config_dir);
                        ui::input::configure_input_profile(&mut game_ui, profile_name, dinput);
                    });
                    app.configure_profile = false;
                    if !app.profile_name.is_empty()
                        && !app.input_profiles.contains(&app.profile_name)
                    {
                        app.input_profiles.push(app.profile_name.clone())
                    }
                };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        app.configure_profile = false
                    };
                })
            });
        });
}

fn show_vru_dialog(app: &mut GopherEguiApp, ctx: &egui::Context) {
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
                    for (i, v) in app.vru_word_list.iter().enumerate() {
                        if i % 5 == 0 {
                            ui.end_row();
                        }
                        if ui.button((*v).to_string()).clicked() {
                            app.vru_word_notifier
                                .as_ref()
                                .unwrap()
                                .try_send(v.clone())
                                .unwrap();
                            app.show_vru_dialog = false;
                        }
                    }
                });
            });

            if ctx.input(|i| i.viewport().close_requested()) {
                app.vru_word_notifier
                    .as_ref()
                    .unwrap()
                    .try_send(String::from(""))
                    .unwrap();
                app.show_vru_dialog = false;
            }
        },
    );
}

pub fn open_rom(app: &mut GopherEguiApp, ctx: &egui::Context) {
    let task;
    let netplay;

    let selected_controller = app.selected_controller;
    let selected_profile = app.selected_profile.clone();
    let controller_enabled = app.controller_enabled;
    let upscale = app.upscale;
    let integer_scaling = app.integer_scaling;
    let fullscreen = app.fullscreen;
    let emulate_vru = app.emulate_vru;
    let config_dir = app.config_dir.clone();
    let cache_dir = app.cache_dir.clone();
    let data_dir = app.data_dir.clone();
    let peer_addr;
    let session;
    let player_number;

    if app.netplay.player_name.is_empty() {
        task = Some(rfd::AsyncFileDialog::new().pick_file());
        netplay = false;
        peer_addr = None;
        session = None;
        player_number = None;
    } else {
        task = None;
        netplay = true;
        peer_addr = app.netplay.peer_addr;
        session = app.netplay.waiting_session.clone();
        player_number = Some(app.netplay.player_number);
    }

    let (netplay_error_notifier, netplay_error_receiver): (
        tokio::sync::mpsc::Sender<String>,
        tokio::sync::mpsc::Receiver<String>,
    ) = tokio::sync::mpsc::channel(8);

    let (vru_window_notifier, vru_window_receiver): (
        tokio::sync::mpsc::Sender<Vec<String>>,
        tokio::sync::mpsc::Receiver<Vec<String>>,
    ) = tokio::sync::mpsc::channel(1);

    let (vru_word_notifier, vru_word_receiver): (
        tokio::sync::mpsc::Sender<String>,
        tokio::sync::mpsc::Receiver<String>,
    ) = tokio::sync::mpsc::channel(1);

    if netplay {
        app.netplay_error_receiver = Some(netplay_error_receiver);
    }
    if emulate_vru && !netplay {
        app.vru_window_receiver = Some(vru_window_receiver);
        app.vru_word_notifier = Some(vru_word_notifier);
    } else {
        app.vru_window_receiver = None;
        app.vru_word_notifier = None;
    }

    let rom_contents = app.netplay.rom_contents.clone();
    let gui_ctx = ctx.clone();
    tokio::spawn(async move {
        let file = if !netplay { task.unwrap().await } else { None };

        if file.is_some() || netplay {
            let running_file = cache_dir.join("game_running");
            if running_file.exists() {
                println!("Game already running");
                return;
            }
            let result = std::fs::File::create(running_file.clone());
            if result.is_err() {
                panic!("could not create running file: {}", result.err().unwrap())
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

            if netplay {
                device.netplay = Some(netplay::init(
                    peer_addr.unwrap(),
                    session.unwrap(),
                    player_number.unwrap(),
                    netplay_error_notifier,
                    gui_ctx,
                ));
                device::run_game(rom_contents, data_dir, &mut device, fullscreen);
                netplay::close(&mut device);
            } else {
                if emulate_vru {
                    device.vru.window_notifier = Some(vru_window_notifier);
                    device.vru.word_receiver = Some(vru_word_receiver);
                    device.vru.gui_ctx = Some(gui_ctx);
                }

                let rom_contents = device::get_rom_contents(file.unwrap().path());
                if rom_contents.is_empty() {
                    println!("Could not read rom file");
                } else {
                    device::run_game(rom_contents, data_dir, &mut device, fullscreen);
                }
            }
            let result = std::fs::remove_file(running_file.clone());
            if result.is_err() {
                panic!("could not remove running file: {}", result.err().unwrap())
            }
        }
    });
}

impl eframe::App for GopherEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.netplay.create {
            gui_netplay::netplay_create(self, ctx);
        }

        if self.netplay.join {
            gui_netplay::netplay_join(self, ctx);
        }

        if self.netplay.wait {
            gui_netplay::netplay_wait(self, ctx);
        }

        if self.netplay_error_receiver.is_some() {
            let result = self.netplay_error_receiver.as_mut().unwrap().try_recv();
            if result.is_ok() {
                self.netplay.error = result.unwrap();
            }
        }

        if !self.netplay.error.is_empty() {
            gui_netplay::netplay_error(self, ctx, self.netplay.error.clone());
        }

        if self.configure_profile {
            configure_profile(self, ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.configure_profile {
                ui.disable()
            }
            egui::Grid::new("button_grid")
                .min_col_width(200.0)
                .show(ui, |ui| {
                    if ui.button("Open ROM").clicked() {
                        open_rom(self, ctx);
                    }

                    if ui.button("Netplay: Create Session").clicked()
                        && !self.cache_dir.join("game_running").exists()
                    {
                        self.netplay.create = true;
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
                        self.netplay.join = true;
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
                        self.controller_names[self.selected_controller[i] as usize].clone()
                    };
                    egui::ComboBox::from_id_salt(format!("controller-combo-{}", i))
                        .selected_text(controller_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_controller[i],
                                -1,
                                "None".to_string(),
                            );
                            for j in 0..self.controller_names.len() {
                                ui.selectable_value(
                                    &mut self.selected_controller[i],
                                    j as i32,
                                    self.controller_names[j].clone(),
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
            show_vru_dialog(self, ctx);
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
