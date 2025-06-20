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
    transfer_pak: [bool; 4],
    controller_paths: Vec<String>,
    upscale: u32,
    integer_scaling: bool,
    fullscreen: bool,
    widescreen: bool,
    crt: bool,
    overclock: bool,
    emulate_vru: bool,
    dinput: bool,
    show_vru_dialog: bool,
    vru_window_receiver: Option<tokio::sync::mpsc::Receiver<Vec<String>>>,
    vru_word_notifier: Option<tokio::sync::mpsc::Sender<String>>,
    vru_word_list: Vec<String>,
    latest_version: Option<semver::Version>,
    update_receiver: Option<tokio::sync::mpsc::Receiver<GithubData>>,
    netplay: gui_netplay::GuiNetplay,
}

#[derive(serde::Deserialize)]
struct GithubData {
    tag_name: String,
}

struct SaveConfig {
    selected_controller: [i32; 4],
    selected_profile: [String; 4],
    controller_enabled: [bool; 4],
    transfer_pak: [bool; 4],
    upscale: u32,
    integer_scaling: bool,
    fullscreen: bool,
    widescreen: bool,
    crt: bool,
    emulate_vru: bool,
    overclock: bool,
}

fn get_input_profiles(config: &ui::config::Config) -> Vec<String> {
    let mut profiles = vec![];
    for key in config.input.input_profiles.keys() {
        profiles.push(key.clone())
    }
    profiles
}

pub fn get_controller_paths(game_ui: &ui::Ui) -> Vec<String> {
    let mut controller_paths: Vec<String> = vec![];

    for joystick in game_ui.input.joysticks.iter() {
        let path = unsafe {
            std::ffi::CStr::from_ptr(sdl3_sys::joystick::SDL_GetJoystickPathForID(*joystick))
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
            transfer_pak: config.input.transfer_pak,
            upscale: config.video.upscale,
            integer_scaling: config.video.integer_scaling,
            fullscreen: config.video.fullscreen,
            widescreen: config.video.widescreen,
            crt: config.video.crt,
            emulate_vru: config.input.emulate_vru,
            overclock: config.emulation.overclock,
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
    config.input.transfer_pak = save_config_items.transfer_pak;

    config.video.upscale = save_config_items.upscale;
    config.video.integer_scaling = save_config_items.integer_scaling;
    config.video.fullscreen = save_config_items.fullscreen;
    config.video.widescreen = save_config_items.widescreen;
    config.video.crt = save_config_items.crt;
    config.input.emulate_vru = save_config_items.emulate_vru;

    config.emulation.overclock = save_config_items.overclock;
}

impl Drop for GopherEguiApp {
    fn drop(&mut self) {
        let save_config_items = SaveConfig {
            selected_controller: self.selected_controller,
            selected_profile: self.selected_profile.clone(),
            controller_enabled: self.controller_enabled,
            transfer_pak: self.transfer_pak,
            upscale: self.upscale,
            integer_scaling: self.integer_scaling,
            fullscreen: self.fullscreen,
            widescreen: self.widescreen,
            crt: self.crt,
            emulate_vru: self.emulate_vru,
            overclock: self.overclock,
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
    egui::Window::new("Configure Input Profile").show(ctx, |ui| {
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
                if !app.profile_name.is_empty() && !app.input_profiles.contains(&app.profile_name) {
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
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("What would you like to say?");
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

        ui.add_space(16.0);

        if ui.button("Close without saying anything").clicked() {
            app.vru_word_notifier
                .as_ref()
                .unwrap()
                .try_send(String::from(""))
                .unwrap();
            app.show_vru_dialog = false;
        };
    });
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

pub fn open_rom(app: &mut GopherEguiApp, ctx: &egui::Context, enable_overclock: bool) {
    let netplay;

    let selected_controller = app.selected_controller;
    let selected_profile = app.selected_profile.clone();
    let controller_enabled = app.controller_enabled;
    let transfer_pak = app.transfer_pak;
    let upscale = app.upscale;
    let integer_scaling = app.integer_scaling;
    let fullscreen = app.fullscreen;
    let widescreen = app.widescreen;
    let crt = app.crt;
    let emulate_vru = app.emulate_vru;
    let overclock = app.overclock;
    let mut peer_addr;
    let player_number;
    let cache_dir = app.dirs.cache_dir.clone();
    let controller_paths = app.controller_paths.clone();

    if app.netplay.player_name.is_empty() {
        netplay = false;
        peer_addr = None;
        player_number = None;
    } else {
        netplay = true;
        peer_addr = app.netplay.peer_addr;
        peer_addr
            .as_mut()
            .unwrap()
            .set_port(app.netplay.waiting_session.as_ref().unwrap().port.unwrap() as u16);
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

    let mut select_rom = None;
    let mut select_gb_rom = [None, None, None, None];
    let mut select_gb_ram = [None, None, None, None];

    if !netplay {
        select_rom = Some(
            rfd::AsyncFileDialog::new()
                .set_title("Select ROM")
                .pick_file(),
        );
        for i in 0..4 {
            if transfer_pak[i] {
                select_gb_rom[i] = Some(
                    rfd::AsyncFileDialog::new()
                        .set_title(format!("GB ROM P{}", i + 1))
                        .pick_file(),
                );
                select_gb_ram[i] = Some(
                    rfd::AsyncFileDialog::new()
                        .set_title(format!("GB RAM P{}", i + 1))
                        .pick_file(),
                );
            }
        }
    }
    tokio::spawn(async move {
        let file = if !netplay {
            select_rom.unwrap().await
        } else {
            None
        };
        let mut gb_rom_path = [None, None, None, None];
        let mut gb_ram_path = [None, None, None, None];
        if !netplay {
            for i in 0..4 {
                if transfer_pak[i] {
                    gb_rom_path[i] = select_gb_rom[i].as_mut().unwrap().await;
                    gb_ram_path[i] = select_gb_ram[i].as_mut().unwrap().await;
                }
            }
        }

        std::thread::Builder::new()
            .name("n64".to_string())
            .stack_size(env!("N64_STACK_SIZE").parse().unwrap())
            .spawn(move || {
                let save_config_items = SaveConfig {
                    selected_controller,
                    selected_profile,
                    controller_enabled,
                    transfer_pak,
                    upscale,
                    integer_scaling,
                    fullscreen,
                    widescreen,
                    crt,
                    emulate_vru,
                    overclock,
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
                        device.netplay =
                            Some(netplay::init(peer_addr.unwrap(), player_number.unwrap()));
                        device::run_game(&mut device, rom_contents, fullscreen, enable_overclock);
                        netplay::close(&mut device);
                    } else {
                        for i in 0..4 {
                            if gb_rom_path[i].is_some() && gb_ram_path[i].is_some() {
                                device.transferpaks[i].cart.rom =
                                    std::fs::read(gb_rom_path[i].as_ref().unwrap().path()).unwrap();

                                device.transferpaks[i].cart.ram =
                                    std::fs::read(gb_ram_path[i].as_ref().unwrap().path()).unwrap();
                            }
                        }

                        if emulate_vru {
                            device.vru_window.window_notifier = Some(vru_window_notifier);
                            device.vru_window.word_receiver = Some(vru_word_receiver);
                            device.vru_window.gui_ctx = Some(gui_ctx);
                        }

                        if let Some(rom_contents) = device::get_rom_contents(file.unwrap().path()) {
                            device::run_game(
                                &mut device,
                                rom_contents,
                                fullscreen,
                                enable_overclock,
                            );
                        } else {
                            println!("Could not read rom file");
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
        if self.show_vru_dialog {
            show_vru_dialog(self, ctx);
            return;
        }

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
                        open_rom(self, ctx, self.overclock);
                    }
                    if ui.button("Netplay: Create Session").clicked()
                        && !self.dirs.cache_dir.join("game_running").exists()
                    {
                        self.netplay.create = true;
                    }

                    if ui.button("Open Saves Folder").clicked() {
                        let command = if cfg!(target_os = "windows") {
                            "explorer"
                        } else if cfg!(target_os = "linux") {
                            "xdg-open"
                        } else {
                            panic!("Unsupported platform");
                        };
                        let _ = std::process::Command::new(command)
                            .arg(self.dirs.data_dir.join("saves"))
                            .spawn();
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

            ui.add_space(16.0);
            ui.label("Controller Config:");
            egui::Grid::new("controller_config").show(ui, |ui| {
                ui.label("Port");
                ui.label("Enabled");
                ui.label("Emulate VRU");
                ui.label("Transfer Pak");
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

                    ui.centered_and_justified(|ui| {
                        ui.checkbox(&mut self.transfer_pak[i], "");
                    });

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
            ui.add_space(16.0);
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
            ui.checkbox(&mut self.widescreen, "Widescreen (stretch)");
            ui.checkbox(&mut self.crt, "Apply CRT shader");

            ui.add_space(16.0);
            ui.checkbox(&mut self.overclock, "Overclock N64 CPU (may cause bugs)");
            ui.add_space(16.0);

            ui.hyperlink_to("Wiki", "https://github.com/gopher64/gopher64/wiki");
            ui.hyperlink_to("Discord Server", "https://discord.gg/9RGXq8W8JQ");
            ui.add_space(16.0);

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
