use crate::ui;
use slint::Model;

slint::include_modules!();

#[derive(serde::Deserialize)]
struct GithubData {
    tag_name: String,
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

fn local_game(app: &AppWindow, dirs: &ui::Dirs) {
    app.on_open_rom_button_clicked(move || {
        //open rom
    });

    let saves_path = dirs.data_dir.join("saves");
    app.on_saves_folder_button_clicked(move || {
        open::that_detached(saves_path.clone()).unwrap();
    });
}

fn get_input_profiles(config: &ui::config::Config) -> Vec<String> {
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
    app.set_resolution(format!("{}x", config.video.upscale).into());
}

fn update_input_profiles(weak: &slint::Weak<AppWindow>, config: &ui::config::Config) {
    let profiles = get_input_profiles(&config);
    weak.upgrade_in_event_loop(move |handle| {
        let input_profiles = slint::VecModel::default();
        for profile in profiles {
            input_profiles.push(profile.into());
        }
        let input_profiles_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
            std::rc::Rc::new(input_profiles);
        handle.set_input_profiles(slint::ModelRc::from(input_profiles_model));
    })
    .unwrap();
}

fn controller_window(
    app: &AppWindow,
    config: &ui::config::Config,
    controller_names: &Vec<String>,
    controller_paths: &Vec<Option<String>>,
    dirs: &ui::Dirs,
) {
    let controller_enabled_model: std::rc::Rc<slint::VecModel<bool>> = std::rc::Rc::new(
        slint::VecModel::from(config.input.controller_enabled.to_vec()),
    );
    app.set_emulate_vru(config.input.emulate_vru);

    app.set_controller_enabled(slint::ModelRc::from(controller_enabled_model));

    let transferpak_enabled_model: std::rc::Rc<slint::VecModel<bool>> =
        std::rc::Rc::new(slint::VecModel::from(config.input.transfer_pak.to_vec()));
    app.set_transferpak(slint::ModelRc::from(transferpak_enabled_model));

    let profile_bindings = slint::VecModel::default();
    for binding in config.input.input_profile_binding.iter() {
        profile_bindings.push(binding.into());
    }
    let input_profile_binding_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
        std::rc::Rc::new(profile_bindings);
    app.set_input_profile_binding(slint::ModelRc::from(input_profile_binding_model));

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

    let game_running = dirs.cache_dir.join("game_running");
    let weak_app = app.as_weak();
    app.on_input_profile_button_clicked(move || {
        if game_running.exists() {
            return;
        }

        let dialog = InputProfileDialog::new().unwrap();
        let weak_dialog = dialog.as_weak();
        let weak_app2 = weak_app.clone();
        dialog.on_profile_creation_button_clicked(move || {
            let weak_app3 = weak_app2.clone();
            weak_dialog
                .upgrade_in_event_loop(move |handle| {
                    handle.hide().unwrap();
                    let profile_name = handle.get_profile_name().into();
                    let dinput = handle.get_dinput();

                    tokio::spawn(async move {
                        let mut game_ui = ui::Ui::new();
                        ui::input::configure_input_profile(&mut game_ui, profile_name, dinput);
                        update_input_profiles(&weak_app3, &game_ui.config);
                    });
                })
                .unwrap();
        });
        dialog.show().unwrap();
    });
}

fn save_settings(app: &AppWindow, controller_paths: &Vec<Option<String>>) {
    let mut config = ui::config::Config::new();
    config.video.integer_scaling = app.get_integer_scaling();
    config.video.fullscreen = app.get_fullscreen();
    config.video.widescreen = app.get_widescreen();
    config.video.crt = app.get_apply_crt_shader();
    config.emulation.overclock = app.get_overclock_n64_cpu();
    config.video.upscale = app.get_resolution().trim_end_matches('x').parse().unwrap();

    config.input.emulate_vru = app.get_emulate_vru();
    for (i, controller_enabled) in app.get_controller_enabled().iter().enumerate() {
        config.input.controller_enabled[i] = controller_enabled;
    }
    for (i, transferpak_enabled) in app.get_transferpak().iter().enumerate() {
        config.input.transfer_pak[i] = transferpak_enabled;
    }
    for (i, input_profile_binding) in app.get_input_profile_binding().iter().enumerate() {
        config.input.input_profile_binding[i] = input_profile_binding.into();
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
    app.on_newversion_button_clicked(move || {
        open::that_detached("https://github.com/gopher64/gopher64/releases/latest").unwrap();
    });
    app.set_version(format!("Version: {}", env!("CARGO_PKG_VERSION")).into());
    check_latest_version(app.as_weak());
}

pub fn app_window() {
    let dirs = ui::get_dirs();
    let app = AppWindow::new().unwrap();
    local_game(&app, &dirs);
    about_window(&app);
    let mut controller_paths;
    {
        let game_ui = ui::Ui::new();
        let mut controller_names = ui::input::get_controller_names(&game_ui);
        controller_names.insert(0, "None".into());
        controller_paths = ui::input::get_controller_paths(&game_ui);
        controller_paths.insert(0, None);
        settings_window(&app, &game_ui.config);
        controller_window(
            &app,
            &game_ui.config,
            &controller_names,
            &controller_paths,
            &dirs,
        );
    }
    app.run().unwrap();
    save_settings(&app, &controller_paths);
}
