use crate::device;
use crate::netplay;
use crate::ui;
use slint::Model;

slint::include_modules!();

#[derive(serde::Deserialize)]
struct GithubData {
    tag_name: String,
}

pub struct NetplayDevice {
    pub peer_addr: std::net::SocketAddr,
    pub player_number: u8,
}

pub struct GbPaths {
    pub rom: [Option<std::path::PathBuf>; 4],
    pub ram: [Option<std::path::PathBuf>; 4],
}

pub struct VruChannel {
    pub vru_window_notifier: Option<tokio::sync::mpsc::Sender<Option<Vec<String>>>>,
    pub vru_word_receiver: Option<tokio::sync::mpsc::Receiver<String>>,
}

fn check_latest_version(weak: slint::Weak<AppWindow>) {
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

            let latest_version = if let Ok(data) = data {
                semver::Version::parse(&data.tag_name[1..]).unwrap()
            } else {
                semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
            };
            let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
            if current_version < latest_version {
                weak.upgrade_in_event_loop(move |handle| handle.set_has_update(true))
                    .unwrap();
            }
        }
    });
}

fn netplay_window(app: &AppWindow, controller_paths: &[Option<String>]) {
    let weak_create = app.as_weak();
    let weak_app = app.as_weak();
    let controller_paths_create = controller_paths.to_owned();
    app.on_create_session_button_clicked(move || {
        let controller_paths = controller_paths_create.clone();
        let weak_app = weak_app.clone();
        weak_create
            .upgrade_in_event_loop(move |handle| {
                let create_window = NetplayCreate::new().unwrap();
                save_settings(&handle, &controller_paths);
                ui::netplay::setup_create_window(
                    &create_window,
                    handle.get_overclock_n64_cpu(),
                    handle.get_fullscreen(),
                    weak_app,
                );
            })
            .unwrap();
    });

    let weak_join = app.as_weak();
    let weak_app = app.as_weak();
    let controller_paths_join = controller_paths.to_owned();
    app.on_join_session_button_clicked(move || {
        let controller_paths = controller_paths_join.clone();
        let weak_app = weak_app.clone();
        weak_join
            .upgrade_in_event_loop(move |handle| {
                let join_window = NetplayJoin::new().unwrap();
                save_settings(&handle, &controller_paths);
                ui::netplay::setup_join_window(&join_window, handle.get_fullscreen(), weak_app);
            })
            .unwrap();
    });

    app.on_netplay_discord_button_clicked(move || {
        open::that_detached("https://discord.gg/JyW6ZgBUyS").unwrap();
    });
}

fn local_game_window(app: &AppWindow, controller_paths: &[Option<String>]) {
    let dirs = ui::get_dirs();
    let weak = app.as_weak();
    let controller_paths = controller_paths.to_owned();
    app.on_open_rom_button_clicked(move || {
        let controller_paths = controller_paths.clone();
        weak.upgrade_in_event_loop(move |handle| {
            save_settings(&handle, &controller_paths);
            open_rom(&handle)
        })
        .unwrap();
    });

    let saves_path = dirs.data_dir.join("saves");
    app.on_saves_folder_button_clicked(move || {
        open::that_detached(saves_path.clone()).unwrap();
    });
}

fn input_profiles(config: &ui::config::Config) -> Vec<String> {
    let mut profiles = vec![];
    for key in config.input.input_profiles.keys() {
        profiles.push(key.clone())
    }
    profiles
}

fn settings_window(app: &AppWindow, config: &ui::config::Config) {
    app.set_integer_scaling(config.video.integer_scaling);
    app.set_fullscreen(config.video.fullscreen);
    app.set_widescreen(config.video.widescreen);
    app.set_apply_crt_shader(config.video.crt);
    app.set_overclock_n64_cpu(config.emulation.overclock);
    let combobox_value = match config.video.upscale {
        1 => 0,
        2 => 1,
        4 => 2,
        _ => 0,
    };
    app.set_resolution(combobox_value);
}

fn update_input_profiles(weak: &slint::Weak<AppWindow>, config: &ui::config::Config) {
    let profiles = input_profiles(config);
    let config_bindings = config.input.input_profile_binding.clone();
    let weak2 = weak.clone();
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
            profile_bindings.push(position.unwrap() as i32);
        }

        let input_profiles = slint::VecModel::default();
        for profile in profiles {
            input_profiles.push(profile.into());
        }
        let input_profiles_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
            std::rc::Rc::new(input_profiles);
        handle.set_input_profiles(slint::ModelRc::from(input_profiles_model));

        let input_profile_binding_model: std::rc::Rc<slint::VecModel<i32>> =
            std::rc::Rc::new(profile_bindings);
        handle.set_selected_profile_binding(slint::ModelRc::from(input_profile_binding_model));

        // this is a workaround to make the input profile combobox update
        handle.set_blank_profiles(true);
        slint::Timer::single_shot(std::time::Duration::from_millis(200), move || {
            weak2
                .upgrade_in_event_loop(move |handle| {
                    handle.set_blank_profiles(false);
                })
                .unwrap();
        });
    })
    .unwrap();
}

fn controller_window(
    app: &AppWindow,
    config: &ui::config::Config,
    controller_names: &Vec<String>,
    controller_paths: &[Option<String>],
) {
    let controller_enabled_model: std::rc::Rc<slint::VecModel<bool>> = std::rc::Rc::new(
        slint::VecModel::from(config.input.controller_enabled.to_vec()),
    );
    app.set_emulate_vru(config.input.emulate_vru);

    app.set_controller_enabled(slint::ModelRc::from(controller_enabled_model));

    let transferpak_enabled_model: std::rc::Rc<slint::VecModel<bool>> =
        std::rc::Rc::new(slint::VecModel::from(config.input.transfer_pak.to_vec()));
    app.set_transferpak(slint::ModelRc::from(transferpak_enabled_model));

    update_input_profiles(&app.as_weak(), config);

    let controllers = slint::VecModel::default();
    for controller in controller_names {
        controllers.push(controller.into());
    }
    let controller_names_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
        std::rc::Rc::new(controllers);
    app.set_controller_names(slint::ModelRc::from(controller_names_model));

    let selected_controllers = slint::VecModel::default();
    for selected in config.input.controller_assignment.iter() {
        let mut found = false;
        for (i, path) in controller_paths.iter().enumerate() {
            if selected == path {
                selected_controllers.push(i as i32);
                found = true;
                continue;
            }
        }
        if !found {
            selected_controllers.push(0);
        }
    }
    let selected_controllers_model: std::rc::Rc<slint::VecModel<i32>> =
        std::rc::Rc::new(selected_controllers);
    app.set_selected_controller(slint::ModelRc::from(selected_controllers_model));

    let weak_app = app.as_weak();
    app.on_input_profile_button_clicked(move || {
        let dialog = InputProfileDialog::new().unwrap();
        let weak_dialog = dialog.as_weak();
        let weak_app = weak_app.clone();
        dialog.on_profile_creation_button_clicked(move || {
            let weak_app = weak_app.clone();
            weak_dialog
                .upgrade_in_event_loop(move |handle| {
                    handle.hide().unwrap();
                    let profile_name = handle.get_profile_name().into();
                    let dinput = handle.get_dinput();

                    tokio::spawn(async move {
                        let mut game_ui = ui::Ui::new();
                        ui::input::configure_input_profile(&mut game_ui, profile_name, dinput);
                        update_input_profiles(&weak_app, &game_ui.config);
                    });
                })
                .unwrap();
        });
        dialog.show().unwrap();
    });
}

fn save_settings(app: &AppWindow, controller_paths: &[Option<String>]) {
    let mut config = ui::config::Config::new();
    config.video.integer_scaling = app.get_integer_scaling();
    config.video.fullscreen = app.get_fullscreen();
    config.video.widescreen = app.get_widescreen();
    config.video.crt = app.get_apply_crt_shader();
    config.emulation.overclock = app.get_overclock_n64_cpu();
    let upscale_values = [1, 2, 4];
    config.video.upscale = upscale_values[app.get_resolution() as usize];

    config.input.emulate_vru = app.get_emulate_vru();
    for (i, controller_enabled) in app.get_controller_enabled().iter().enumerate() {
        config.input.controller_enabled[i] = controller_enabled;
    }
    for (i, transferpak_enabled) in app.get_transferpak().iter().enumerate() {
        config.input.transfer_pak[i] = transferpak_enabled;
    }
    for (i, input_profile_binding) in app.get_selected_profile_binding().iter().enumerate() {
        config.input.input_profile_binding[i] = app
            .get_input_profiles()
            .row_data(input_profile_binding as usize)
            .unwrap()
            .to_string();
    }

    for (i, selected_controller) in app.get_selected_controller().iter().enumerate() {
        config.input.controller_assignment[i] =
            controller_paths[selected_controller as usize].clone();
    }
}

fn about_window(app: &AppWindow) {
    app.on_wiki_button_clicked(move || {
        open::that_detached("https://github.com/gopher64/gopher64/wiki").unwrap();
    });
    app.on_discord_button_clicked(move || {
        open::that_detached("https://discord.gg/9RGXq8W8JQ").unwrap();
    });
    app.on_patreon_button_clicked(move || {
        open::that_detached("https://patreon.com/loganmc10").unwrap();
    });
    app.on_github_sponsors_button_clicked(move || {
        open::that_detached("https://github.com/sponsors/loganmc10").unwrap();
    });
    app.on_source_code_button_clicked(move || {
        open::that_detached("https://github.com/gopher64/gopher64").unwrap();
    });
    app.on_newversion_button_clicked(move || {
        open::that_detached("https://github.com/gopher64/gopher64/releases/latest").unwrap();
    });
    app.set_version(format!("Version: {}", env!("CARGO_PKG_VERSION")).into());
    check_latest_version(app.as_weak());
}

pub fn app_window() {
    let app = AppWindow::new().unwrap();
    about_window(&app);
    let mut controller_paths;
    {
        let game_ui = ui::Ui::new();
        let mut controller_names = ui::input::get_controller_names(&game_ui);
        controller_names.insert(0, "None".into());
        controller_paths = ui::input::get_controller_paths(&game_ui);
        controller_paths.insert(0, None);
        settings_window(&app, &game_ui.config);
        controller_window(&app, &game_ui.config, &controller_names, &controller_paths);
    }
    local_game_window(&app, &controller_paths);
    netplay_window(&app, &controller_paths);
    app.run().unwrap();
    save_settings(&app, &controller_paths);
}

fn setup_vru_word_watcher(
    weak_vru: slint::Weak<AppWindow>,
    vru_word_notifier: tokio::sync::mpsc::Sender<String>,
    mut vru_window_receiver: tokio::sync::mpsc::Receiver<Option<Vec<String>>>,
) {
    tokio::spawn(async move {
        loop {
            let notifier = vru_word_notifier.clone();
            let notifier_closed = vru_word_notifier.clone();
            let result = vru_window_receiver.recv().await;
            if let Some(Some(words)) = result {
                weak_vru
                    .upgrade_in_event_loop(move |_handle| {
                        let vru_dialog = VruDialog::new().unwrap();
                        let vru_dialog_weak = vru_dialog.as_weak();

                        vru_dialog.on_vru_button_clicked(move |chosen_word| {
                            notifier.try_send(chosen_word.to_string()).unwrap();
                            vru_dialog_weak.unwrap().window().hide().unwrap();
                        });

                        vru_dialog.window().on_close_requested(move || {
                            notifier_closed.try_send("".to_string()).unwrap();
                            slint::CloseRequestResponse::HideWindow
                        });

                        let words_vec = slint::VecModel::default();
                        for word in words {
                            words_vec.push(word.into());
                        }
                        let words_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
                            std::rc::Rc::new(words_vec);
                        vru_dialog.set_words(slint::ModelRc::from(words_model));

                        vru_dialog.show().unwrap();
                    })
                    .unwrap();
            } else {
                return;
            }
        }
    });
}

pub fn run_rom(
    gb_paths: GbPaths,
    file_path: std::path::PathBuf,
    fullscreen: bool,
    overclock: bool,
    vru_channel: VruChannel,
    netplay: Option<NetplayDevice>,
    weak: slint::Weak<AppWindow>,
) {
    std::thread::Builder::new()
        .name("n64".to_string())
        .stack_size(env!("N64_STACK_SIZE").parse().unwrap())
        .spawn(move || {
            weak.upgrade_in_event_loop(move |handle| handle.set_game_running(true))
                .unwrap();

            let mut device = device::Device::new();

            for i in 0..4 {
                if gb_paths.rom[i].is_some() && gb_paths.ram[i].is_some() {
                    device.transferpaks[i].cart.rom =
                        std::fs::read(gb_paths.rom[i].as_ref().unwrap()).unwrap();

                    device.transferpaks[i].cart.ram =
                        std::fs::read(gb_paths.ram[i].as_ref().unwrap()).unwrap();
                }
            }

            device.vru_window.window_notifier = vru_channel.vru_window_notifier;
            device.vru_window.word_receiver = vru_channel.vru_word_receiver;

            if let Some(rom_contents) = device::get_rom_contents(file_path.as_path()) {
                if let Some(netplay_device) = netplay {
                    device.netplay = Some(netplay::init(
                        netplay_device.peer_addr,
                        netplay_device.player_number,
                    ));
                }
                device::run_game(&mut device, rom_contents, fullscreen, overclock);
                if device.netplay.is_some() {
                    netplay::close(&mut device);
                }
            } else {
                println!("Could not read rom file");
            }

            if let Some(vru_window_notifier) = device.vru_window.window_notifier {
                vru_window_notifier.try_send(None).unwrap();
            }

            weak.upgrade_in_event_loop(move |handle| handle.set_game_running(false))
                .unwrap();
        })
        .unwrap();
}

fn open_rom(app: &AppWindow) {
    let select_rom = rfd::AsyncFileDialog::new()
        .set_title("Select ROM")
        .pick_file();
    let mut select_gb_rom = [None, None, None, None];
    let mut select_gb_ram = [None, None, None, None];

    for (i, transfer_pak_enabled) in app.get_transferpak().iter().enumerate() {
        if transfer_pak_enabled {
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

    #[allow(clippy::type_complexity)]
    let (vru_window_notifier, vru_window_receiver): (
        tokio::sync::mpsc::Sender<Option<Vec<String>>>,
        tokio::sync::mpsc::Receiver<Option<Vec<String>>>,
    ) = tokio::sync::mpsc::channel(5);

    let (vru_word_notifier, vru_word_receiver): (
        tokio::sync::mpsc::Sender<String>,
        tokio::sync::mpsc::Receiver<String>,
    ) = tokio::sync::mpsc::channel(5);

    let fullscreen = app.get_fullscreen();
    let overclock = app.get_overclock_n64_cpu();
    let emulate_vru = app.get_emulate_vru();

    if emulate_vru {
        setup_vru_word_watcher(app.as_weak(), vru_word_notifier, vru_window_receiver);
    }
    let weak = app.as_weak();
    tokio::spawn(async move {
        if let Some(file) = select_rom.await {
            let mut gb_rom_path = [None, None, None, None];
            let mut gb_ram_path = [None, None, None, None];

            for i in 0..4 {
                if let (Some(gb_rom), Some(gb_ram)) =
                    (select_gb_rom[i].as_mut(), select_gb_ram[i].as_mut())
                    && let (Some(gb_rom), Some(gb_ram)) = (gb_rom.await, gb_ram.await)
                {
                    gb_rom_path[i] = Some(gb_rom.path().to_path_buf());
                    gb_ram_path[i] = Some(gb_ram.path().to_path_buf());
                }
            }

            run_rom(
                GbPaths {
                    rom: gb_rom_path,
                    ram: gb_ram_path,
                },
                file.path().to_path_buf(),
                fullscreen,
                overclock,
                if emulate_vru {
                    VruChannel {
                        vru_window_notifier: Some(vru_window_notifier),
                        vru_word_receiver: Some(vru_word_receiver),
                    }
                } else {
                    VruChannel {
                        vru_window_notifier: None,
                        vru_word_receiver: None,
                    }
                },
                None,
                weak,
            );
        }
    });
}
