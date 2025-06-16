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

fn settings_window(app: &AppWindow) {
    let config = ui::config::Config::new();
    app.set_integer_scaling(config.video.integer_scaling);
    app.set_fullscreen(config.video.fullscreen);
    app.set_widescreen(config.video.widescreen);
    app.set_apply_crt_shader(config.video.crt);
    app.set_overclock_n64_cpu(config.emulation.overclock);
    app.set_resolution(format!("{}x", config.video.upscale).into());

    app.set_emulate_vru(config.input.emulate_vru);

    let controller_enabled_model: std::rc::Rc<slint::VecModel<bool>> = std::rc::Rc::new(
        slint::VecModel::from(config.input.controller_enabled.to_vec()),
    );
    app.set_controller_enabled(slint::ModelRc::from(controller_enabled_model));
    let transferpak_enabled_model: std::rc::Rc<slint::VecModel<bool>> =
        std::rc::Rc::new(slint::VecModel::from(config.input.transfer_pak.to_vec()));
    app.set_transferpak(slint::ModelRc::from(transferpak_enabled_model));
}

fn save_settings(app: &AppWindow) {
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
    settings_window(&app);
    app.run().unwrap();
    save_settings(&app);
}
