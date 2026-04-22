use crate::retroachievements;
use crate::ui;
use slint::ComponentHandle;

pub fn ra_window(app: &ui::gui::AppWindow) {
    let mut token = String::new();
    if let Ok(ra_config) = std::fs::read(ui::get_dirs().config_dir.join("retroachievements.json"))
        && let Ok(result) =
            serde_json::from_slice::<retroachievements::RAConfig>(ra_config.as_ref())
    {
        app.set_ra_username(result.username.into());
        app.set_ra_enabled(result.enabled);
        app.set_ra_hardcore(result.hardcore);
        app.set_ra_challenge(result.challenge);
        app.set_ra_leaderboard(result.leaderboard);
        token = result.token;
    } else {
        app.set_ra_hardcore(true);
    }

    if !cfg!(ra_hardcore_enabled) {
        app.set_ra_softcore_only(true);
    }

    if app.get_ra_enabled() && !app.get_ra_username().is_empty() {
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
        retroachievements::login_token_user(app.get_ra_username().to_string(), token, tx);
        set_current_user_message(app, rx);
    } else {
        app.set_ra_current_user_message("Not currently logged in".into());
    }

    let weak_app2 = app.as_weak();
    app.on_ra_button_clicked(move |password| {
        weak_app2
            .upgrade_in_event_loop(move |handle| {
                let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                retroachievements::login_user(
                    handle.get_ra_username().to_string(),
                    password.to_string(),
                    tx,
                );

                set_current_user_message(&handle, rx);
            })
            .unwrap();
    });

    app.on_ra_toggled(move |enabled, hardcore, challenge, leaderboard| {
        let file_path = ui::get_dirs().config_dir.join("retroachievements.json");
        let raconfig = if let Ok(ra_config) = std::fs::read(&file_path)
            && let Ok(result) =
                serde_json::from_slice::<retroachievements::RAConfig>(ra_config.as_ref())
        {
            if !enabled {
                retroachievements::logout_user();
                retroachievements::RAConfig {
                    username: String::new(),
                    token: String::new(),
                    enabled,
                    hardcore,
                    challenge,
                    leaderboard,
                }
            } else {
                retroachievements::RAConfig {
                    username: result.username,
                    token: result.token,
                    enabled,
                    hardcore,
                    challenge,
                    leaderboard,
                }
            }
        } else {
            retroachievements::RAConfig {
                username: String::new(),
                token: String::new(),
                enabled,
                hardcore,
                challenge,
                leaderboard,
            }
        };
        let f = std::fs::File::create(&file_path).unwrap();
        serde_json::to_writer_pretty(f, &raconfig).unwrap();
    });

    app.on_ra_games_clicked(move || {
        open::that_detached("https://retroachievements.org/system/2-nintendo-64/games").unwrap();
    });

    app.on_ra_show_profile_clicked(move || {
        open::that_detached(format!(
            "https://retroachievements.org/user/{}",
            retroachievements::get_username().unwrap_or_default()
        ))
        .unwrap();
    });
}

fn set_current_user_message(app: &ui::gui::AppWindow, rx: tokio::sync::oneshot::Receiver<bool>) {
    app.set_ra_current_user_message("Logging in...".into());
    let weak_app = app.as_weak();
    tokio::spawn(async move {
        rx.await.unwrap();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                if let Some(username) = retroachievements::get_username() {
                    handle.set_ra_current_user_message(format!("Logged in as {}", username).into());
                    handle.set_ra_show_profile(true);
                } else {
                    handle.set_ra_current_user_message("Login failed".into());
                    handle.set_ra_show_profile(false);
                }
                handle.set_ra_logging_in(false);
            })
            .unwrap();
    });
}
