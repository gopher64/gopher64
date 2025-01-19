use crate::device;
use crate::ui::gui::GopherEguiApp;
use eframe::egui;
use sha2::{Digest, Sha256};

const NETPLAY_VERSION: i32 = 17;
const EMU_NAME: &str = "gopher64";
pub struct Netplay {
    pub create: bool,
    pub join: bool,
    pub wait: bool,
    pub session_name: String,
    pub password: String,
    pub player_name: String,
    pub error: String,
    pub rom_label: String,
    pub server: (String, String),
    pub game_info: (String, String, String),
    pub servers: std::collections::HashMap<String, String>,
    pub server_receiver:
        Option<tokio::sync::mpsc::Receiver<std::collections::HashMap<String, String>>>,
    pub game_info_receiver: Option<tokio::sync::mpsc::Receiver<(String, String, String)>>,
    pub broadcast_socket: Option<std::net::UdpSocket>,
    pub broadcast_timer: Option<std::time::Instant>,
}

#[derive(serde::Serialize, serde::Deserialize, std::fmt::Debug)]
pub struct NetplayRoom {
    room_name: String,
    password: Option<String>,
    protected: Option<bool>,
    #[serde(rename = "MD5")]
    md5: String,
    game_name: String,
    port: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, std::fmt::Debug)]
pub struct NetplayMessage {
    #[serde(rename = "type")]
    message_type: String,
    player_name: Option<String>,
    client_sha: Option<String>,
    netplay_version: Option<i32>,
    emulator: Option<String>,
    room: Option<NetplayRoom>,
    accept: Option<i32>,
    message: Option<String>,
    auth: Option<String>,
    #[serde(rename = "authTime")]
    auth_time: Option<String>,
}

pub fn netplay_create(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Create Netplay Session").show(ctx, |ui| {
        egui::Grid::new("button_grid").show(ui, |ui| {
            let profile_name_label = ui.label("Session Name:");
            let mut size = ui.spacing().interact_size;
            size.x = 200.0;
            ui.add_sized(size, |ui: &mut egui::Ui| {
                ui.text_edit_singleline(&mut app.netplay.session_name)
                    .labelled_by(profile_name_label.id)
            });

            ui.end_row();

            let password_label = ui.label("Password (Optional):");

            ui.text_edit_singleline(&mut app.netplay.password)
                .labelled_by(password_label.id);

            ui.end_row();

            ui.label("ROM");
            if ui.button(app.netplay.rom_label.clone()).clicked() {
                let task = rfd::AsyncFileDialog::new().pick_file();
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                app.netplay.game_info_receiver = Some(rx);
                let gui_ctx = ctx.clone();
                tokio::spawn(async move {
                    let file = task.await;

                    if let Some(file) = file {
                        let rom_contents = device::get_rom_contents(file.path());
                        let hash = device::cart_rom::calculate_hash(&rom_contents);
                        let game_name = std::str::from_utf8(&rom_contents[0x20..0x20 + 0x14])
                            .unwrap()
                            .to_string();
                        let _ = tx.send((hash, game_name, file.file_name())).await;
                        gui_ctx.request_repaint();
                    }
                });
            }

            ui.end_row();

            let player_name_label = ui.label("Player Name:");

            ui.text_edit_singleline(&mut app.netplay.player_name)
                .labelled_by(player_name_label.id);

            ui.end_row();

            ui.label("Server:");

            if app.netplay.servers.is_empty() {
                if app.netplay.broadcast_socket.is_none() {
                    app.netplay.broadcast_socket = Some(
                        std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
                            .expect("couldn't bind to address"),
                    );
                    let socket = app.netplay.broadcast_socket.as_ref().unwrap();
                    socket
                        .set_broadcast(true)
                        .expect("set_broadcast call failed");
                    socket
                        .set_nonblocking(true)
                        .expect("could not set up socket");
                    let data: [u8; 1] = [1];
                    socket
                        .send_to(&data, (std::net::Ipv4Addr::BROADCAST, 45000))
                        .expect("couldn't send data");
                    app.netplay.broadcast_timer =
                        Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
                }
                if app.netplay.server_receiver.is_none() {
                    let (tx, rx) = tokio::sync::mpsc::channel(1);
                    app.netplay.server_receiver = Some(rx);
                    let gui_ctx = ctx.clone();
                    tokio::spawn(async move {
                        if let Ok(response) =
                            reqwest::get("https://m64p.s3.amazonaws.com/servers.json").await
                        {
                            if let Ok(servers) = response
                                .json::<std::collections::HashMap<String, String>>()
                                .await
                            {
                                let _ = tx.send(servers).await;
                                gui_ctx.request_repaint();
                            }
                        }
                    });
                }
            }
            if app.netplay.broadcast_timer.is_some()
                && std::time::Instant::now() > app.netplay.broadcast_timer.unwrap()
            {
                app.netplay.broadcast_timer = None;
            }
            if app.netplay.broadcast_socket.is_some() && app.netplay.broadcast_timer.is_some() {
                let mut buffer = [0; 1024];
                let result = app
                    .netplay
                    .broadcast_socket
                    .as_ref()
                    .unwrap()
                    .recv_from(&mut buffer);
                if result.is_ok() {
                    let (amt, _src) = result.unwrap();
                    let data: std::collections::HashMap<String, String> =
                        serde_json::from_slice(&buffer[..amt]).unwrap();
                    for server in data.iter() {
                        let (server_name, server_ip) = server;
                        app.netplay
                            .servers
                            .insert(server_name.to_string(), server_ip.to_string());
                        app.netplay.server = (server.0.clone(), server.1.clone());
                    }
                    app.netplay.broadcast_socket = None;
                }
            }
            if app.netplay.server_receiver.is_some() {
                let result = app.netplay.server_receiver.as_mut().unwrap().try_recv();
                if result.is_ok() {
                    app.netplay.servers.extend(result.unwrap());
                    app.netplay.server_receiver = None;
                    if app.netplay.server.0.is_empty() {
                        let first_server = app.netplay.servers.iter().next().unwrap();
                        app.netplay.server = (first_server.0.clone(), first_server.1.clone());
                    }
                }
            }
            if app.netplay.game_info_receiver.is_some() {
                let result = app.netplay.game_info_receiver.as_mut().unwrap().try_recv();
                if result.is_ok() {
                    app.netplay.game_info = result.unwrap();
                    app.netplay.game_info_receiver = None;
                    app.netplay.rom_label = app.netplay.game_info.2.clone();
                }
            }

            egui::ComboBox::from_id_salt("server-combobox")
                .selected_text(app.netplay.server.0.to_string())
                .show_ui(ui, |ui| {
                    for server in app.netplay.servers.iter() {
                        ui.selectable_value(
                            &mut app.netplay.server,
                            (server.0.clone(), server.1.clone()),
                            server.0,
                        );
                    }
                });

            ui.end_row();

            if ui.button("Create Session").clicked() {
                if app.netplay.player_name.is_empty() {
                    app.netplay.error = "Player Name cannot be empty".to_string();
                } else if app.netplay.session_name.is_empty() {
                    app.netplay.error = "Session Name cannot be empty".to_string();
                } else if app.netplay.game_info.0.is_empty() {
                    app.netplay.error = "ROM not loaded".to_string();
                } else {
                    let now_utc = chrono::Utc::now().timestamp_millis().to_string();
                    let hasher = Sha256::new()
                        .chain_update(now_utc.clone())
                        .chain_update(EMU_NAME);
                    let netplay_message = NetplayMessage {
                        message_type: "request_create_room".to_string(),
                        player_name: Some(app.netplay.player_name.clone()),
                        client_sha: Some(env!("CARGO_PKG_VERSION").to_string()),
                        netplay_version: Some(NETPLAY_VERSION),
                        emulator: Some(EMU_NAME.to_string()),
                        accept: None,
                        message: None,
                        auth_time: Some(now_utc),
                        auth: Some(format!("{:x}", hasher.finalize())),
                        room: Some(NetplayRoom {
                            room_name: app.netplay.session_name.clone(),
                            password: Some(app.netplay.password.clone()),
                            game_name: app.netplay.game_info.1.trim().to_string(),
                            md5: app.netplay.game_info.0.clone(),
                            protected: None,
                            port: None,
                        }),
                    };
                    let (mut socket, _response) =
                        tungstenite::connect(app.netplay.server.1.clone()).expect("Can't connect");
                    socket
                        .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                            serde_json::to_vec(&netplay_message).unwrap(),
                        )))
                        .unwrap();
                    let data = socket.read().unwrap().into_data();
                    let message: NetplayMessage = serde_json::from_slice(data.as_ref()).unwrap();
                    if message.accept.unwrap() == 0 {
                        app.netplay.create = false;
                        app.netplay.wait = true;
                    } else {
                        app.netplay.error = message.message.unwrap();
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    app.netplay.create = false
                };
            })
        });
    });
}

pub fn netplay_join(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Join Netplay Session").show(ctx, |ui| {
        if ui.button("Close").clicked() {
            app.netplay.join = false
        };
    });
}

pub fn netplay_wait(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Pending Netplay Session").show(ctx, |ui| {
        if ui.button("Close").clicked() {
            app.netplay.wait = false
        };
    });
}

pub fn netplay_error(app: &mut GopherEguiApp, ctx: &egui::Context, error: String) {
    egui::Window::new("Netplay Error").show(ctx, |ui| {
        ui.label(error);
        if ui.button("Close").clicked() {
            app.netplay.error = "".to_string();
        };
    });
}
