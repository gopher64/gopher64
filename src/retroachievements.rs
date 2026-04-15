include!(concat!(env!("OUT_DIR"), "/retroachievements_bindings.rs"));

use crate::ui;
use slint::ComponentHandle;

static WEB_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("GIT_DESCRIBE")
        ))
        .build()
        .unwrap()
});

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RAConfig {
    pub username: String,
    pub token: String,
    pub enabled: bool,
    pub hardcore: bool,
    pub challenge: bool,
    pub leaderboard: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn notify_load_game(ctx: *mut std::ffi::c_void) {
    let tx = unsafe { Box::from_raw(ctx as *mut tokio::sync::oneshot::Sender<bool>) };
    tx.send(true).unwrap();
}

#[unsafe(no_mangle)]
pub extern "C" fn store_retroachievements_credentials(
    c_username: *const std::ffi::c_char,
    c_token: *const std::ffi::c_char,
    ctx: *mut std::ffi::c_void,
) {
    let tx = unsafe { Box::from_raw(ctx as *mut tokio::sync::oneshot::Sender<bool>) };

    if c_username.is_null() || c_token.is_null() {
        tx.send(false).unwrap();
        return;
    }

    let file_path = ui::get_dirs().config_dir.join("retroachievements.json");
    let raconfig = if let Ok(ra_config) = std::fs::read(&file_path)
        && let Ok(result) = serde_json::from_slice::<RAConfig>(ra_config.as_ref())
    {
        RAConfig {
            username: unsafe { std::ffi::CStr::from_ptr(c_username).to_str().unwrap() }.to_string(),
            token: unsafe { std::ffi::CStr::from_ptr(c_token).to_str().unwrap() }.to_string(),
            enabled: result.enabled,
            hardcore: result.hardcore,
            challenge: result.challenge,
            leaderboard: result.leaderboard,
        }
    } else {
        RAConfig {
            username: unsafe { std::ffi::CStr::from_ptr(c_username).to_str().unwrap() }.to_string(),
            token: unsafe { std::ffi::CStr::from_ptr(c_token).to_str().unwrap() }.to_string(),
            enabled: false,
            hardcore: false,
            challenge: false,
            leaderboard: false,
        }
    };
    let f = std::fs::File::create(&file_path).unwrap();
    serde_json::to_writer_pretty(f, &raconfig).unwrap();

    tx.send(true).unwrap();
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_server_call(
    c_url: *const std::ffi::c_char,
    c_post_data: *const std::ffi::c_char,
    c_content_type: *const std::ffi::c_char,
    c_callback: *mut std::ffi::c_void,
    c_callback_data: *mut std::ffi::c_void,
) {
    let url = unsafe { std::ffi::CStr::from_ptr(c_url).to_str().unwrap() }.to_string();

    let task = if !c_post_data.is_null() {
        let post_data =
            unsafe { std::ffi::CStr::from_ptr(c_post_data).to_str().unwrap() }.to_string();
        let content_type =
            unsafe { std::ffi::CStr::from_ptr(c_content_type).to_str().unwrap() }.to_string();
        WEB_CLIENT
            .post(url)
            .body(post_data)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .send()
    } else {
        WEB_CLIENT.get(url).send()
    };
    let callback = c_callback.addr();
    let callback_data = c_callback_data.addr();
    tokio::spawn(async move {
        let response = task.await;
        match response {
            Ok(response) => {
                let status = response.status().as_u16() as i32;
                match response.text().await {
                    Ok(text) => {
                        let c_text = std::ffi::CString::new(text).unwrap();
                        unsafe {
                            ra_http_callback(
                                c_text.as_ptr(),
                                c_text.count_bytes(),
                                status,
                                callback as *mut std::ffi::c_void,
                                callback_data as *mut std::ffi::c_void,
                            )
                        };
                    }
                    Err(error) => {
                        let c_error = std::ffi::CString::new(error.to_string()).unwrap();
                        unsafe {
                            ra_http_callback(
                                c_error.as_ptr(),
                                c_error.count_bytes(),
                                status,
                                callback as *mut std::ffi::c_void,
                                callback_data as *mut std::ffi::c_void,
                            )
                        };
                    }
                }
            }
            Err(error) => {
                let c_error = std::ffi::CString::new(error.to_string()).unwrap();
                unsafe {
                    ra_http_callback(
                        c_error.as_ptr(),
                        c_error.count_bytes(),
                        0,
                        callback as *mut std::ffi::c_void,
                        callback_data as *mut std::ffi::c_void,
                    )
                };
            }
        }
    });
}

fn set_current_user_message(app: &ui::gui::AppWindow, rx: tokio::sync::oneshot::Receiver<bool>) {
    app.set_ra_current_user_message("Logging in...".into());
    let weak_app = app.as_weak();
    tokio::spawn(async move {
        rx.await.unwrap();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                if is_user_logged_in() {
                    handle.set_ra_current_user_message(
                        format!("Logged in as {}", handle.get_ra_username()).into(),
                    );
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

pub fn ra_window(app: &ui::gui::AppWindow) {
    let mut token = String::new();
    if let Ok(ra_config) = std::fs::read(ui::get_dirs().config_dir.join("retroachievements.json"))
        && let Ok(result) = serde_json::from_slice::<RAConfig>(ra_config.as_ref())
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

    if app.get_ra_enabled() && !app.get_ra_username().is_empty() {
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
        login_token_user(app.get_ra_username().to_string(), token, tx);
        set_current_user_message(app, rx);
    } else {
        app.set_ra_current_user_message("Not currently logged in".into());
    }

    let weak_app2 = app.as_weak();
    app.on_ra_button_clicked(move |password| {
        weak_app2
            .upgrade_in_event_loop(move |handle| {
                let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                login_user(
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
            && let Ok(result) = serde_json::from_slice::<RAConfig>(ra_config.as_ref())
        {
            if !enabled {
                unsafe { ra_logout_user() };
                RAConfig {
                    username: "".into(),
                    token: "".into(),
                    enabled,
                    hardcore,
                    challenge,
                    leaderboard,
                }
            } else {
                RAConfig {
                    username: result.username,
                    token: result.token,
                    enabled,
                    hardcore,
                    challenge,
                    leaderboard,
                }
            }
        } else {
            RAConfig {
                username: "".into(),
                token: "".into(),
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

    app.on_ra_show_profile_clicked(move |username| {
        open::that_detached(format!("https://retroachievements.org/user/{}", username)).unwrap();
    });
}

pub async fn load_game(rom: &[u8], rom_size: usize) {
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    unsafe {
        let tx_ptr = Box::into_raw(Box::new(tx)) as *mut std::ffi::c_void;
        ra_load_game(rom.as_ptr(), rom_size, tx_ptr);
    };
    rx.await.unwrap();
}

pub fn welcome() {
    unsafe { ra_welcome() };
}

pub fn set_dmem(dmem: *const u8, dmem_size: usize) {
    unsafe { ra_set_dmem(dmem, dmem_size) };
}

pub fn do_frame() {
    unsafe { ra_do_frame() };
}

pub fn do_idle() {
    unsafe { ra_do_idle() };
}

pub fn init_client(hardcore: bool, challenge: bool, leaderboard: bool) {
    unsafe { ra_init_client(hardcore, challenge, leaderboard) };
}

pub fn shutdown_client() {
    unsafe { ra_shutdown_client() };
}

pub fn get_hardcore() -> bool {
    unsafe { ra_get_hardcore() }
}

pub fn login_user(username: String, password: String, tx: tokio::sync::oneshot::Sender<bool>) {
    unsafe {
        let tx_ptr = Box::into_raw(Box::new(tx)) as *mut std::ffi::c_void;
        let c_username = std::ffi::CString::new(username).unwrap();
        let c_password = std::ffi::CString::new(password).unwrap();
        ra_login_user(c_username.as_ptr(), c_password.as_ptr(), tx_ptr)
    };
}

pub fn login_token_user(username: String, token: String, tx: tokio::sync::oneshot::Sender<bool>) {
    unsafe {
        let tx_ptr = Box::into_raw(Box::new(tx)) as *mut std::ffi::c_void;
        let c_username = std::ffi::CString::new(username).unwrap();
        let c_token = std::ffi::CString::new(token).unwrap();
        ra_login_token_user(c_username.as_ptr(), c_token.as_ptr(), tx_ptr)
    };
}

pub fn is_user_logged_in() -> bool {
    unsafe { ra_is_user_logged_in() }
}

pub fn get_username() -> &'static str {
    let c_username = unsafe { ra_get_username() };
    if c_username.is_null() {
        "unknown"
    } else {
        unsafe { std::ffi::CStr::from_ptr(c_username) }
            .to_str()
            .unwrap()
    }
}

pub fn get_token() -> &'static str {
    let c_token = unsafe { ra_get_token() };
    if c_token.is_null() {
        "unknown"
    } else {
        unsafe { std::ffi::CStr::from_ptr(c_token) }
            .to_str()
            .unwrap()
    }
}

pub fn state_size() -> usize {
    unsafe { ra_state_size() }
}

pub fn save_state(state: *mut u8, state_size: usize) {
    unsafe { ra_save_state(state, state_size) };
}

pub fn load_state(state: *const u8, state_size: usize) {
    unsafe { ra_load_state(state, state_size) };
}
