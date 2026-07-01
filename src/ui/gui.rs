use crate::device;
use crate::retroachievements;
use crate::ui;
#[cfg(target_os = "android")]
use crate::ui::android;
use slint::Model;
#[cfg(not(target_os = "android"))]
use slint::winit_030::WinitWindowAccessor;

slint::include_modules!();

#[cfg(not(target_os = "android"))]
pub const N64_EXTENSIONS: [&str; 12] = [
    "n64", "v64", "z64", "7z", "zip", "bin", "N64", "V64", "Z64", "7Z", "ZIP", "BIN",
];

#[derive(serde::Deserialize)]
struct GithubData {
    tag_name: String,
}

pub struct NetplayDevice {
    pub server_addr: String,
    pub player_number: usize,
    pub number_of_players: usize,
    pub input_delay: usize,
}

fn check_latest_version(weak: slint::Weak<AppWindow>) {
    let task = ui::WEB_CLIENT
        .get("https://api.github.com/repos/gopher64/gopher64/releases/latest")
        .send();
    tokio::spawn(async move {
        let response = task.await;
        if let Ok(response) = response {
            let data: Result<GithubData, reqwest::Error> = response.json().await;

            let latest_version = match data {
                Ok(data) => match semver::Version::parse(&data.tag_name[1..]) {
                    Ok(github_version) => github_version,
                    Err(e) => {
                        eprintln!("Error parsing latest version from GitHub: {}", e);
                        semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
                    }
                },
                Err(e) => {
                    eprintln!("Error getting latest version from GitHub: {}", e);
                    semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
                }
            };
            let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
            if current_version < latest_version {
                weak.upgrade_in_event_loop(move |handle| handle.set_has_update(true))
                    .unwrap();
            }
        }
    });
}

pub fn open_uri(path: impl AsRef<std::ffi::OsStr>) {
    #[cfg(target_os = "android")]
    return ui::android::open_uri(path.as_ref().to_str().unwrap());

    #[cfg(not(target_os = "android"))]
    if let Err(e) = open::that_detached(path) {
        eprintln!("Error opening path: {}", e);
    }
}

fn run_with_path(weak: slint::Weak<AppWindow>, path: std::path::PathBuf) {
    let weak2 = weak.clone();
    weak.upgrade_in_event_loop(move |handle| {
        if handle.get_game_running() {
            return;
        }
        save_settings(&handle);

        run_rom(
            path,
            ui::GameSettings {
                overclock: handle.get_overclock_n64_cpu(),
                disable_expansion_pak: handle.get_disable_expansion_pak(),
                cheats: rustc_hash::FxHashMap::default(), // will be filled in later
                load_savestate_slot: None,
            },
            None,
            weak2,
        );
    })
    .unwrap();
}

#[cfg(not(target_os = "android"))]
fn file_dropped(app: &AppWindow) {
    let weak = app.as_weak();
    app.window()
        .on_winit_window_event(move |_winit_window, event| {
            if let slint::winit_030::winit::event::WindowEvent::DroppedFile(path) = event {
                run_with_path(weak.clone(), path.to_path_buf());
            }
            slint::winit_030::EventResult::Propagate
        });
}

fn rom_exists(path: &str) -> bool {
    #[cfg(not(target_os = "android"))]
    return std::fs::exists(path).unwrap_or(false);
    #[cfg(target_os = "android")]
    return android::rom_exists(path);
}

fn local_game_window(app: &AppWindow, config: &ui::config::Config) {
    app.set_recent_roms(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
                .recent_roms
                .iter()
                .filter(|x| rom_exists(x))
                .map(|x| {
                    (
                        x.into(),
                        std::path::Path::new(&decode_path(x))
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .into(),
                    )
                })
                .collect::<Vec<(slint::SharedString, slint::SharedString)>>(),
        ),
    )));

    let weak = app.as_weak();
    app.on_open_rom_button_clicked(move || {
        weak.upgrade_in_event_loop(move |handle| {
            save_settings(&handle);
            open_rom(&handle)
        })
        .unwrap();
    });

    let weak = app.as_weak();
    app.on_recent_rom_button_clicked(move |rom| {
        weak.upgrade_in_event_loop(move |handle| {
            run_with_path(handle.as_weak(), std::path::PathBuf::from(rom.to_string()));
        })
        .unwrap();
    });

    #[cfg(not(target_os = "android"))]
    {
        let saves_path = ui::get_dirs().data_dir.join("saves");
        app.on_saves_folder_button_clicked(move || {
            open_uri(&saves_path);
        });

        file_dropped(app);
    }
}

fn input_profiles(config: &ui::config::Config) -> Vec<String> {
    let mut profiles = vec![];
    for key in config.input.input_profiles.keys() {
        profiles.push(key.clone())
    }

    // make sure default profile is always first
    if let Some(pos) = profiles.iter().position(|x| x == "default") {
        let default_profile = profiles.remove(pos);
        profiles.insert(0, default_profile);
    }
    profiles
}

fn settings_window(app: &AppWindow, config: &ui::config::Config) {
    app.set_integer_scaling(config.video.integer_scaling);
    app.set_ssaa(config.video.ssaa);
    app.set_fullscreen(config.video.fullscreen);
    app.set_widescreen(config.video.widescreen);
    app.set_vsync(config.video.vsync);
    app.set_apply_crt_shader(config.video.crt);
    app.set_overclock_n64_cpu(config.emulation.overclock);
    app.set_disable_expansion_pak(config.emulation.disable_expansion_pak);
    app.set_emulate_usb(config.emulation.usb);
    app.set_rewind(config.emulation.rewind);
    let combobox_value = match config.video.upscale {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        _ => 0,
    };
    app.set_resolution(combobox_value);

    if let Some(rom_dir_str) = config.rom_dir.to_str() {
        app.set_rom_dir(rom_dir_str.into());
    }
}

pub fn update_input_profiles(weak: &slint::Weak<AppWindow>, config: &ui::config::Config) {
    let profiles = input_profiles(config);
    let config_bindings = config.input.input_profile_binding.clone();
    weak.upgrade_in_event_loop(move |handle| {
        let profile_bindings = slint::VecModel::default();
        for (i, input_profile_binding) in handle.get_selected_profile_binding().iter().enumerate() {
            let currently_selected = handle
                .get_input_profiles()
                .row_data(input_profile_binding as usize)
                .unwrap_or(config_bindings[i].clone().into())
                .to_string();
            let position = profiles
                .iter()
                .position(|profile| *profile == currently_selected);
            profile_bindings.push(position.unwrap_or(0) as i32);
        }

        handle.set_input_profiles(slint::ModelRc::from(std::rc::Rc::new(
            slint::VecModel::from(
                profiles
                    .iter()
                    .map(|x| x.into())
                    .collect::<Vec<slint::SharedString>>(),
            ),
        )));

        handle
            .set_selected_profile_binding(slint::ModelRc::from(std::rc::Rc::new(profile_bindings)));
    })
    .unwrap();
}

fn clear_gb_paths(weak: &slint::Weak<AppWindow>, player: i32) {
    weak.upgrade_in_event_loop(move |handle| {
        let rom_paths = handle.get_gb_rom_paths();
        let ram_paths = handle.get_gb_ram_paths();
        rom_paths.set_row_data(player as usize, String::new().into());
        ram_paths.set_row_data(player as usize, String::new().into());
        handle.set_gb_rom_paths(rom_paths);
        handle.set_gb_ram_paths(ram_paths);
    })
    .unwrap();
}

fn controller_window(app: &AppWindow, config: &ui::config::Config) {
    #[cfg(not(target_os = "android"))]
    ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);

    app.set_emulate_vru(config.input.emulate_vru);

    app.set_controller_enabled(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(config.input.controller_enabled.to_vec()),
    )));

    app.set_transferpak(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(config.input.transfer_pak.to_vec()),
    )));

    app.set_gb_rom_paths(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
                .input
                .gb_rom_path
                .iter()
                .map(|x| x.into())
                .collect::<Vec<slint::SharedString>>(),
        ),
    )));

    app.set_gb_ram_paths(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
                .input
                .gb_ram_path
                .iter()
                .map(|x| x.into())
                .collect::<Vec<slint::SharedString>>(),
        ),
    )));

    update_input_profiles(&app.as_weak(), config);

    app.set_controller_changed(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(vec![false, false, false, false]),
    )));

    let config_controller_assignment = config.input.controller_assignment.clone();
    let weak_app = app.as_weak();
    app.on_controller_window_created(move || {
        let controller_assignment = config_controller_assignment.clone();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                let mut current_selected_paths = vec![None; 4];
                for (i, selected_controller) in handle.get_selected_controller().iter().enumerate()
                {
                    current_selected_paths[i] = handle
                        .get_controller_paths()
                        .row_data(selected_controller as usize);
                }

                let controller_names = ui::input::get_controller_names();
                handle.set_controller_names(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(
                        controller_names
                            .iter()
                            .map(|x| x.into())
                            .collect::<Vec<slint::SharedString>>(),
                    ),
                )));

                let controller_paths = ui::input::get_controller_paths();
                handle.set_controller_paths(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(
                        controller_paths
                            .iter()
                            .map(|x| x.into())
                            .collect::<Vec<slint::SharedString>>(),
                    ),
                )));

                let selected_controllers = slint::VecModel::default();
                for i in 0..4 {
                    let assigned_path =
                        if let Some(current_selected_path) = &current_selected_paths[i] {
                            current_selected_path.to_string()
                        } else if let Some(config_assigned_path) = &controller_assignment[i] {
                            config_assigned_path.to_string()
                        } else {
                            String::new()
                        };
                    let selected_index = controller_paths
                        .iter()
                        .position(|controller_path| assigned_path == *controller_path)
                        .unwrap_or(0) as i32;
                    selected_controllers.push(selected_index);
                }
                handle.set_selected_controller(slint::ModelRc::from(std::rc::Rc::new(
                    selected_controllers,
                )));
            })
            .unwrap();
    });

    let weak_app = app.as_weak();
    app.on_input_profile_button_clicked(move || {
        weak_app
            .upgrade_in_event_loop(move |handle| {
                handle.set_input_deadzone(ui::input::DEADZONE_DEFAULT);
                handle.set_input_profile_name(String::new().into());
                handle.set_input_dinput(false);
                handle.set_show_input_profile(true);
            })
            .unwrap();
    });
    wizard::init(app);

    let weak_app2 = app.as_weak();
    app.on_transferpak_toggled(move |player, enabled| {
        if enabled {
            let select_gb_rom = select_gb_rom(player);

            let weak_app3 = weak_app2.clone();
            tokio::spawn(async move {
                if let Some(gb_rom) = select_gb_rom.await {
                    let weak_app4 = weak_app3.clone();
                    weak_app3
                        .upgrade_in_event_loop(move |_handle| {
                            let select_gb_ram = select_gb_ram(player);

                            tokio::spawn(async move {
                                if let Some(gb_ram) = select_gb_ram.await {
                                    weak_app4
                                        .upgrade_in_event_loop(move |handle| {
                                            let rom_paths = handle.get_gb_rom_paths();
                                            let ram_paths = handle.get_gb_ram_paths();
                                            rom_paths.set_row_data(
                                                player as usize,
                                                gb_rom.to_str().unwrap().into(),
                                            );
                                            ram_paths.set_row_data(
                                                player as usize,
                                                gb_ram.to_str().unwrap().into(),
                                            );
                                            handle.set_gb_rom_paths(rom_paths);
                                            handle.set_gb_ram_paths(ram_paths);
                                        })
                                        .unwrap();
                                } else {
                                    clear_gb_paths(&weak_app4, player);
                                }
                            });
                        })
                        .unwrap();
                } else {
                    clear_gb_paths(&weak_app3, player);
                }
            });
        }
    });
}

pub fn save_settings(app: &AppWindow) {
    let mut config = ui::config::Config::new();
    config.rom_dir = app.get_rom_dir().to_string().into();
    config.video.integer_scaling = app.get_integer_scaling();
    config.video.ssaa = app.get_ssaa();
    config.video.fullscreen = app.get_fullscreen();
    config.video.widescreen = app.get_widescreen();
    config.video.vsync = app.get_vsync();
    config.video.crt = app.get_apply_crt_shader();
    config.emulation.overclock = app.get_overclock_n64_cpu();
    config.emulation.disable_expansion_pak = app.get_disable_expansion_pak();
    config.emulation.usb = app.get_emulate_usb();
    config.emulation.rewind = app.get_rewind();
    let upscale_values = [1, 2, 4, 8];
    config.video.upscale = upscale_values[app.get_resolution() as usize];

    config.input.emulate_vru = app.get_emulate_vru();
    for (i, controller_enabled) in app.get_controller_enabled().iter().enumerate() {
        config.input.controller_enabled[i] = controller_enabled;
    }
    for (i, transferpak_enabled) in app.get_transferpak().iter().enumerate() {
        config.input.transfer_pak[i] = transferpak_enabled;
        config.input.gb_rom_path[i] = app.get_gb_rom_paths().row_data(i).unwrap().to_string();
        config.input.gb_ram_path[i] = app.get_gb_ram_paths().row_data(i).unwrap().to_string();
    }
    for (i, input_profile_binding) in app.get_selected_profile_binding().iter().enumerate() {
        config.input.input_profile_binding[i] = app
            .get_input_profiles()
            .row_data(input_profile_binding as usize)
            .unwrap()
            .to_string();
    }

    for (i, selected_controller) in app.get_selected_controller().iter().enumerate() {
        if app.get_controller_changed().row_data(i).unwrap_or(false) {
            let controller_path = app
                .get_controller_paths()
                .row_data(selected_controller as usize)
                .unwrap()
                .to_string();
            if controller_path.is_empty() {
                config.input.controller_assignment[i] = None;
            } else {
                config.input.controller_assignment[i] = Some(controller_path);
            }
        }
    }
}

fn about_window(app: &AppWindow) {
    app.on_wiki_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64/wiki");
    });
    app.on_discord_button_clicked(move || {
        open_uri("https://discord.gg/9RGXq8W8JQ");
    });
    app.on_patreon_button_clicked(move || {
        open_uri("https://patreon.com/loganmc10");
    });
    app.on_github_sponsors_button_clicked(move || {
        open_uri("https://github.com/sponsors/loganmc10");
    });
    app.on_source_code_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64");
    });
    app.on_newversion_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64/releases/latest");
    });
    app.set_version(format!("Version: {}", env!("GIT_DESCRIBE")).into());

    //flatpak, itch.io, and android have their own update checking mechanism
    if std::env::var("FLATPAK_ID").is_err()
        && std::env::var("ITCHIO_APP").is_err()
        && cfg!(not(target_os = "android"))
    {
        check_latest_version(app.as_weak());
    }
}

pub fn app_window(
    app: &AppWindow,
    is_android: bool,
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    let no_intro_map_clone = no_intro_map.clone();
    tokio::spawn(async move {
        load_no_intro(no_intro_map_clone).await;
    });

    retroachievements::init_client(false, false, false);
    app.set_is_android(is_android);
    about_window(app);
    ui::retroachievements::ra_window(app);
    {
        let config = ui::config::Config::new();
        settings_window(app, &config);
        controller_window(app, &config);
        local_game_window(app, &config);
    }
    ui::netplay::netplay_window(app, no_intro_map.clone());
    ui::cheats::cheats_window(app, no_intro_map);

    #[cfg(not(target_os = "android"))]
    {
        let weak_app = app.as_weak();
        app.window().on_close_requested(move || {
            weak_app
                .upgrade_in_event_loop(move |handle| {
                    save_settings(&handle);
                    handle.invoke_netplay_close();
                })
                .unwrap();
            slint::CloseRequestResponse::HideWindow
        });
    }

    app.run().unwrap();
    retroachievements::shutdown_client();
}

pub fn run_rom(
    file_path: std::path::PathBuf,
    game_settings: ui::GameSettings,
    netplay: Option<NetplayDevice>,
    weak: slint::Weak<AppWindow>,
) {
    #[cfg(target_os = "android")]
    ui::android::run_rom(file_path, game_settings, netplay, weak);

    #[cfg(not(target_os = "android"))]
    tokio::spawn(async move {
        weak.upgrade_in_event_loop(move |handle| handle.set_game_running(true))
            .unwrap();

        let cli_path = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(format!("{}-cli", env!("CARGO_PKG_NAME")));
        let cmd_path = if cfg!(target_os = "macos") && cli_path.exists() {
            cli_path
        } else {
            std::env::current_exe().unwrap()
        };
        let mut command = tokio::process::Command::new(cmd_path);
        command.args([
            "--overclock",
            &game_settings.overclock.to_string(),
            "--disable-expansion-pak",
            &game_settings.disable_expansion_pak.to_string(),
        ]);
        let cheats_path = ui::get_dirs().cache_dir.join("cheats.json");
        if let Some(netplay_device) = netplay {
            let f = std::fs::File::create(&cheats_path).unwrap();
            serde_json::to_writer_pretty(f, &game_settings.cheats).unwrap();

            command.args([
                "--netplay-server-addr",
                &netplay_device.server_addr,
                "--netplay-player-number",
                &netplay_device.player_number.to_string(),
                "--netplay-number-of-players",
                &netplay_device.number_of_players.to_string(),
                "--netplay-input-delay",
                &netplay_device.input_delay.to_string(),
                "--cheats",
                cheats_path.to_str().unwrap(),
            ]);
        }

        let success = command
            .arg(file_path.to_str().unwrap())
            .status()
            .await
            .unwrap()
            .success();

        if !success {
            eprintln!("Failed to run game");
        }

        let _ = std::fs::remove_file(cheats_path);

        weak.upgrade_in_event_loop(move |handle| {
            if let Some(rom_dir) = file_path.parent().unwrap().to_str() {
                handle.set_rom_dir(rom_dir.into());
            }
            if success {
                update_recent_roms(&handle, file_path);
            }
            handle.set_game_running(false);
        })
        .unwrap();
    });
}

fn decode_path(path: &str) -> String {
    #[cfg(target_os = "android")]
    return ui::android::decode_path(path);
    #[cfg(not(target_os = "android"))]
    return path.to_string();
}

pub fn update_recent_roms(app: &AppWindow, file_path: std::path::PathBuf) {
    let recent_roms = slint::VecModel::default();
    recent_roms.push((
        file_path.to_str().unwrap().into(),
        std::path::Path::new(&decode_path(file_path.to_str().unwrap()))
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .into(),
    ));

    for rom in app.get_recent_roms().iter() {
        if rom.0 != file_path.to_str().unwrap() && recent_roms.row_count() < 5 && rom_exists(&rom.0)
        {
            recent_roms.push(rom);
        }
    }
    app.set_recent_roms(slint::ModelRc::from(std::rc::Rc::new(recent_roms)));
}

pub async fn get_nointro_name(
    rom: &[u8],
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) -> String {
    let hash = device::cart::rom::calculate_hash(rom).to_lowercase();
    if let Some(name) = no_intro_map.lock().await.get(&hash) {
        name.clone()
    } else {
        ui::storage::get_game_name(rom)
    }
}

async fn load_no_intro(
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    let mut reader = quick_xml::Reader::from_str(include_str!(
        "../../data/ui/Nintendo - Nintendo 64 (DB Export) (20260609-194259).xml"
    ));
    let mut buf = Vec::new();
    let mut current_game = String::new();
    let mut map = no_intro_map.lock().await;
    loop {
        match reader.read_event_into_async(&mut buf).await {
            Ok(quick_xml::events::Event::Start(e)) => {
                if e.name().as_ref() == b"game"
                    && let Ok(Some(name_attribute)) = e.try_get_attribute("name")
                    && let Ok(name) = String::from_utf8(name_attribute.value.into_owned())
                {
                    current_game = name;
                }
            }
            Ok(quick_xml::events::Event::Empty(e)) => {
                if e.name().as_ref() == b"file"
                    && let Ok(Some(format_attribute)) = e.try_get_attribute("format")
                    && format_attribute.value.as_ref() == b"BigEndian"
                    && let Ok(Some(sha256_attribute)) = e.try_get_attribute("sha256")
                    && let Ok(sha256) = String::from_utf8(sha256_attribute.value.into_owned())
                {
                    map.insert(sha256.to_lowercase(), current_game.clone());
                }
            }
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(quick_xml::events::Event::Eof) => break,
            _ => (),
        }
        buf.clear();
    }
}

pub async fn select_rom(rom_dir: slint::SharedString) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    return ui::android::select_rom(rom_dir).await;

    #[cfg(not(target_os = "android"))]
    {
        if !rom_dir.is_empty() && std::fs::exists(&rom_dir).unwrap_or(false) {
            rfd::AsyncFileDialog::new().set_directory(rom_dir)
        } else {
            rfd::AsyncFileDialog::new()
        }
        .set_title("Select ROM")
        .add_filter("ROM files", &N64_EXTENSIONS)
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
    }
}

pub async fn select_gb_rom(player: i32) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    return ui::android::select_gb_rom(player).await;

    #[cfg(not(target_os = "android"))]
    {
        rfd::AsyncFileDialog::new()
            .set_title(format!("GB ROM P{}", player + 1))
            .add_filter("GB ROM files", &["gb", "gbc", "GB", "GBC"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }
}

pub async fn select_gb_ram(player: i32) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    return ui::android::select_gb_ram(player).await;

    #[cfg(not(target_os = "android"))]
    {
        rfd::AsyncFileDialog::new()
            .set_title(format!("GB RAM P{}", player + 1))
            .add_filter("GB RAM files", &["sav", "ram", "srm", "SAV", "RAM", "SRM"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }
}

fn open_rom(app: &AppWindow) {
    let select_rom = select_rom(app.get_rom_dir());

    let overclock = app.get_overclock_n64_cpu();
    let disable_expansion_pak = app.get_disable_expansion_pak();

    let weak = app.as_weak();
    tokio::spawn(async move {
        if let Some(file) = select_rom.await {
            run_rom(
                file,
                ui::GameSettings {
                    overclock,
                    disable_expansion_pak,
                    cheats: rustc_hash::FxHashMap::default(), // will be filled in later
                    load_savestate_slot: None,
                },
                None,
                weak,
            );
        }
    });
}

/// In-app input-profile wizard: drives the pure menu state machine in
/// `ui::input` from the Slint event loop. Raw controller input arrives
/// per-platform: on desktop a 16 ms `slint::Timer` pumps SDL (no SDL window —
/// the GAMEPAD subsystem plus the background-events hint deliver device
/// events windowless); on Android `SlintActivity` forwards events over JNI
/// into [`apply_android_input`] (SDL is NOT initialized in that process).
/// Keyboard arrives through the wizard's FocusScope, and taps through its
/// TouchAreas. Replaces the old standalone SDL window
/// (`configure_input_profile`) and the old Android config `N64Activity`.
pub(crate) mod wizard {
    use super::{AppWindow, ProfileRow, ProfileWizardData, update_input_profiles};
    use crate::ui;
    use crate::ui::input;
    use crate::ui::input_capture::{self, CaptureEvent};
    use slint::ComponentHandle;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Ignore raw/key input for this long after entering a capture step, so
    /// the input that triggered capture (or a still-held key) does not
    /// immediately re-bind — the pump-friendly version of the old `debounce`.
    const DEBOUNCE_MS: u64 = 150;

    struct Session {
        state: input::MenuState,
        bindings: input::Bindings,
        profile_name: String,
        dinput: bool,
        deadzone: i32,
        /// Skip axis events until the stick returns near neutral (set after
        /// binding an axis so a still-deflected stick does not instantly
        /// re-bind).
        await_axis_neutral: bool,
        /// Left-Y position for edge-triggered list navigation.
        last_axis_y: i16,
        /// Instant until which capture input is ignored (debounce window).
        ignore_until: std::time::Instant,
        #[cfg(not(target_os = "android"))]
        open_joysticks: Vec<*mut sdl3_sys::joystick::SDL_Joystick>,
        #[cfg(not(target_os = "android"))]
        open_controllers: Vec<*mut sdl3_sys::gamepad::SDL_Gamepad>,
    }

    impl Session {
        /// Open the ~150 ms ignore window; on desktop also flush queued SDL
        /// events. (No SDL calls on Android — SDL is not initialized there.)
        fn debounce(&mut self) {
            #[cfg(not(target_os = "android"))]
            unsafe {
                sdl3_sys::events::SDL_PumpEvents();
                sdl3_sys::events::SDL_FlushEvents(
                    u32::from(sdl3_sys::events::SDL_EVENT_FIRST),
                    u32::from(sdl3_sys::events::SDL_EVENT_LAST),
                );
            }
            self.ignore_until =
                std::time::Instant::now() + std::time::Duration::from_millis(DEBOUNCE_MS);
        }
    }

    type Shared = Rc<RefCell<Option<Session>>>;

    /// Wire the wizard's callbacks once at startup. The session and pump
    /// timer live behind `Rc`s shared by every callback; everything runs on
    /// the Slint event-loop thread.
    pub(super) fn init(app: &AppWindow) {
        let session: Shared = Rc::new(RefCell::new(None));
        let timer = Rc::new(slint::Timer::default());

        // Let the JNI feed reach the session from the event-loop thread.
        #[cfg(target_os = "android")]
        ANDROID_WIZARD.with(|w| {
            *w.borrow_mut() = Some((app.as_weak(), session.clone(), timer.clone()));
        });

        {
            let session = session.clone();
            let timer = timer.clone();
            let weak = app.as_weak();
            app.on_input_profile_creation_button_clicked(move || {
                let Some(handle) = weak.upgrade() else { return };
                open(&handle, &session, &timer);
            });
        }

        let wiz = app.global::<ProfileWizardData>();

        // Keyboard from the wizard's FocusScope: bind in capture (Esc skips),
        // navigate in review.
        {
            let session = session.clone();
            let timer = timer.clone();
            let weak = app.as_weak();
            wiz.on_key_pressed(move |text| {
                let Some(handle) = weak.upgrade() else { return };
                let action = {
                    let mut guard = session.borrow_mut();
                    let Some(s) = guard.as_mut() else { return };
                    let Some(scancode) = input_capture::slint_key_to_scancode(&text) else {
                        return;
                    };
                    match s.state.screen {
                        input::Screen::Capture => {
                            if scancode == i32::from(sdl3_sys::scancode::SDL_SCANCODE_ESCAPE) {
                                input::Action::Cancel
                            } else if std::time::Instant::now() < s.ignore_until {
                                return;
                            } else {
                                let value = input::KEY_LABELS[s.state.selected].1;
                                s.bindings.bind(value, CaptureEvent::Key(scancode));
                                input::Action::Bound
                            }
                        }
                        input::Screen::List => {
                            if scancode == i32::from(sdl3_sys::scancode::SDL_SCANCODE_UP) {
                                input::Action::Up
                            } else if scancode == i32::from(sdl3_sys::scancode::SDL_SCANCODE_DOWN) {
                                input::Action::Down
                            } else if scancode == i32::from(sdl3_sys::scancode::SDL_SCANCODE_RETURN)
                            {
                                input::Action::Confirm
                            } else if scancode == i32::from(sdl3_sys::scancode::SDL_SCANCODE_ESCAPE)
                            {
                                input::Action::Cancel
                            } else {
                                return;
                            }
                        }
                    }
                };
                dispatch(&handle, &session, &timer, action);
            });
        }

        // Explicit navigation intents from the UI layer (unused today; kept
        // as the wizard's generic entry point for e.g. hardware-back wiring).
        {
            let session = session.clone();
            let timer = timer.clone();
            let weak = app.as_weak();
            wiz.on_nav(move |code| {
                let Some(handle) = weak.upgrade() else { return };
                let action = match code {
                    0 => input::Action::Up,
                    1 => input::Action::Down,
                    2 => input::Action::Confirm,
                    3 => input::Action::Cancel,
                    _ => input::Action::Quit,
                };
                dispatch(&handle, &session, &timer, action);
            });
        }

        // Tap a row to select it; tap the selected row again to rebind.
        {
            let session = session.clone();
            let timer = timer.clone();
            let weak = app.as_weak();
            wiz.on_tap_row(move |i| {
                let Some(handle) = weak.upgrade() else { return };
                let confirm = {
                    let mut guard = session.borrow_mut();
                    let Some(s) = guard.as_mut() else { return };
                    if s.state.screen != input::Screen::List {
                        return;
                    }
                    let i = i as usize;
                    if s.state.selected == i {
                        true
                    } else {
                        s.state.selected = i;
                        false
                    }
                };
                if confirm {
                    dispatch(&handle, &session, &timer, input::Action::Confirm);
                } else {
                    render(&handle, &session);
                }
            });
        }

        {
            let session = session.clone();
            let timer = timer.clone();
            let weak = app.as_weak();
            wiz.on_tap_save(move || {
                let Some(handle) = weak.upgrade() else { return };
                {
                    let mut guard = session.borrow_mut();
                    let Some(s) = guard.as_mut() else { return };
                    if s.state.screen != input::Screen::List {
                        return;
                    }
                    s.state.selected = input::SAVE_ROW;
                }
                dispatch(&handle, &session, &timer, input::Action::Confirm);
            });
        }

        {
            let session = session.clone();
            let timer = timer.clone();
            let weak = app.as_weak();
            wiz.on_tap_cancel(move || {
                let Some(handle) = weak.upgrade() else { return };
                dispatch(&handle, &session, &timer, input::Action::Cancel);
            });
        }
    }

    /// Start the wizard for the dialog's profile name / dinput / deadzone.
    fn open(handle: &AppWindow, session: &Shared, timer: &Rc<slint::Timer>) {
        let profile_name = handle.get_input_profile_name().to_string();
        let dinput = handle.get_input_dinput();
        let deadzone = handle.get_input_deadzone();
        handle.set_show_input_profile(false);
        // The dialog's button is disabled for these; keep the old CLI guard.
        if profile_name.is_empty() || profile_name == "default" {
            return;
        }

        // Mirror how the old SDL window opened devices: gamepads normally,
        // raw joysticks for DirectInput. Desktop only — on Android SDL is not
        // initialized in this process; input arrives from Kotlin over JNI.
        #[cfg(not(target_os = "android"))]
        let (open_joysticks, open_controllers) = {
            let mut open_joysticks = Vec::new();
            let mut open_controllers = Vec::new();
            for joystick in input::get_joysticks() {
                if dinput {
                    let j = unsafe { sdl3_sys::joystick::SDL_OpenJoystick(joystick) };
                    if !j.is_null() {
                        open_joysticks.push(j);
                    }
                } else {
                    let c = unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(joystick) };
                    if !c.is_null() {
                        open_controllers.push(c);
                    }
                }
            }
            (open_joysticks, open_controllers)
        };

        let config = ui::config::Config::new();
        let existing = config.input.input_profiles.get(&profile_name);
        // Pre-load the existing profile so editing only re-captures the
        // inputs the user picks; a new profile starts the guided flow.
        let bindings = existing.map_or_else(input::Bindings::empty, input::Bindings::from_profile);
        let state = input::MenuState::entry(existing.is_some());

        let mut s = Session {
            state,
            bindings,
            profile_name: profile_name.clone(),
            dinput,
            deadzone,
            await_axis_neutral: false,
            last_axis_y: 0,
            ignore_until: std::time::Instant::now(),
            #[cfg(not(target_os = "android"))]
            open_joysticks,
            #[cfg(not(target_os = "android"))]
            open_controllers,
        };
        // Drain anything still held from the click that opened the wizard so
        // it cannot auto-bind the first guided input.
        s.debounce();
        *session.borrow_mut() = Some(s);

        handle
            .global::<ProfileWizardData>()
            .set_profile_name(profile_name.into());
        render(handle, session);
        handle.set_show_profile_wizard(true);

        #[cfg(not(target_os = "android"))]
        {
            let weak = handle.as_weak();
            let session = session.clone();
            let timer_weak = Rc::downgrade(timer);
            timer.start(
                slint::TimerMode::Repeated,
                std::time::Duration::from_millis(16),
                move || {
                    let (Some(handle), Some(timer)) = (weak.upgrade(), timer_weak.upgrade()) else {
                        return;
                    };
                    pump(&handle, &session, &timer);
                },
            );
        }
        // Android: no SDL pump; tell Kotlin to start forwarding controller
        // input (dispatch overrides + a focused capture overlay).
        #[cfg(target_os = "android")]
        {
            let _ = timer;
            ui::android::set_capture_active(true);
        }
    }

    /// One 16 ms tick: drain queued SDL events into bindings / nav actions.
    #[cfg(not(target_os = "android"))]
    fn pump(handle: &AppWindow, session: &Shared, timer: &Rc<slint::Timer>) {
        let mut event: sdl3_sys::events::SDL_Event = Default::default();
        while unsafe { sdl3_sys::events::SDL_PollEvent(&mut event) } {
            let action = {
                let mut guard = session.borrow_mut();
                let Some(s) = guard.as_mut() else { return };
                let decoded = input_capture::decode_sdl(&event, s.dinput);
                translate(s, &decoded)
            };
            if let Some(action) = action {
                dispatch(handle, session, timer, action);
            }
        }
    }

    /// Per-event-loop-thread hook for the Android JNI feed: set by `init`,
    /// read by [`apply_android_input`].
    #[cfg(target_os = "android")]
    type AndroidHook = (slint::Weak<AppWindow>, Shared, Rc<slint::Timer>);
    #[cfg(target_os = "android")]
    thread_local! {
        static ANDROID_WIZARD: RefCell<Option<AndroidHook>> = const { RefCell::new(None) };
    }

    /// Feed one JNI-forwarded input through the SAME decode → policy →
    /// dispatch path the desktop pump uses. Must run on the Slint event-loop
    /// thread (`slint::invoke_from_event_loop`); a no-op while no session is
    /// open (Kotlin only forwards while capture is active, but late events
    /// can still race `finish`).
    #[cfg(target_os = "android")]
    pub(crate) fn apply_android_input(ev: input_capture::AndroidEvent) {
        let Some((weak, session, timer)) = ANDROID_WIZARD.with(|w| w.borrow().clone()) else {
            return;
        };
        let Some(handle) = weak.upgrade() else { return };
        let action = {
            let mut guard = session.borrow_mut();
            let Some(s) = guard.as_mut() else { return };
            let decoded = input_capture::decode_android(&ev, s.dinput);
            translate(s, &decoded)
        };
        if let Some(action) = action {
            dispatch(&handle, &session, &timer, action);
        }
    }

    /// Apply one platform-decoded event against the current screen: the
    /// debounce / axis-neutral gates around binding, and edge-triggered
    /// left-Y list navigation. One deliberate divergence from the old
    /// SDL-window loop: East/back skips a capture even during the debounce
    /// window (the old flush simply discarded it).
    fn translate(s: &mut Session, d: &input_capture::Decoded) -> Option<input::Action> {
        match s.state.screen {
            input::Screen::Capture => {
                // East / back skips, bypassing the debounce window.
                if d.nav == Some(input::Action::Cancel) {
                    return Some(input::Action::Cancel);
                }
                if std::time::Instant::now() < s.ignore_until {
                    return None;
                }
                match d.bind {
                    Some(ev) => {
                        if d.axis_stream && s.await_axis_neutral {
                            return None; // stick still deflected from the last bind
                        }
                        if d.axis_stream {
                            s.await_axis_neutral = true;
                        }
                        let value = input::KEY_LABELS[s.state.selected].1;
                        s.bindings.bind(value, ev);
                        Some(input::Action::Bound)
                    }
                    None => {
                        // A below-threshold deflection on the bindable axis
                        // stream re-arms axis capture (the old neutral gate).
                        if d.axis_stream {
                            s.await_axis_neutral = false;
                        }
                        None
                    }
                }
            }
            input::Screen::List => {
                if let Some(v) = d.list_y {
                    // Left-Y navigates on threshold crossings only.
                    let thresh = (i16::MAX as i32 * 3 / 4) as i16;
                    let was_neutral = s.last_axis_y.saturating_abs() <= thresh;
                    s.last_axis_y = v;
                    if was_neutral && v < -thresh {
                        return Some(input::Action::Up);
                    } else if was_neutral && v > thresh {
                        return Some(input::Action::Down);
                    }
                    return None;
                }
                d.nav
            }
        }
    }

    /// Run one decoded action through `advance` and act on the transition.
    fn dispatch(
        handle: &AppWindow,
        session: &Shared,
        timer: &Rc<slint::Timer>,
        action: input::Action,
    ) {
        let exit = {
            let mut guard = session.borrow_mut();
            let Some(s) = guard.as_mut() else { return };
            let was_capture = s.state.screen == input::Screen::Capture;
            let t = input::advance(&mut s.state, action);
            if t.begin_capture {
                if was_capture {
                    // Guided auto-advance: the old loop only debounced when
                    // the axis-neutral latch was clear.
                    if !s.await_axis_neutral {
                        s.debounce();
                    }
                } else {
                    // Entering capture from the list re-arms axes and debounces.
                    s.await_axis_neutral = false;
                    s.debounce();
                }
            }
            t.exit.then_some(t.save)
        };
        match exit {
            Some(save) => finish(handle, session, timer, save),
            None => render(handle, session),
        }
    }

    /// Mirror the session into `ProfileWizardData`.
    fn render(handle: &AppWindow, session: &Shared) {
        let guard = session.borrow();
        let Some(s) = guard.as_ref() else { return };
        let wiz = handle.global::<ProfileWizardData>();

        match s.state.screen {
            input::Screen::Capture => {
                wiz.set_mode(0);
                wiz.set_capture_label(input::KEY_LABELS[s.state.selected].0.into());
                let next = if s.state.guided {
                    input::KEY_LABELS
                        .get(s.state.selected + 1)
                        .map_or("", |&(label, _)| label)
                } else {
                    ""
                };
                wiz.set_next_label(next.into());
                wiz.set_progress_index(s.state.selected as i32 + 1);
                wiz.set_progress_total(input::PROFILE_SIZE as i32);
            }
            input::Screen::List => {
                wiz.set_mode(1);
                let rows: Vec<ProfileRow> = input::KEY_LABELS
                    .iter()
                    .map(|&(label, value)| ProfileRow {
                        label: label.into(),
                        binding: s.bindings.label(value).into(),
                        bound: s.bindings.is_bound(value),
                    })
                    .collect();
                wiz.set_rows(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(rows),
                )));
                wiz.set_selected(s.state.selected as i32);
                wiz.set_warning(
                    if s.state.quit_armed {
                        "unsaved changes - quit again to discard"
                    } else {
                        ""
                    }
                    .into(),
                );
            }
        }

        match input::KEY_LABELS
            .get(s.state.selected)
            .and_then(|&(_, value)| input::glow_center(value))
        {
            Some((cx, cy, _)) => {
                wiz.set_glow_cx(cx);
                wiz.set_glow_cy(cy);
                wiz.set_glow_visible(true);
            }
            None => wiz.set_glow_visible(false),
        }
    }

    /// Close the wizard: release capture (SDL devices on desktop, the Kotlin
    /// forwarder on Android), optionally persist the profile (via `Config`'s
    /// write-on-drop, as the old CLI did), stop the pump and return to the
    /// controller page.
    fn finish(handle: &AppWindow, session: &Shared, timer: &Rc<slint::Timer>, save: bool) {
        timer.stop();
        let Some(s) = session.borrow_mut().take() else {
            return;
        };
        #[cfg(not(target_os = "android"))]
        {
            for joystick in &s.open_joysticks {
                unsafe { sdl3_sys::joystick::SDL_CloseJoystick(*joystick) };
            }
            for controller in &s.open_controllers {
                unsafe { sdl3_sys::gamepad::SDL_CloseGamepad(*controller) };
            }
        }
        #[cfg(target_os = "android")]
        ui::android::set_capture_active(false);
        if save {
            let mut config = ui::config::Config::new();
            config
                .input
                .input_profiles
                .insert(s.profile_name, s.bindings.to_profile(s.dinput, s.deadzone));
            update_input_profiles(&handle.as_weak(), &config);
            // `config` drops here and writes config.json to disk.
        }
        handle.set_show_profile_wizard(false);
    }
}
