use crate::ui;

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
        let command = if cfg!(target_os = "windows") {
            "explorer"
        } else if cfg!(target_os = "linux") {
            "xdg-open"
        } else {
            panic!("Unsupported platform");
        };
        let _ = std::process::Command::new(command)
            .arg(saves_path.clone())
            .spawn();
    });
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
    app.run().unwrap();
}
