use crate::device;
use crate::ui;
use crate::ui::gui::{
    AppWindow, CustomNetplayServer, ErrorDialog, GameSettings, GbPaths, NetplayCreate,
    NetplayDevice, NetplayJoin, NetplayWait, VruChannel, run_rom, save_settings,
};
use futures::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use slint::{ComponentHandle, Model};
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::protocol::Message;

const NETPLAY_VERSION: i32 = 17;
const EMU_NAME: &str = "gopher64";

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct NetplayRoom {
    room_name: Option<String>,
    password: Option<String>,
    protected: Option<bool>,
    #[serde(rename = "MD5")]
    md5: Option<String>,
    game_name: Option<String>,
    pub port: Option<i32>,
    features: Option<std::collections::HashMap<String, String>>,
    buffer_target: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
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

trait NetplayPages {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>);
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>);
    fn set_ping(&self, ping: slint::SharedString);
    fn set_game_name(&self, game_name: slint::SharedString);
    fn set_game_hash(&self, game_hash: slint::SharedString);
    fn set_game_cheats(&self, game_cheats: slint::SharedString);
    fn set_rom_path(&self, rom_path: slint::SharedString);
    fn set_peer_addr(&self, peer_addr: slint::SharedString);
    fn refresh_sessions(&self, _server: slint::SharedString) {
        // Default implementation does nothing
    }
    fn invoke_get_ping(&self, server_url: slint::SharedString);
    fn set_custom_server_url(&self, server_url: slint::SharedString);
}

impl NetplayPages for NetplayCreate {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>) {
        self.set_server_urls(urls);
    }
    fn set_ping(&self, ping: slint::SharedString) {
        self.set_ping(ping);
    }
    fn set_game_name(&self, game_name: slint::SharedString) {
        self.set_game_name(game_name);
    }
    fn set_game_hash(&self, game_hash: slint::SharedString) {
        self.set_game_hash(game_hash);
    }
    fn set_game_cheats(&self, game_cheats: slint::SharedString) {
        self.set_game_cheats(game_cheats);
    }
    fn set_rom_path(&self, rom_path: slint::SharedString) {
        self.set_rom_path(rom_path);
    }
    fn set_peer_addr(&self, peer_addr: slint::SharedString) {
        self.set_peer_addr(peer_addr);
    }
    fn invoke_get_ping(&self, server_url: slint::SharedString) {
        self.invoke_get_ping(server_url);
    }
    fn set_custom_server_url(&self, server_url: slint::SharedString) {
        self.set_custom_server_url(server_url);
    }
}

impl NetplayPages for NetplayJoin {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>) {
        self.set_server_urls(urls);
    }
    fn set_ping(&self, ping: slint::SharedString) {
        self.set_ping(ping);
    }
    fn refresh_sessions(&self, server: slint::SharedString) {
        self.invoke_refresh_session(server);
    }
    fn set_game_name(&self, game_name: slint::SharedString) {
        self.set_game_name(game_name);
    }
    fn set_game_hash(&self, game_hash: slint::SharedString) {
        self.set_game_hash(game_hash);
    }
    fn set_game_cheats(&self, game_cheats: slint::SharedString) {
        self.set_game_cheats(game_cheats);
    }
    fn set_rom_path(&self, rom_path: slint::SharedString) {
        self.set_rom_path(rom_path);
    }
    fn set_peer_addr(&self, peer_addr: slint::SharedString) {
        self.set_peer_addr(peer_addr);
    }
    fn invoke_get_ping(&self, server_url: slint::SharedString) {
        self.invoke_get_ping(server_url);
    }
    fn set_custom_server_url(&self, server_url: slint::SharedString) {
        self.set_custom_server_url(server_url);
    }
}

fn populate_server_names<T: ComponentHandle + NetplayPages + 'static>(weak: slint::Weak<T>) {
    let task = reqwest::get("https://cdn.gopher64.com/servers-gopher64.json");
    tokio::spawn(async move {
        let mut local_servers: Vec<(String, String)> = vec![];

        let broadcast_sock = tokio::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
            .await
            .unwrap();
        broadcast_sock.set_broadcast(true).unwrap();
        let data: [u8; 1] = [1];
        broadcast_sock
            .send_to(&data, (std::net::Ipv4Addr::BROADCAST, 45000))
            .await
            .unwrap();
        let mut buffer = [0; 1024];
        if let Ok(Ok(result)) = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            broadcast_sock.recv(&mut buffer),
        )
        .await
        {
            let data: std::collections::HashMap<String, String> =
                serde_json::from_slice(&buffer[..result]).unwrap();
            for server in data.iter() {
                local_servers.push((server.0.into(), server.1.into()));
            }
        }

        let response = task.await;
        if let Ok(response) = response {
            let servers: std::collections::HashMap<String, String> = response.json().await.unwrap();

            let weak2 = weak.clone();
            weak.upgrade_in_event_loop(move |handle| {
                let server_names: slint::VecModel<slint::SharedString> = slint::VecModel::default();
                let server_urls: slint::VecModel<slint::SharedString> = slint::VecModel::default();
                for local_server in local_servers {
                    server_names.push(local_server.0.into());
                    server_urls.push(local_server.1.into());
                }
                for server in servers {
                    server_names.push(server.0.into());
                    server_urls.push(server.1.into());
                }
                server_names.push("Custom".into());
                update_ping(weak2, server_urls.row_data(0).unwrap().into());
                handle.refresh_sessions(server_urls.row_data(0).unwrap());
                let server_names_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
                    std::rc::Rc::new(server_names);
                let server_urls_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
                    std::rc::Rc::new(server_urls);
                handle.set_server_names(slint::ModelRc::from(server_names_model));
                handle.set_server_urls(slint::ModelRc::from(server_urls_model));
            })
            .unwrap();
        }
    });
}

fn select_rom<T: ComponentHandle + NetplayPages + 'static>(
    weak: slint::Weak<T>,
    rom_dir: slint::SharedString,
) {
    let select_rom = if !rom_dir.is_empty()
        && let Ok(exists) = std::fs::exists(&rom_dir)
        && exists
    {
        rfd::AsyncFileDialog::new().set_directory(rom_dir)
    } else {
        rfd::AsyncFileDialog::new()
    }
    .set_title("Select ROM")
    .add_filter("ROM files", &ui::gui::N64_EXTENSIONS)
    .pick_file();
    tokio::spawn(async move {
        if let Some(file) = select_rom.await {
            if let Some(rom_contents) = device::get_rom_contents(file.path()) {
                let hash = device::cart::rom::calculate_hash(&rom_contents);
                let game_name = ui::storage::get_game_name(&rom_contents);
                let game_crc = ui::storage::get_game_crc(&rom_contents);
                let cheats = ui::config::Cheats::new();
                let mut parsed_cheats = "".to_string();
                if let Some(game_cheats) = cheats.cheats.get(&game_crc)
                    && !game_cheats.is_empty()
                {
                    parsed_cheats = serde_json::to_string(game_cheats).unwrap();
                }

                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_game_name(game_name.into());
                    handle.set_game_hash(hash.into());
                    handle.set_game_cheats(parsed_cheats.into());
                    handle.set_rom_path(file.path().to_str().unwrap().into());
                })
                .unwrap();
            } else {
                weak.upgrade_in_event_loop(move |handle| {
                    let message_dialog = ErrorDialog::new().unwrap();
                    let weak_dialog = message_dialog.as_weak();
                    message_dialog.on_close_clicked(move || {
                        weak_dialog.unwrap().window().hide().unwrap();
                    });
                    message_dialog.set_text("Could not read ROM".into());
                    message_dialog.show().unwrap();

                    handle.set_game_name("".into());
                    handle.set_game_hash("".into());
                    handle.set_game_cheats("".into());
                    handle.set_rom_path("".into());
                })
                .unwrap();
            }
        }
    });
}

fn update_ping<T: ComponentHandle + NetplayPages + 'static>(
    weak: slint::Weak<T>,
    server_url: String,
) {
    weak.upgrade_in_event_loop(move |handle| {
        handle.set_ping("Ping: Unknown".into());
    })
    .unwrap();
    tokio::spawn(async move {
        if let Ok(Ok((mut sock, _response))) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            tokio_tungstenite::connect_async(server_url),
        )
        .await
        {
            sock.send(Message::Ping(Vec::new().into())).await.unwrap();
            let start = std::time::Instant::now();

            if let Some(Ok(_response)) = sock.next().await {
                let elapsed = start.elapsed();
                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_ping(format!("Ping: {:.0} ms", elapsed.as_millis()).into());
                })
                .unwrap();
            }
            sock.close(None).await.unwrap();
        }
    });
}

fn show_custom_url_dialog<T: ComponentHandle + NetplayPages + 'static>(
    weak: slint::Weak<T>,
    server_url: slint::SharedString,
) {
    let url_dialog = CustomNetplayServer::new().unwrap();
    url_dialog.set_custom_server_url(server_url);
    let weak_dialog = url_dialog.as_weak();
    url_dialog.on_ok_clicked(move |server_url| {
        weak.upgrade_in_event_loop(move |handle| {
            handle.set_custom_server_url(server_url.clone());
            let prefix: slint::SharedString = "ws://".into();
            handle.invoke_get_ping(prefix + &server_url);
        })
        .unwrap();
        weak_dialog.unwrap().window().hide().unwrap();
    });
    url_dialog.show().unwrap();
}

pub fn setup_create_window(
    create_window: &NetplayCreate,
    game_settings: GameSettings,
    rom_dir: slint::SharedString,
    weak_app: slint::Weak<AppWindow>,
) {
    let (netplay_read_sender, netplay_read_receiver): (
        tokio::sync::broadcast::Sender<NetplayMessage>,
        tokio::sync::broadcast::Receiver<NetplayMessage>,
    ) = tokio::sync::broadcast::channel(5);

    let (netplay_write_sender, netplay_write_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    ) = tokio::sync::broadcast::channel(5);

    create_window.set_rom_dir(rom_dir);
    populate_server_names(create_window.as_weak());
    let weak = create_window.as_weak();
    create_window.on_get_ping(move |server_url| {
        update_ping(weak.clone(), server_url.to_string());
    });
    let weak = create_window.as_weak();
    create_window.on_get_custom_url(move || {
        let weak2 = weak.clone();
        weak.upgrade_in_event_loop(move |handle| {
            handle.set_ping("Ping: Unknown".into());
            show_custom_url_dialog(weak2, handle.get_custom_server_url());
        })
        .unwrap();
    });
    let weak = create_window.as_weak();
    create_window.on_select_rom(move |rom_dir| {
        select_rom(weak.clone(), rom_dir);
    });

    let weak = create_window.as_weak();
    create_window.on_create_session(
        move |server_url,
              session_name,
              player_name,
              game_name,
              game_hash,
              game_cheats,
              password| {
            let _ = netplay_write_sender.send(None); // close current websocket if any
            manage_websocket(
                server_url.to_string(),
                netplay_read_sender.clone(),
                netplay_write_receiver.resubscribe(),
                weak.clone(),
            );

            create_session(
                netplay_write_sender.clone(),
                netplay_read_receiver.resubscribe(),
                session_name.to_string(),
                player_name.to_string(),
                game_name.to_string(),
                game_hash.to_string(),
                game_cheats.to_string(),
                password.to_string(),
                game_settings.clone(),
                weak_app.clone(),
                weak.clone(),
            );
        },
    );

    create_window.show().unwrap();
}

fn manage_websocket<T: ComponentHandle + NetplayPages + 'static>(
    server_url: String,
    netplay_read_sender: tokio::sync::broadcast::Sender<NetplayMessage>,
    mut netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    weak: slint::Weak<T>,
) {
    tokio::spawn(async move {
        if let Ok(Ok((socket, _response))) = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio_tungstenite::connect_async(server_url.clone()),
        )
        .await
        {
            match socket.get_ref() {
                tokio_tungstenite::MaybeTlsStream::Plain(stream) => {
                    let addr = stream.peer_addr().unwrap().to_string();
                    weak.upgrade_in_event_loop(move |handle| {
                        handle.set_peer_addr(addr.into());
                    })
                    .unwrap();
                }
                _ => unimplemented!(),
            }

            let (mut write, mut read) = socket.split();
            tokio::spawn(async move {
                while let Some(Ok(response)) = read.next().await {
                    if let Ok(message) = serde_json::from_slice(&response.into_data()) {
                        let _ = netplay_read_sender.send(message);
                    }
                }
            });
            tokio::spawn(async move {
                loop {
                    match netplay_write_receiver.recv().await {
                        Ok(Some(response)) => {
                            write
                                .send(Message::Binary(Bytes::from(
                                    serde_json::to_vec(&response).unwrap(),
                                )))
                                .await
                                .unwrap();
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            panic!("netplay_write_receiver lagged");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break; // exit the loop if the receiver is closed
                        }
                    }
                }
                write.close().await.unwrap();
            });
        }
    });
}

fn show_netplay_error(message: String) {
    let message_dialog = ErrorDialog::new().unwrap();
    let weak_dialog = message_dialog.as_weak();
    message_dialog.on_close_clicked(move || {
        weak_dialog.unwrap().window().hide().unwrap();
    });
    message_dialog.set_text(message.into());
    message_dialog.show().unwrap();
}

fn clear_sessions(handle: &NetplayJoin, message: Option<String>) {
    handle.set_sessions(slint::ModelRc::default());
    handle.set_ports(slint::ModelRc::default());
    handle.set_current_session(-1);
    if let Some(message) = message {
        show_netplay_error(message);
    }
}

fn update_sessions(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<NetplayMessage>,
    weak: slint::Weak<NetplayJoin>,
) {
    tokio::spawn(async move {
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

        netplay_write_sender.send(Some(request_rooms)).unwrap();

        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            netplay_read_receiver.recv(),
        )
        .await
        {
            Ok(Ok(message)) => {
                if message.accept.unwrap() == 0 {
                    if let Some(rooms) = message.rooms {
                        weak.upgrade_in_event_loop(move |handle| {
                            let sessions_vec = slint::VecModel::default();
                            let ports_vec = slint::VecModel::default();
                            for room in rooms {
                                let session_vec = slint::VecModel::default();
                                session_vec.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(room.room_name.unwrap()),
                                ));
                                session_vec.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(room.game_name.unwrap()),
                                ));
                                session_vec.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(if room.protected.unwrap() {
                                        "True"
                                    } else {
                                        "False"
                                    }),
                                ));
                                session_vec.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(
                                        if room.features.unwrap_or_default().contains_key("cheats")
                                        {
                                            "True"
                                        } else {
                                            "False"
                                        },
                                    ),
                                ));
                                let session_model: std::rc::Rc<
                                    slint::VecModel<slint::StandardListViewItem>,
                                > = std::rc::Rc::new(session_vec);
                                sessions_vec.push(slint::ModelRc::from(session_model));
                                ports_vec.push(room.port.unwrap());
                            }
                            let rooms_model: std::rc::Rc<
                                slint::VecModel<slint::ModelRc<slint::StandardListViewItem>>,
                            > = std::rc::Rc::new(sessions_vec);
                            let ports_model: std::rc::Rc<slint::VecModel<i32>> =
                                std::rc::Rc::new(ports_vec);
                            handle.set_sessions(slint::ModelRc::from(rooms_model));
                            handle.set_ports(slint::ModelRc::from(ports_model));
                            handle.set_current_session(-1);
                        })
                        .unwrap();
                    } else {
                        weak.upgrade_in_event_loop(move |handle| {
                            clear_sessions(&handle, None);
                        })
                        .unwrap();
                    }
                } else {
                    weak.upgrade_in_event_loop(move |handle| {
                        clear_sessions(&handle, message.message);
                    })
                    .unwrap();
                }
            }
            Ok(Err(err)) => {
                panic!("netplay_read_receiver error: {err}");
            }
            Err(_) => {
                weak.upgrade_in_event_loop(move |handle| {
                    clear_sessions(&handle, Some("Server did not respond".to_string()));
                })
                .unwrap();
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn create_session(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<NetplayMessage>,
    session_name: String,
    player_name: String,
    game_name: String,
    game_hash: String,
    game_cheats: String,
    password: String,
    game_settings: GameSettings,
    weak_app: slint::Weak<AppWindow>,
    weak: slint::Weak<NetplayCreate>,
) {
    tokio::spawn(async move {
        let now_utc = chrono::Utc::now().timestamp_millis().to_string();
        let hasher = Sha256::new().chain_update(&now_utc).chain_update(EMU_NAME);
        let mut features = std::collections::HashMap::new();

        if !game_cheats.is_empty() {
            features.insert("cheats".to_string(), game_cheats);
        }

        features.insert("overclock".to_string(), game_settings.overclock.to_string());
        features.insert(
            "disable_expansion_pak".to_string(),
            game_settings.disable_expansion_pak.to_string(),
        );

        let create_room = NetplayMessage {
            message_type: "request_create_room".to_string(),
            player_name: Some(player_name),
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
                room_name: Some(session_name),
                password: Some(password),
                game_name: Some(game_name),
                md5: Some(game_hash),
                protected: None,
                port: None,
                features: Some(features),
                buffer_target: None,
            }),
        };

        netplay_write_sender.send(Some(create_room)).unwrap();

        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            netplay_read_receiver.recv(),
        )
        .await
        {
            Ok(Ok(message)) => {
                if message.accept.unwrap() == 0 {
                    weak.upgrade_in_event_loop(move |handle| {
                        let session = message.room.as_ref().unwrap();
                        let features_default = "false".to_string();
                        let cheats_default = "{}".to_string();
                        let overclock = session
                            .features
                            .as_ref()
                            .unwrap()
                            .get("overclock")
                            .unwrap_or(&features_default);
                        let disable_expansion_pak = session
                            .features
                            .as_ref()
                            .unwrap()
                            .get("disable_expansion_pak")
                            .unwrap_or(&features_default);
                        let cheats = session
                            .features
                            .as_ref()
                            .unwrap()
                            .get("cheats")
                            .unwrap_or(&cheats_default);
                        setup_wait_window(
                            netplay_write_sender,
                            netplay_read_receiver,
                            session.room_name.as_ref().unwrap().into(),
                            session.game_name.as_ref().unwrap().into(),
                            handle.get_rom_path(),
                            message.player_name.as_ref().unwrap().into(),
                            session.port.unwrap(),
                            true,
                            GameSettings {
                                fullscreen: game_settings.fullscreen,
                                overclock: overclock.parse().unwrap(),
                                disable_expansion_pak: disable_expansion_pak.parse().unwrap(),
                                cheats: serde_json::from_str(cheats).unwrap(),
                            },
                            handle.get_peer_addr(),
                            weak_app,
                        );
                        handle.window().hide().unwrap();
                    })
                    .unwrap();
                } else {
                    weak.upgrade_in_event_loop(move |handle| {
                        handle.set_pending_session(false);
                        if let Some(message) = message.message {
                            show_netplay_error(message);
                        }
                    })
                    .unwrap();
                }
            }
            Ok(Err(err)) => {
                panic!("netplay_read_receiver error: {err}");
            }
            Err(_) => {
                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_pending_session(false);
                    show_netplay_error("Server did not respond".to_string());
                })
                .unwrap();
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn join_session(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<NetplayMessage>,
    player_name: String,
    game_hash: String,
    password: String,
    port: i32,
    fullscreen: bool,
    weak_app: slint::Weak<AppWindow>,
    weak: slint::Weak<NetplayJoin>,
) {
    tokio::spawn(async move {
        let join_room = NetplayMessage {
            message_type: "request_join_room".to_string(),
            player_name: Some(player_name),
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
                password: Some(password),
                game_name: None,
                md5: Some(game_hash),
                protected: None,
                port: Some(port),
                features: None,
                buffer_target: None,
            }),
        };

        netplay_write_sender.send(Some(join_room)).unwrap();

        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            netplay_read_receiver.recv(),
        )
        .await
        {
            Ok(Ok(message)) => {
                if message.accept.unwrap() == 0 {
                    weak.upgrade_in_event_loop(move |handle| {
                        let session = message.room.as_ref().unwrap();
                        let features_default = "false".to_string();
                        let cheats_default = "{}".to_string();
                        let overclock = session
                            .features
                            .as_ref()
                            .unwrap()
                            .get("overclock")
                            .unwrap_or(&features_default);
                        let disable_expansion_pak = session
                            .features
                            .as_ref()
                            .unwrap()
                            .get("disable_expansion_pak")
                            .unwrap_or(&features_default);
                        let cheats = session
                            .features
                            .as_ref()
                            .unwrap()
                            .get("cheats")
                            .unwrap_or(&cheats_default);
                        setup_wait_window(
                            netplay_write_sender,
                            netplay_read_receiver,
                            session.room_name.as_ref().unwrap().into(),
                            session.game_name.as_ref().unwrap().into(),
                            handle.get_rom_path(),
                            message.player_name.as_ref().unwrap().into(),
                            session.port.unwrap(),
                            false,
                            GameSettings {
                                fullscreen,
                                overclock: overclock.parse().unwrap(),
                                disable_expansion_pak: disable_expansion_pak.parse().unwrap(),
                                cheats: serde_json::from_str(cheats).unwrap(),
                            },
                            handle.get_peer_addr(),
                            weak_app,
                        );
                        handle.window().hide().unwrap();
                    })
                    .unwrap();
                } else {
                    weak.upgrade_in_event_loop(move |handle| {
                        handle.set_pending_session(false);
                        if let Some(message) = message.message {
                            show_netplay_error(message);
                        }
                    })
                    .unwrap();
                }
            }
            Ok(Err(err)) => {
                panic!("netplay_read_receiver error: {err}");
            }
            Err(_) => {
                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_pending_session(false);
                    show_netplay_error("Server did not respond".to_string());
                })
                .unwrap();
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn setup_wait_window(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<NetplayMessage>,
    session_name: slint::SharedString,
    game_name: slint::SharedString,
    rom_path: slint::SharedString,
    player_name: slint::SharedString,
    port: i32,
    can_start: bool,
    game_settings: GameSettings,
    peer_addr: slint::SharedString,
    weak_app: slint::Weak<AppWindow>,
) {
    let local_player = player_name.clone();

    let mut socket_addr: std::net::SocketAddr = peer_addr.to_string().parse().unwrap();
    socket_addr.set_port(port as u16);
    let wait = NetplayWait::new().unwrap();
    wait.set_session_name(session_name);
    wait.set_game_name(game_name);
    wait.set_rom_path(rom_path);
    wait.set_port(port);
    wait.set_can_start(can_start);

    let sender = netplay_write_sender.clone();
    wait.on_send_chat_message(move |message| {
        let send_chat = NetplayMessage {
            message_type: "request_chat_message".to_string(),
            player_name: Some(player_name.to_string()),
            client_sha: None,
            netplay_version: None,
            player_names: None,
            rooms: None,
            emulator: None,
            accept: None,
            message: Some(message.into()),
            auth_time: None,
            auth: None,
            room: Some(NetplayRoom {
                room_name: None,
                password: None,
                game_name: None,
                md5: None,
                protected: None,
                port: Some(port),
                features: None,
                buffer_target: None,
            }),
        };
        sender.send(Some(send_chat)).unwrap();
    });

    let sender = netplay_write_sender.clone();
    wait.on_begin_game(move || {
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
                port: Some(port),
                features: None,
                buffer_target: None,
            }),
        };
        sender.send(Some(begin_game)).unwrap();
    });

    let sender = netplay_write_sender.clone();
    wait.window().on_close_requested(move || {
        let _ = sender.send(None); // close current websocket if any
        slint::CloseRequestResponse::HideWindow
    });

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

    netplay_write_sender.send(Some(motd_message)).unwrap();

    let weak = wait.as_weak();
    tokio::spawn(async move {
        loop {
            match netplay_read_receiver.recv().await {
                Ok(response) => match response.message_type.as_str() {
                    "reply_motd" => {
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
                                port: Some(port),
                                features: None,
                                buffer_target: None,
                            }),
                        };
                        netplay_write_sender.send(Some(request_players)).unwrap();

                        weak.upgrade_in_event_loop(move |handle| {
                            #[allow(clippy::regex_creation_in_loops)]
                            let re = regex::Regex::new(r"<[^>]*>").unwrap();
                            let motd = re
                                .replace_all(response.message.unwrap().as_str(), "")
                                .into_owned();
                            handle.set_motd(motd.into());
                        })
                        .unwrap();
                    }
                    "reply_players" => {
                        weak.upgrade_in_event_loop(move |handle| {
                            if let Some(player_names) = response.player_names {
                                let players_vec: slint::VecModel<slint::SharedString> =
                                    slint::VecModel::default();
                                for player in player_names {
                                    players_vec.push(player.into());
                                }
                                let players_model: std::rc::Rc<
                                    slint::VecModel<slint::SharedString>,
                                > = std::rc::Rc::new(players_vec);
                                handle.set_players(slint::ModelRc::from(players_model));
                            }
                        })
                        .unwrap();
                    }
                    "reply_chat_message" => {
                        weak.upgrade_in_event_loop(move |handle| {
                            let mut chat_text = handle.get_chat_text();
                            chat_text.push_str(&format!("{}\n", response.message.unwrap()));
                            handle.set_chat_text(chat_text);
                        })
                        .unwrap();
                    }
                    "reply_begin_game" => {
                        if response.accept.unwrap() == 0 {
                            weak.upgrade_in_event_loop(move |handle| {
                                handle.window().hide().unwrap();
                                let _ = netplay_write_sender.send(None);

                                let mut player_number = 4;
                                let players = handle.get_players();
                                for (i, player) in players.iter().enumerate() {
                                    if player == local_player {
                                        player_number = i;
                                    }
                                }
                                if player_number > 3 {
                                    panic!("Could not determine player number");
                                }

                                run_rom(
                                    GbPaths {
                                        rom: [None, None, None, None],
                                        ram: [None, None, None, None],
                                    },
                                    handle.get_rom_path().as_str().into(),
                                    GameSettings {
                                        fullscreen: game_settings.fullscreen,
                                        overclock: game_settings.overclock,
                                        disable_expansion_pak: game_settings.disable_expansion_pak,
                                        cheats: game_settings.cheats,
                                    },
                                    VruChannel {
                                        vru_window_notifier: None,
                                        vru_word_receiver: None,
                                    },
                                    Some(NetplayDevice {
                                        peer_addr: socket_addr,
                                        player_number: player_number as u8,
                                    }),
                                    weak_app,
                                );
                            })
                            .unwrap();
                            return;
                        } else {
                            weak.upgrade_in_event_loop(move |handle| {
                                handle.set_can_start(can_start);
                                if let Some(message) = response.message {
                                    show_netplay_error(message);
                                }
                            })
                            .unwrap();
                        }
                    }
                    _ => {
                        println!("Unknown netplay message type: {}", response.message_type);
                    }
                },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    panic!("netplay_read_receiver lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break; // exit the loop if the receiver is closed
                }
            }
        }
    });

    wait.show().unwrap();
}

pub fn setup_join_window(
    join_window: &NetplayJoin,
    fullscreen: bool,
    rom_dir: slint::SharedString,
    weak_app: slint::Weak<AppWindow>,
) {
    let (netplay_read_sender, netplay_read_receiver): (
        tokio::sync::broadcast::Sender<NetplayMessage>,
        tokio::sync::broadcast::Receiver<NetplayMessage>,
    ) = tokio::sync::broadcast::channel(5);

    let (netplay_write_sender, netplay_write_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    ) = tokio::sync::broadcast::channel(5);

    join_window.set_rom_dir(rom_dir);
    populate_server_names(join_window.as_weak());
    let weak = join_window.as_weak();
    join_window.on_get_ping(move |server_url| {
        update_ping(weak.clone(), server_url.to_string());
        weak.upgrade_in_event_loop(move |handle| {
            handle.invoke_refresh_session(server_url);
        })
        .unwrap();
    });
    let weak = join_window.as_weak();
    join_window.on_get_custom_url(move || {
        let weak2 = weak.clone();
        weak.upgrade_in_event_loop(move |handle| {
            handle.set_ping("Ping: Unknown".into());
            show_custom_url_dialog(weak2, handle.get_custom_server_url());
        })
        .unwrap();
    });
    let weak = join_window.as_weak();
    join_window.on_select_rom(move |rom_dir| {
        select_rom(weak.clone(), rom_dir);
    });

    let sender = netplay_write_sender.clone();
    join_window.window().on_close_requested(move || {
        let _ = sender.send(None); // close current websocket if any
        slint::CloseRequestResponse::HideWindow
    });

    let weak = join_window.as_weak();
    let sender = netplay_write_sender.clone();
    let receiver = netplay_read_receiver.resubscribe();
    join_window.on_refresh_session(move |server_url| {
        let _ = sender.send(None); // close current websocket if any
        manage_websocket(
            server_url.to_string(),
            netplay_read_sender.clone(),
            netplay_write_receiver.resubscribe(),
            weak.clone(),
        );
        update_sessions(sender.clone(), receiver.resubscribe(), weak.clone());
    });
    let weak = join_window.as_weak();
    join_window.on_join_session(move |player_name, game_hash, password, port| {
        join_session(
            netplay_write_sender.clone(),
            netplay_read_receiver.resubscribe(),
            player_name.to_string(),
            game_hash.to_string(),
            password.to_string(),
            port,
            fullscreen,
            weak_app.clone(),
            weak.clone(),
        );
    });

    join_window.show().unwrap();
}

pub fn netplay_window(app: &AppWindow, controller_paths: &[Option<String>]) {
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
                setup_create_window(
                    &create_window,
                    GameSettings {
                        fullscreen: handle.get_fullscreen(),
                        overclock: handle.get_overclock_n64_cpu(),
                        disable_expansion_pak: handle.get_disable_expansion_pak(),
                        cheats: std::collections::HashMap::new(), // not used here
                    },
                    handle.get_rom_dir(),
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
                setup_join_window(
                    &join_window,
                    handle.get_fullscreen(),
                    handle.get_rom_dir(),
                    weak_app,
                );
            })
            .unwrap();
    });

    app.on_netplay_discord_button_clicked(move || {
        open::that_detached("https://discord.gg/JyW6ZgBUyS").unwrap();
    });
    app.on_netplay_feedback_button_clicked(move || {
        open::that_detached("https://github.com/gopher64/gopher64/discussions/453").unwrap();
    });
}
