use crate::{device, netplay, ui};
use eframe::egui;

pub mod gui_netplay;

pub struct GopherEguiApp {
    dirs: ui::Dirs,
    configure_profile: bool,
    profile_name: String,
    controller_names: Vec<String>,
    selected_controller: [i32; 4],
    selected_profile: [String; 4],
    input_profiles: Vec<String>,
    controller_enabled: [bool; 4],
    controller_paths: Vec<String>,
    upscale: u32,
    integer_scaling: bool,
    fullscreen: bool,
    emulate_vru: bool,
    dinput: bool,
    show_vru_dialog: bool,
    vru_window_receiver: Option<tokio::sync::mpsc::Receiver<Vec<String>>>,
    vru_word_notifier: Option<tokio::sync::mpsc::Sender<String>>,
    vru_word_list: Vec<String>,
    latest_version: Option<semver::Version>,
    update_receiver: Option<tokio::sync::mpsc::Receiver<GithubData>>,
    pub netplay: gui_netplay::GuiNetplay,
}

#[derive(serde::Deserialize)]
struct GithubData {
    tag_name: String,
}

struct SaveConfig {
    selected_controller: [i32; 4],
    selected_profile: [String; 4],
    controller_enabled: [bool; 4],
    upscale: u32,
    integer_scaling: bool,
    fullscreen: bool,
    emulate_vru: bool,
}

fn get_input_profiles(config: &ui::config::Config) -> Vec<String> {
    let mut profiles = vec![];
    for key in config.input.input_profiles.keys() {
        profiles.push(key.clone())
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
        controller_paths: Vec<String>,
        controller_names: Vec<String>,
    ) -> GopherEguiApp {
        add_fonts(&cc.egui_ctx);
        let config = ui::config::Config::new();

        let mut selected_controller = [-1, -1, -1, -1];
        for (pos, item) in config.input.controller_assignment.iter().enumerate() {
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
            configure_profile: false,
            profile_name: "".to_string(),
            selected_profile: config.input.input_profile_binding.clone(),
            selected_controller,
            controller_names,
            input_profiles: get_input_profiles(&config),
            controller_enabled: config.input.controller_enabled,
            upscale: config.video.upscale,
            integer_scaling: config.video.integer_scaling,
            fullscreen: config.video.fullscreen,
            emulate_vru: config.input.emulate_vru,
            show_vru_dialog: false,
            dinput: false,
            controller_paths,
            vru_window_receiver: None,
            vru_word_notifier: None,
            latest_version: None,
            update_receiver: None,
            vru_word_list: Vec::new(),
            netplay: Default::default(),
            dirs: ui::get_dirs(),
        }
    }
}

fn save_config(
    config: &mut ui::config::Config,
    controller_paths: Vec<String>,
    save_config_items: SaveConfig,
) {
    for (pos, item) in save_config_items.selected_controller.iter().enumerate() {
        if *item != -1 {
            config.input.controller_assignment[pos] =
                Some(controller_paths[*item as usize].clone());
        } else {
            config.input.controller_assignment[pos] = None
        }
    }

    config.input.input_profile_binding = save_config_items.selected_profile;
    config.input.controller_enabled = save_config_items.controller_enabled;

    config.video.upscale = save_config_items.upscale;
    config.video.integer_scaling = save_config_items.integer_scaling;
    config.video.fullscreen = save_config_items.fullscreen;
    config.input.emulate_vru = save_config_items.emulate_vru;
}

impl Drop for GopherEguiApp {
    fn drop(&mut self) {
        let save_config_items = SaveConfig {
            selected_controller: self.selected_controller,
            selected_profile: self.selected_profile.clone(),
            controller_enabled: self.controller_enabled,
            upscale: self.upscale,
            integer_scaling: self.integer_scaling,
            fullscreen: self.fullscreen,
            emulate_vru: self.emulate_vru,
        };
        let mut config = ui::config::Config::new();
        save_config(
            &mut config,
            self.controller_paths.clone(),
            save_config_items,
        );
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
                    let dinput = app.dinput;
                    std::thread::spawn(move || {
                        let mut game_ui = ui::Ui::new();
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

fn get_latest_version(app: &mut GopherEguiApp, ctx: &egui::Context) {
    if app.update_receiver.is_none() {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        app.update_receiver = Some(rx);
        let gui_ctx = ctx.clone();
        let client = reqwest::Client::builder()
            .user_agent(env!("CARGO_PKG_NAME"))
            .build()
            .unwrap();
        let task = client
            .get("https://api.github.com/repos/gopher64/gopher64/releases/latest")
            .send();
        tokio::spawn(async move {
            let response = task.await;
            if let Ok(response) = response {
                let data: Result<GithubData, reqwest::Error> = response.json().await;
                if data.is_ok() {
                    tx.send(data.unwrap()).await.unwrap();
                } else {
                    tx.send(GithubData {
                        tag_name: format!("v{}", env!("CARGO_PKG_VERSION")),
                    })
                    .await
                    .unwrap();
                }
                gui_ctx.request_repaint();
            }
        });
    } else if app.latest_version.is_none() {
        let result = app.update_receiver.as_mut().unwrap().try_recv();
        if result.is_ok() {
            let tag = &result.unwrap().tag_name[1..];
            app.latest_version = Some(semver::Version::parse(tag).unwrap());
        } else {
            ctx.request_repaint();
        }
    }
}

pub fn open_rom(app: &mut GopherEguiApp, ctx: &egui::Context) {
    let netplay;

    let selected_controller = app.selected_controller;
    let selected_profile = app.selected_profile.clone();
    let controller_enabled = app.controller_enabled;
    let upscale = app.upscale;
    let integer_scaling = app.integer_scaling;
    let fullscreen = app.fullscreen;
    let emulate_vru = app.emulate_vru;
    let peer_addr;
    let session;
    let player_number;
    let cache_dir = app.dirs.cache_dir.clone();
    let controller_paths = app.controller_paths.clone();

    if app.netplay.player_name.is_empty() {
        netplay = false;
        peer_addr = None;
        session = None;
        player_number = None;
    } else {
        netplay = true;
        peer_addr = app.netplay.peer_addr;
        session = app.netplay.waiting_session.clone();
        player_number = Some(app.netplay.player_number);
    }

    let (vru_window_notifier, vru_window_receiver): (
        tokio::sync::mpsc::Sender<Vec<String>>,
        tokio::sync::mpsc::Receiver<Vec<String>>,
    ) = tokio::sync::mpsc::channel(1);

    let (vru_word_notifier, vru_word_receiver): (
        tokio::sync::mpsc::Sender<String>,
        tokio::sync::mpsc::Receiver<String>,
    ) = tokio::sync::mpsc::channel(1);

    if emulate_vru && !netplay {
        app.vru_window_receiver = Some(vru_window_receiver);
        app.vru_word_notifier = Some(vru_word_notifier);
    } else {
        app.vru_window_receiver = None;
        app.vru_word_notifier = None;
    }

    let rom_contents = app.netplay.rom_contents.clone();
    let gui_ctx = ctx.clone();

    let mut task = None;
    if !netplay {
        task = Some(rfd::AsyncFileDialog::new().pick_file());
    }
    tokio::spawn(async move {
        let file = if !netplay { task.unwrap().await } else { None };

        std::thread::Builder::new()
            .name("n64".to_string())
            .stack_size(env!("N64_STACK_SIZE").parse().unwrap())
            .spawn(move || {
                let save_config_items = SaveConfig {
                    selected_controller,
                    selected_profile,
                    controller_enabled,
                    upscale,
                    integer_scaling,
                    fullscreen,
                    emulate_vru,
                };

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

                    let mut device = device::Device::new();
                    save_config(&mut device.ui.config, controller_paths, save_config_items);

                    if netplay {
                        device.netplay = Some(netplay::init(
                            peer_addr.unwrap(),
                            session.unwrap(),
                            player_number.unwrap(),
                        ));
                        device.ui.fullscreen = fullscreen;
                        device::run_game(rom_contents, &mut device);
                        netplay::close(&mut device);
                    } else {
                        if emulate_vru {
                            device.vru_window.window_notifier = Some(vru_window_notifier);
                            device.vru_window.word_receiver = Some(vru_word_receiver);
                            device.vru_window.gui_ctx = Some(gui_ctx);
                        }

                        let rom_contents = device::get_rom_contents(file.unwrap().path());
                        if rom_contents.is_empty() {
                            println!("Could not read rom file");
                        } else {
                            device.ui.fullscreen = fullscreen;
                            device::run_game(rom_contents, &mut device);
                        }
                    }
                    let result = std::fs::remove_file(running_file);
                    if result.is_err() {
                        panic!("could not remove running file: {}", result.err().unwrap())
                    }
                }
            })
            .unwrap();
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
                        && !self.dirs.cache_dir.join("game_running").exists()
                    {
                        self.netplay.create = true;
                    }

                    ui.end_row();

                    if ui.button("Configure Input Profile").clicked()
                        && !self.dirs.cache_dir.join("game_running").exists()
                    {
                        self.configure_profile = true;
                    }

                    if ui.button("Netplay: Join Session").clicked()
                        && !self.dirs.cache_dir.join("game_running").exists()
                    {
                        self.netplay.join = true;
                    }
                });

            ui.add_space(32.0);
            ui.label("Controller Config:");
            egui::Grid::new("controller_config").show(ui, |ui| {
                ui.label("Port");
                ui.label("Enabled");
                ui.label("Emulate VRU");
                //ui.label("Transfer Pak");
                ui.label("Profile");
                ui.label("Controller");
                ui.end_row();
                for i in 0..4 {
                    ui.label(format!("{}", i + 1));
                    ui.centered_and_justified(|ui| {
                        ui.checkbox(&mut self.controller_enabled[i], "");
                    });
                    let mut vru = false;
                    ui.centered_and_justified(|ui| {
                        if i < 3 {
                            ui.add_enabled(false, egui::Checkbox::new(&mut vru, ""));
                        } else {
                            ui.add_enabled(true, egui::Checkbox::new(&mut self.emulate_vru, ""));
                        }
                    });
                    /*
                    let mut tpak: bool = false;
                    ui.centered_and_justified(|ui| {
                        ui.checkbox(&mut tpak, "");
                    });
                    */
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
            let upscale_values = [1, 2, 4];
            let mut slider_value = match self.upscale {
                1 => 0,
                2 => 1,
                4 => 2,
                _ => 0,
            };
            let display_text = format!("{}x Resolution", upscale_values[slider_value]);
            if ui
                .add(
                    egui::Slider::new(&mut slider_value, 0..=2)
                        .show_value(false)
                        .text(display_text),
                )
                .changed()
            {
                self.upscale = upscale_values[slider_value];
            };
            ui.checkbox(&mut self.integer_scaling, "Integer Scaling");
            ui.checkbox(&mut self.fullscreen, "Fullscreen (Esc closes game)");

            ui.add_space(32.0);
            ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
            if self.latest_version.is_some() {
                let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
                if current_version < *self.latest_version.as_ref().unwrap() {
                    ui.hyperlink_to(
                        "New version available!",
                        "https://github.com/gopher64/gopher64/releases/latest",
                    );
                }
            }
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

        get_latest_version(self, ctx);
    }
}

fn add_fonts(ctx: &egui::Context) {
    ctx.add_font(eframe::epaint::text::FontInsert::new(
        "regular_font",
        egui::FontData::from_static(include_bytes!("../../data/Roboto-Regular.ttf")),
        vec![
            eframe::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Highest,
            },
            eframe::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Monospace,
                priority: egui::epaint::text::FontPriority::Highest,
            },
        ],
    ));
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
