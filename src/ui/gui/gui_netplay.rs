use crate::device;
use crate::ui::gui::GopherEguiApp;
use crate::ui::{self, gui};
use eframe::egui;
use sha2::{Digest, Sha256};

const NETPLAY_VERSION: i32 = 17;
const EMU_NAME: &str = "gopher64";

#[derive(Default)]
pub struct GameInfo {
    pub md5: String,
    pub game_name: String,
    pub rom_label: String,
    pub rom_contents: Vec<u8>,
}

#[derive(PartialEq, Default)]
pub struct NetplayServer {
    pub name: String,
    pub ip: String,
}

#[derive(Default)]
pub struct GuiNetplay {
    pub create: bool,
    pub join: bool,
    pub wait: bool,
    pub session_name: String,
    pub password: String,
    pub player_name: String,
    pub error: String,
    pub create_rom_label: String,
    pub join_rom_label: String,
    pub send_chat: bool,
    pub have_sessions: Option<NetplayServer>,
    pub begin_game: bool,
    pub chat_log: String,
    pub chat_message: String,
    pub selected_session: Option<NetplayRoom>,
    pub pending_begin: bool,
    pub peer_addr: Option<std::net::SocketAddr>,
    pub motd: String,
    pub sessions: Vec<NetplayRoom>,
    pub rom_contents: Vec<u8>,
    pub player_number: u8,
    pub player_names: [String; 4],
    pub server: NetplayServer,
    pub socket_waiting: bool,
    pub game_info: GameInfo,
    pub servers: Vec<NetplayServer>,
    pub waiting_session: Option<NetplayRoom>,
    pub socket:
        Option<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>>,
    pub server_receiver:
        Option<tokio::sync::mpsc::Receiver<std::collections::HashMap<String, String>>>,
    pub game_info_receiver: Option<tokio::sync::mpsc::Receiver<GameInfo>>,
    pub broadcast_socket: Option<std::net::UdpSocket>,
    pub broadcast_timer: Option<std::time::Instant>,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Default, Clone)]
pub struct NetplayRoom {
    room_name: Option<String>,
    password: Option<String>,
    protected: Option<bool>,
    #[serde(rename = "MD5")]
    md5: Option<String>,
    game_name: Option<String>,
    pub port: Option<i32>,
    features: Option<std::collections::HashMap<String, String>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
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
    player_names: Option<[String; 4]>,
    #[serde(rename = "authTime")]
    auth_time: Option<String>,
    rooms: Option<Vec<NetplayRoom>>,
}

fn get_servers(app: &mut GopherEguiApp, ctx: &egui::Context) {
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
            ctx.request_repaint();
        }
        if app.netplay.server_receiver.is_none() {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            app.netplay.server_receiver = Some(rx);
            let gui_ctx = ctx.clone();
            let task = reqwest::get("https://m64p.s3.amazonaws.com/servers.json");
            tokio::spawn(async move {
                let response = task.await;
                if let Ok(response) = response {
                    tx.send(response.json().await.unwrap()).await.unwrap();
                    gui_ctx.request_repaint();
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
                app.netplay.servers.push(NetplayServer {
                    name: server_name.to_string(),
                    ip: server_ip.to_string(),
                });
                app.netplay.server = NetplayServer {
                    name: server.0.clone(),
                    ip: server.1.clone(),
                };
            }
            app.netplay.broadcast_socket = None;
        }
        ctx.request_repaint();
    }
    if app.netplay.server_receiver.is_some() {
        let result = app.netplay.server_receiver.as_mut().unwrap().try_recv();
        if result.is_ok() {
            for server in result.unwrap().iter() {
                app.netplay.servers.push(NetplayServer {
                    name: server.0.clone(),
                    ip: server.1.clone(),
                });
            }
            app.netplay.server_receiver = None;
            if app.netplay.server.name.is_empty() {
                let first_server = app.netplay.servers.first().unwrap();
                app.netplay.server = NetplayServer {
                    name: first_server.name.clone(),
                    ip: first_server.ip.clone(),
                };
            }
        }
        ctx.request_repaint();
    }
}

pub fn netplay_create(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Create Netplay Session").show(ctx, |ui| {
        egui::Grid::new("netplay_create_grid").show(ui, |ui| {
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
            if app.netplay.create_rom_label.is_empty() {
                app.netplay.create_rom_label = "Open ROM".to_string();
            }
            if ui.button(&app.netplay.create_rom_label).clicked() {
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                app.netplay.game_info_receiver = Some(rx);
                let gui_ctx = ctx.clone();
                app.netplay.create_rom_label = "Inspecting ROM".to_string();
                let task = rfd::AsyncFileDialog::new()
                    .set_title("Select ROM")
                    .pick_file();
                tokio::spawn(async move {
                    let file = task.await;

                    if let Some(file) = file {
                        parse_rom_file(file, tx).await;
                    } else {
                        tx.send(GameInfo {
                            md5: "".to_string(),
                            game_name: "".to_string(),
                            rom_label: "Open ROM".to_string(),
                            rom_contents: vec![],
                        })
                        .await
                        .unwrap();
                    }
                    gui_ctx.request_repaint();
                });
            }

            ui.end_row();

            let player_name_label = ui.label("Player Name:");

            ui.text_edit_singleline(&mut app.netplay.player_name)
                .labelled_by(player_name_label.id);

            ui.end_row();

            ui.label("Server:");

            get_servers(app, ctx);

            if app.netplay.game_info_receiver.is_some() {
                let result = app.netplay.game_info_receiver.as_mut().unwrap().try_recv();
                if result.is_ok() {
                    app.netplay.game_info_receiver = None;
                    let data = result.unwrap();
                    if !data.md5.is_empty() {
                        app.netplay.game_info = data;
                        app.netplay.rom_contents = app.netplay.game_info.rom_contents.clone();
                        app.netplay.create_rom_label = app.netplay.game_info.rom_label.clone();
                    } else {
                        app.netplay.create_rom_label = data.rom_label;
                    }
                }
                ctx.request_repaint();
            }

            egui::ComboBox::from_id_salt("server-combobox")
                .selected_text(app.netplay.server.name.to_string())
                .show_ui(ui, |ui| {
                    for server in app.netplay.servers.iter() {
                        ui.selectable_value(
                            &mut app.netplay.server,
                            NetplayServer {
                                name: server.name.clone(),
                                ip: server.ip.clone(),
                            },
                            server.name.clone(),
                        );
                    }
                });

            ui.end_row();

            if app.netplay.socket_waiting {
                let data = app.netplay.socket.as_mut().unwrap().read();
                if data.is_ok() {
                    let message: NetplayMessage =
                        serde_json::from_slice(&data.unwrap().into_data()).unwrap();
                    if message.accept.unwrap() == 0 {
                        if message.message_type == "reply_create_room" {
                            app.netplay.create = false;
                            app.netplay.wait = true;
                            app.netplay.waiting_session = Some(message.room.unwrap());
                        }
                    } else {
                        app.netplay.error = message.message.unwrap();
                    }
                    app.netplay.socket_waiting = false;
                }
                ctx.request_repaint();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Create Session").clicked() {
                if app.netplay.player_name.is_empty() {
                    app.netplay.error = "Player Name cannot be empty".to_string();
                } else if app.netplay.session_name.is_empty() {
                    app.netplay.error = "Session Name cannot be empty".to_string();
                } else if app.netplay.game_info.md5.is_empty() {
                    app.netplay.error = "ROM not loaded".to_string();
                } else {
                    let now_utc = chrono::Utc::now().timestamp_millis().to_string();
                    let hasher = Sha256::new().chain_update(&now_utc).chain_update(EMU_NAME);
                    let mut game_name = app.netplay.game_info.game_name.to_string();
                    if game_name.is_empty() {
                        // If the ROM doesn't report a name, use the filename
                        game_name = app.netplay.create_rom_label.clone();
                    }
                    let mut features = std::collections::HashMap::new();
                    features.insert("overclock".to_string(), app.overclock.to_string());
                    let netplay_message = NetplayMessage {
                        message_type: "request_create_room".to_string(),
                        player_name: Some(app.netplay.player_name.clone()),
                        client_sha: Some(env!("GIT_HASH").to_string()),
                        netplay_version: Some(NETPLAY_VERSION),
                        emulator: Some(EMU_NAME.to_string()),
                        accept: None,
                        message: None,
                        rooms: None,
                        auth_time: Some(now_utc),
                        player_names: None,
                        auth: Some(format!("{:x}", hasher.finalize())),
                        room: Some(NetplayRoom {
                            room_name: Some(app.netplay.session_name.clone()),
                            password: Some(app.netplay.password.clone()),
                            game_name: Some(game_name),
                            md5: Some(app.netplay.game_info.md5.clone()),
                            protected: None,
                            port: None,
                            features: Some(features),
                        }),
                    };
                    let (mut socket, _response) =
                        tungstenite::connect(&app.netplay.server.ip).expect("Can't connect");
                    socket
                        .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                            serde_json::to_vec(&netplay_message).unwrap(),
                        )))
                        .unwrap();
                    match socket.get_mut() {
                        tungstenite::stream::MaybeTlsStream::Plain(stream) => {
                            app.netplay.peer_addr = Some(stream.peer_addr().unwrap());
                            stream.set_nonblocking(true)
                        }
                        _ => unimplemented!(),
                    }
                    .expect("could not set socket to non-blocking");
                    app.netplay.socket_waiting = true;
                    app.netplay.socket = Some(socket);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    if let Some(socket) = app.netplay.socket.as_mut() {
                        socket.close(None).unwrap();
                        loop {
                            match socket.read() {
                                Err(tungstenite::Error::ConnectionClosed) => break,
                                _ => continue,
                            };
                        }
                    }
                    app.netplay = Default::default();
                };
            })
        });
    });
}

fn get_sessions(app: &mut GopherEguiApp, ctx: &egui::Context) {
    if app.netplay.have_sessions.is_some()
        && app.netplay.server != *app.netplay.have_sessions.as_ref().unwrap()
    {
        // User has changed the server
        app.netplay.have_sessions = None;
        app.netplay.socket = None;
    }
    if app.netplay.socket.is_none() {
        let (mut sock, _response) =
            tungstenite::connect(&app.netplay.server.ip).expect("Can't connect");
        match sock.get_mut() {
            tungstenite::stream::MaybeTlsStream::Plain(stream) => {
                app.netplay.peer_addr = Some(stream.peer_addr().unwrap());
                stream.set_nonblocking(true)
            }
            _ => unimplemented!(),
        }
        .expect("could not set socket to non-blocking");
        app.netplay.socket = Some(sock);
    }
    let socket = app.netplay.socket.as_mut().unwrap();
    if app.netplay.have_sessions.is_none() {
        let now_utc = chrono::Utc::now().timestamp_millis().to_string();
        let hasher = Sha256::new().chain_update(&now_utc).chain_update(EMU_NAME);
        let request_rooms = NetplayMessage {
            message_type: "request_get_rooms".to_string(),
            player_name: None,
            client_sha: None,
            netplay_version: Some(NETPLAY_VERSION),
            player_names: None,
            emulator: Some(EMU_NAME.to_string()),
            accept: None,
            rooms: None,
            message: None,
            auth_time: Some(now_utc),
            auth: Some(format!("{:x}", hasher.finalize())),
            room: None,
        };

        socket
            .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                serde_json::to_vec(&request_rooms).unwrap(),
            )))
            .unwrap();
        app.netplay.have_sessions = Some(NetplayServer {
            name: app.netplay.server.name.clone(),
            ip: app.netplay.server.ip.clone(),
        });
        app.netplay.socket_waiting = true;
        ctx.request_repaint();
    }
}

async fn parse_rom_file(file: rfd::FileHandle, tx: tokio::sync::mpsc::Sender<GameInfo>) {
    let rom_contents = device::get_rom_contents(file.path());
    if !rom_contents.is_empty() {
        let hash = device::cart::rom::calculate_hash(&rom_contents);
        let game_name = ui::storage::get_game_name(&rom_contents);

        tx.send(GameInfo {
            md5: hash,
            game_name,
            rom_label: file.file_name(),
            rom_contents,
        })
        .await
        .unwrap();
    } else {
        tx.send(GameInfo {
            md5: "".to_string(),
            game_name: "".to_string(),
            rom_label: "Invalid ROM".to_string(),
            rom_contents: vec![],
        })
        .await
        .unwrap();
    }
}

pub fn netplay_join(app: &mut GopherEguiApp, ctx: &egui::Context) {
    if app.netplay.socket_waiting {
        let socket = app.netplay.socket.as_mut().unwrap();
        let data = socket.read();
        if data.is_ok() {
            let message: NetplayMessage =
                serde_json::from_slice(&data.unwrap().into_data()).unwrap();
            if message.accept.unwrap() == 0 {
                if message.message_type == "reply_get_rooms" {
                    if message.rooms.is_some() {
                        app.netplay.sessions = message.rooms.unwrap();
                    }
                } else if message.message_type == "reply_join_room" {
                    app.netplay.join = false;
                    app.netplay.wait = true;
                    app.netplay.waiting_session = Some(message.room.unwrap());
                }
                app.netplay.socket_waiting = false;
            } else {
                app.netplay.error = message.message.unwrap();
                app.netplay.join_rom_label = "Join Session (Open ROM)".to_string();
            }
        }
        ctx.request_repaint();
    }
    if app.netplay.game_info_receiver.is_some() {
        let result = app.netplay.game_info_receiver.as_mut().unwrap().try_recv();
        if result.is_ok() {
            app.netplay.game_info_receiver = None;
            let data = result.unwrap();
            if !data.md5.is_empty() {
                app.netplay.game_info = data;
                app.netplay.rom_contents = app.netplay.game_info.rom_contents.clone();

                let netplay_message = NetplayMessage {
                    message_type: "request_join_room".to_string(),
                    player_name: Some(app.netplay.player_name.clone()),
                    client_sha: Some(env!("GIT_HASH").to_string()),
                    netplay_version: None,
                    emulator: None,
                    accept: None,
                    message: None,
                    rooms: None,
                    auth_time: None,
                    player_names: None,
                    auth: None,
                    room: Some(NetplayRoom {
                        room_name: None,
                        password: Some(app.netplay.password.clone()),
                        game_name: None,
                        md5: Some(app.netplay.game_info.md5.clone()),
                        protected: None,
                        port: app.netplay.selected_session.as_ref().unwrap().port,
                        features: None,
                    }),
                };
                let socket = app.netplay.socket.as_mut().unwrap();
                socket
                    .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                        serde_json::to_vec(&netplay_message).unwrap(),
                    )))
                    .unwrap();

                app.netplay.socket_waiting = true;
            } else {
                app.netplay.error = data.rom_label;
                app.netplay.join_rom_label = "Join Session (Open ROM)".to_string();
            }
        }
        ctx.request_repaint();
    }
    egui::Window::new("Join Netplay Session").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let mut size = ui.spacing().interact_size;
            size.x = 100.0;
            ui.add_sized(
                size,
                egui::TextEdit::singleline(&mut app.netplay.player_name).hint_text("Player name"),
            );

            get_servers(app, ctx);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    app.netplay.socket = None;
                    app.netplay.have_sessions = None;
                    app.netplay.selected_session = None;
                    ctx.request_repaint();
                }
                egui::ComboBox::from_id_salt("server-combobox")
                    .selected_text(app.netplay.server.name.to_string())
                    .show_ui(ui, |ui| {
                        for server in app.netplay.servers.iter() {
                            ui.selectable_value(
                                &mut app.netplay.server,
                                NetplayServer {
                                    name: server.name.clone(),
                                    ip: server.ip.clone(),
                                },
                                server.name.clone(),
                            );
                        }
                    });
            });
        });
        if !app.netplay.server.name.is_empty() {
            get_sessions(app, ctx);
        }
        ui.add_space(16.0);
        if app.netplay.sessions.is_empty() {
            ui.label("No sessions available");
        } else {
            egui::Grid::new("netplay_join_grid").show(ui, |ui| {
                ui.label(egui::RichText::new("Session Name (click to select)").underline());
                ui.label(egui::RichText::new("Game Name").underline());
                ui.label(egui::RichText::new("Password Protected").underline());
                ui.end_row();

                for room in app.netplay.sessions.iter() {
                    ui.selectable_value(
                        &mut app.netplay.selected_session,
                        Some(room.clone()),
                        room.room_name.as_ref().unwrap(),
                    );
                    ui.label(room.game_name.as_ref().unwrap());
                    ui.label(room.protected.unwrap_or(false).to_string());
                    ui.end_row();
                }
            });
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            let mut size = ui.spacing().interact_size;
            size.x = 130.0;
            ui.add_sized(
                size,
                egui::TextEdit::singleline(&mut app.netplay.password)
                    .hint_text("Password (if required)"),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    if let Some(socket) = app.netplay.socket.as_mut() {
                        socket.close(None).unwrap();
                        loop {
                            match socket.read() {
                                Err(tungstenite::Error::ConnectionClosed) => break,
                                _ => continue,
                            };
                        }
                    }
                    app.netplay = Default::default();
                };
                if app.netplay.join_rom_label.is_empty() {
                    app.netplay.join_rom_label = "Join Session (Open ROM)".to_string();
                }
                if ui.button(&app.netplay.join_rom_label).clicked() {
                    if app.netplay.player_name.is_empty() {
                        app.netplay.error = "Player Name cannot be empty".to_string();
                    } else if app.netplay.selected_session.is_none() {
                        app.netplay.error = "No session selected".to_string();
                    } else if app
                        .netplay
                        .selected_session
                        .as_ref()
                        .unwrap()
                        .protected
                        .unwrap()
                        && app.netplay.password.is_empty()
                    {
                        app.netplay.error = "Session requires a password".to_string();
                    } else {
                        let (tx, rx) = tokio::sync::mpsc::channel(1);
                        app.netplay.game_info_receiver = Some(rx);
                        let gui_ctx = ctx.clone();
                        app.netplay.join_rom_label = "Inspecting ROM".to_string();
                        let task = rfd::AsyncFileDialog::new()
                            .set_title("Select ROM")
                            .pick_file();
                        tokio::spawn(async move {
                            let file = task.await;

                            if let Some(file) = file {
                                parse_rom_file(file, tx).await;
                            } else {
                                tx.send(GameInfo {
                                    md5: "".to_string(),
                                    game_name: "".to_string(),
                                    rom_label: "No ROM selected".to_string(),
                                    rom_contents: vec![],
                                })
                                .await
                                .unwrap();
                            }
                            gui_ctx.request_repaint();
                        });
                    }
                };
            });
        });
    });
}

pub fn netplay_wait(app: &mut GopherEguiApp, ctx: &egui::Context) {
    let overclock = app
        .netplay
        .waiting_session
        .as_ref()
        .unwrap()
        .features
        .as_ref()
        .unwrap()
        .get("overclock")
        .unwrap();

    let motd_message = NetplayMessage {
        message_type: "request_motd".to_string(),
        player_name: None,
        client_sha: None,
        netplay_version: None,
        emulator: None,
        accept: None,
        rooms: None,
        player_names: None,
        message: None,
        auth_time: None,
        auth: None,
        room: None,
    };
    let request_players = NetplayMessage {
        message_type: "request_players".to_string(),
        player_name: None,
        client_sha: None,
        netplay_version: None,
        player_names: None,
        emulator: None,
        rooms: None,
        accept: None,
        message: None,
        auth_time: None,
        auth: None,
        room: Some(NetplayRoom {
            room_name: None,
            password: None,
            game_name: None,
            md5: None,
            protected: None,
            port: app.netplay.waiting_session.as_ref().unwrap().port,
            features: None,
        }),
    };

    if !app.netplay.socket_waiting {
        let socket = app.netplay.socket.as_mut().unwrap();
        socket
            .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                serde_json::to_vec(&motd_message).unwrap(),
            )))
            .unwrap();
        socket
            .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                serde_json::to_vec(&request_players).unwrap(),
            )))
            .unwrap();
        app.netplay.socket_waiting = true;
    }

    if app.netplay.begin_game {
        let begin_game = NetplayMessage {
            message_type: "request_begin_game".to_string(),
            player_name: None,
            client_sha: None,
            netplay_version: None,
            player_names: None,
            rooms: None,
            emulator: None,
            accept: None,
            message: None,
            auth_time: None,
            auth: None,
            room: Some(NetplayRoom {
                room_name: None,
                password: None,
                game_name: None,
                md5: None,
                protected: None,
                port: app.netplay.waiting_session.as_ref().unwrap().port,
                features: None,
            }),
        };
        let socket = app.netplay.socket.as_mut().unwrap();
        socket
            .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                serde_json::to_vec(&begin_game).unwrap(),
            )))
            .unwrap();
        app.netplay.begin_game = false;
    }

    if app.netplay.send_chat {
        let send_chat = NetplayMessage {
            message_type: "request_chat_message".to_string(),
            player_name: Some(app.netplay.player_name.clone()),
            client_sha: None,
            netplay_version: None,
            player_names: None,
            rooms: None,
            emulator: None,
            accept: None,
            message: Some(app.netplay.chat_message.clone()),
            auth_time: None,
            auth: None,
            room: Some(NetplayRoom {
                room_name: None,
                password: None,
                game_name: None,
                md5: None,
                protected: None,
                port: app.netplay.waiting_session.as_ref().unwrap().port,
                features: None,
            }),
        };
        app.netplay.chat_message.clear();
        let socket = app.netplay.socket.as_mut().unwrap();
        socket
            .send(tungstenite::Message::Binary(tungstenite::Bytes::from(
                serde_json::to_vec(&send_chat).unwrap(),
            )))
            .unwrap();
        app.netplay.send_chat = false;
    }

    if app.netplay.socket_waiting {
        let socket = app.netplay.socket.as_mut().unwrap();
        let data = socket.read();
        if data.is_ok() {
            let message: NetplayMessage =
                serde_json::from_slice(&data.unwrap().into_data()).unwrap();
            if message.accept.unwrap() == 0 {
                if message.message_type == "reply_motd" {
                    let re = regex::Regex::new(r"<[^>]*>").unwrap();
                    app.netplay.motd = re
                        .replace_all(message.message.unwrap().as_str(), "")
                        .into_owned();
                } else if message.message_type == "reply_players" {
                    app.netplay.player_names = message.player_names.unwrap();
                } else if message.message_type == "reply_chat_message" {
                    app.netplay.chat_log.push_str(&message.message.unwrap());
                    app.netplay.chat_log.push('\n');
                } else if message.message_type == "reply_begin_game" {
                    let mut player = 0;
                    for (i, name) in app.netplay.player_names.iter().enumerate() {
                        if *name == app.netplay.player_name {
                            player = i;
                            break;
                        }
                    }
                    app.netplay.player_number = player as u8;

                    if message.accept.unwrap() == 0 {
                        socket.close(None).unwrap();
                        loop {
                            match socket.read() {
                                Err(tungstenite::Error::ConnectionClosed) => break,
                                _ => continue,
                            };
                        }

                        gui::open_rom(app, ctx, *overclock == "true");
                        app.netplay = Default::default();
                        return;
                    } else {
                        app.netplay.error = message.message.unwrap();
                    }
                }
            } else {
                app.netplay.error = message.message.unwrap();
            }
        }
        ctx.request_repaint();
    }

    egui::Window::new("Pending Netplay Session").show(ctx, |ui| {
        egui::Grid::new("netplay_wait_grid_1").show(ui, |ui| {
            ui.label("Session Name:");
            let room_name = app
                .netplay
                .waiting_session
                .as_ref()
                .unwrap()
                .room_name
                .as_ref()
                .unwrap();
            let game_name = app
                .netplay
                .waiting_session
                .as_ref()
                .unwrap()
                .game_name
                .as_ref()
                .unwrap();
            ui.label(room_name);
            ui.end_row();
            ui.label("Game Name:");
            ui.label(game_name);
            ui.end_row();
            for i in 0..4 {
                ui.label(format!("Player {}:", i + 1));
                ui.label(app.netplay.player_names[i].clone());
                ui.end_row();
            }
        });
        egui::ScrollArea::vertical()
            .max_height(100.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut app.netplay.chat_log)
                        .interactive(false)
                        .desired_width(ui.available_width()),
                );
            });

        ui.horizontal(|ui| {
            let mut size = ui.spacing().interact_size;
            size.x = 200.0;

            ui.add_sized(
                size,
                egui::TextEdit::singleline(&mut app.netplay.chat_message)
                    .hint_text("Enter chat message here"),
            );

            if ui
                .add_enabled(!app.netplay.send_chat, egui::Button::new("Send Message"))
                .clicked()
                && !app.netplay.chat_message.is_empty()
            {
                app.netplay.send_chat = true;
            }
        });

        ui.add_space(16.0);
        ui.label(app.netplay.motd.clone());
        ui.add_space(16.0);

        ui.horizontal(|ui| {
            let button_enabled = app.netplay.player_name == app.netplay.player_names[0]
                && !app.netplay.pending_begin;
            if ui
                .add_enabled(button_enabled, egui::Button::new("Start Session"))
                .clicked()
            {
                app.netplay.begin_game = true;
                app.netplay.pending_begin = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    if let Some(socket) = app.netplay.socket.as_mut() {
                        socket.close(None).unwrap();
                        loop {
                            match socket.read() {
                                Err(tungstenite::Error::ConnectionClosed) => break,
                                _ => continue,
                            };
                        }
                    }
                    app.netplay = Default::default();
                };
            });
        });
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
