use crate::device;
use crate::ui;
use crate::ui::gui::{AppWindow, NetplayDevice, open_uri, run_rom, save_settings};
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

async fn get_local_servers() -> Vec<(String, String)> {
    if let Ok(broadcast_sock) =
        tokio::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).await
        && broadcast_sock.set_broadcast(true).is_ok()
    {
        let data: [u8; 1] = [1];
        if let Err(e) = broadcast_sock
            .send_to(&data, (std::net::Ipv4Addr::BROADCAST, 45000))
            .await
        {
            eprintln!("Error sending broadcast: {}", e);
            return vec![];
        }
        let mut buffer = [0; 1024];
        if let Ok(Ok(result)) = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            broadcast_sock.recv(&mut buffer),
        )
        .await
            && let Ok(data) = serde_json::from_slice::<std::collections::HashMap<String, String>>(
                &buffer[..result],
            )
        {
            let mut local_servers = vec![];
            for server in data.iter() {
                local_servers.push((server.0.into(), server.1.into()));
            }
            local_servers
        } else {
            vec![]
        }
    } else {
        eprintln!("Error creating netplay broadcast socket");
        vec![]
    }
}

fn populate_server_names(weak: slint::Weak<AppWindow>, refresh_sessions: bool) {
    let task = ui::WEB_CLIENT
        .get("https://dispatch.gopher64.com/getRegions")
        .header("netplay-id", env!("NETPLAY_ID"))
        .send();
    tokio::spawn(async move {
        let (response, local_servers) = tokio::join!(task, get_local_servers());
        let public_servers = if let Ok(response) = response
            && let Ok(servers) = response.json::<Vec<String>>().await
        {
            servers
        } else {
            vec![]
        };
        weak.upgrade_in_event_loop(move |handle| {
            let server_names: slint::VecModel<slint::SharedString> = slint::VecModel::default();
            let server_urls: slint::VecModel<slint::SharedString> = slint::VecModel::default();
            for local_server in local_servers {
                server_names.push(local_server.0.into());
                server_urls.push(local_server.1.into());
            }
            for server in public_servers {
                server_names.push(server.clone().into());
                server_urls.push(format!("dispatcher:{server}").into());
            }
            server_names.push("Custom".into());
            if refresh_sessions {
                handle.invoke_netplay_refresh_sessions();
            }
            handle.set_netplay_server_names(slint::ModelRc::from(std::rc::Rc::new(server_names)));
            handle.set_netplay_server_urls(slint::ModelRc::from(std::rc::Rc::new(server_urls)));
        })
        .unwrap();
    });
}

fn select_rom(weak: slint::Weak<AppWindow>, rom_dir: slint::SharedString) {
    let select_rom = ui::gui::select_rom(rom_dir);
    tokio::spawn(async move {
        if let Some(file) = select_rom.await {
            if let Some(rom_contents) = device::get_rom_contents(&file) {
                let hash = device::cart::rom::calculate_hash(&rom_contents);
                let mut game_name = ui::storage::get_game_name(&rom_contents);
                let game_crc = ui::storage::get_game_crc(&rom_contents);
                let cheats = ui::config::Cheats::new();
                let mut parsed_cheats = String::new();
                if let Some(game_cheats) = cheats.cheats.get(&game_crc)
                    && !game_cheats.is_empty()
                {
                    parsed_cheats = serde_json::to_string(game_cheats).unwrap();
                }
                if game_name.is_empty() {
                    game_name = file.file_name().unwrap().to_string_lossy().to_string();
                }

                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_netplay_game_name(game_name.into());
                    handle.set_netplay_game_hash(hash.into());
                    handle.set_netplay_game_cheats(parsed_cheats.into());
                    handle.set_netplay_rom_path(file.to_str().unwrap().into());
                })
                .unwrap();
            } else {
                weak.upgrade_in_event_loop(move |handle| {
                    handle.invoke_show_message("Could not read ROM".into(), true);

                    handle.set_netplay_game_name(String::new().into());
                    handle.set_netplay_game_hash(String::new().into());
                    handle.set_netplay_game_cheats(String::new().into());
                    handle.set_netplay_rom_path(String::new().into());
                })
                .unwrap();
            }
        }
    });
}

fn setup_create_window(
    app: &AppWindow,
    game_settings: ui::GameSettings,
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
) {
    populate_server_names(app.as_weak(), false);
    let weak = app.as_weak();
    app.on_netplay_get_custom_url(move || {
        weak.upgrade_in_event_loop(move |handle| {
            handle.invoke_show_custom_server_dialog();
        })
        .unwrap();
    });

    let weak = app.as_weak();
    app.on_netplay_create_session(
        move |server_url,
              session_name,
              player_name,
              game_name,
              game_hash,
              game_cheats,
              password| {
            let _ = netplay_write_sender.send(None); // close current websocket if any
            if server_url.starts_with("dispatcher:") {
                weak.upgrade_in_event_loop(move |handle| {
                    handle.invoke_show_message(
                        "Creating server, please wait...This may take about 30 seconds.".into(),
                        false,
                    );
                })
                .unwrap();
                let task = ui::WEB_CLIENT
                    .get("https://dispatch.gopher64.com/createServer")
                    .query(&[("region", server_url.strip_prefix("dispatcher:").unwrap())])
                    .header("netplay-id", env!("NETPLAY_ID"))
                    .send();
                let netplay_read_sender = netplay_read_sender.clone();
                let netplay_write_receiver = netplay_write_receiver.resubscribe();
                let netplay_write_sender = netplay_write_sender.clone();
                let netplay_read_receiver = netplay_read_receiver.resubscribe();
                let game_settings = game_settings.clone();
                let weak = weak.clone();
                let weak_app = weak.clone();
                tokio::spawn(async move {
                    let response = task.await;

                    if let Ok(response) = response
                        && let Ok(server) = response
                            .json::<std::collections::HashMap<String, String>>()
                            .await
                    {
                        let server_url = server.values().next().unwrap();

                        manage_websocket(
                            server_url.to_string(),
                            netplay_read_sender,
                            netplay_write_receiver,
                            weak.clone(),
                        );

                        create_session(
                            netplay_write_sender,
                            netplay_read_receiver,
                            session_name.to_string(),
                            player_name.to_string(),
                            game_name.to_string(),
                            game_hash.to_string(),
                            game_cheats.to_string(),
                            password.to_string(),
                            game_settings,
                            weak_app,
                        );
                    } else {
                        weak.upgrade_in_event_loop(|handle| {
                            handle.set_netplay_pending_session(false);
                            handle.invoke_show_message("Server could not be created".into(), true);
                        })
                        .unwrap();
                    }
                });
            } else {
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
                    weak.clone(),
                );
            }
        },
    );

    app.set_show_netplay_create_room(true);
}

fn manage_websocket(
    server_url: String,
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    weak: slint::Weak<AppWindow>,
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
                        handle.set_netplay_peer_addr(addr.into());
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

fn update_sessions(weak: slint::Weak<AppWindow>) {
    let task = ui::WEB_CLIENT
        .get("https://dispatch.gopher64.com/getServers")
        .header("netplay-id", env!("NETPLAY_ID"))
        .send();
    tokio::spawn(async move {
        let mut dispatcher_servers = std::collections::HashMap::new();
        let response = task.await;
        if let Ok(response) = response
            && let Ok(servers) = response
                .json::<std::collections::HashMap<String, String>>()
                .await
        {
            dispatcher_servers = servers;
        }
        let weak2 = weak.clone();
        weak.upgrade_in_event_loop(move |handle| {
            let mut servers: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            let server_names = handle.get_netplay_server_names();
            let server_urls = handle.get_netplay_server_urls();
            for (i, server_name) in server_names.iter().enumerate() {
                if server_name == "Custom" {
                    let custom_server_url = handle.get_netplay_custom_server_url();
                    if !custom_server_url.is_empty() {
                        servers.insert(
                            "Custom".to_string(),
                            "ws://".to_string() + &custom_server_url,
                        );
                    }
                } else if let Some(url) = server_urls.row_data(i)
                    && url.starts_with("dispatcher:")
                {
                    continue;
                } else {
                    servers.insert(
                        server_name.to_string(),
                        server_urls.row_data(i).unwrap().to_string(),
                    );
                }
            }
            servers.extend(dispatcher_servers);

            tokio::spawn(async move {
                let mut sessions = vec![];
                let mut room_urls = vec![];
                let mut room_ports = vec![];
                for (server_name, server_url) in servers.iter() {
                    if let Ok(Ok((socket, _response))) = tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        tokio_tungstenite::connect_async(server_url.clone()),
                    )
                    .await
                    {
                        let (mut write, mut read) = socket.split();

                        let now_utc = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                            .to_string();
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
                            auth: Some(
                                hasher
                                    .finalize()
                                    .iter()
                                    .map(|b| format!("{:02x}", b))
                                    .collect(),
                            ),
                            room: None,
                        };
                        write
                            .send(Message::Binary(Bytes::from(
                                serde_json::to_vec(&request_rooms).unwrap(),
                            )))
                            .await
                            .unwrap();

                        if let Some(Ok(response)) = read.next().await
                            && let Ok(message) =
                                serde_json::from_slice::<NetplayMessage>(&response.into_data())
                            && message.message_type == "reply_get_rooms"
                            && message.accept.unwrap() == 0
                            && let Some(rooms) = message.rooms
                        {
                            for room in rooms {
                                let mut session = vec![];
                                room_urls.push(server_url.into());
                                room_ports.push(room.port.unwrap());

                                session.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(server_name),
                                ));
                                session.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(room.room_name.unwrap()),
                                ));
                                session.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(room.game_name.unwrap()),
                                ));
                                session.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(if room.protected.unwrap() {
                                        "True"
                                    } else {
                                        "False"
                                    }),
                                ));
                                session.push(slint::StandardListViewItem::from(
                                    slint::SharedString::from(
                                        if room.features.unwrap_or_default().contains_key("cheats")
                                        {
                                            "True"
                                        } else {
                                            "False"
                                        },
                                    ),
                                ));
                                sessions.push(session);
                            }
                        }
                    }
                }
                weak2
                    .upgrade_in_event_loop(move |handle| {
                        let sessions_vec = slint::VecModel::default();
                        for session in sessions.iter() {
                            sessions_vec.push(slint::ModelRc::from(std::rc::Rc::new(
                                slint::VecModel::from(session.to_vec()),
                            )));
                        }
                        handle.set_netplay_sessions(slint::ModelRc::from(std::rc::Rc::new(
                            sessions_vec,
                        )));

                        handle.set_netplay_room_urls(slint::ModelRc::from(std::rc::Rc::new(
                            slint::VecModel::from(room_urls.to_vec()),
                        )));

                        handle.set_netplay_room_ports(slint::ModelRc::from(std::rc::Rc::new(
                            slint::VecModel::from(room_ports.to_vec()),
                        )));

                        handle.set_netplay_current_session(-1);
                        handle.set_netplay_pending_refresh(false);
                    })
                    .unwrap();
            });
        })
        .unwrap();
    });
}

#[allow(clippy::too_many_arguments)]
fn create_session(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    session_name: String,
    player_name: String,
    game_name: String,
    game_hash: String,
    game_cheats: String,
    password: String,
    game_settings: ui::GameSettings,
    weak_app: slint::Weak<AppWindow>,
) {
    tokio::spawn(async move {
        let now_utc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
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
            client_sha: Some(env!("GIT_DESCRIBE").to_string()),
            netplay_version: Some(NETPLAY_VERSION),
            emulator: Some(EMU_NAME.to_string()),
            accept: None,
            message: None,
            rooms: None,
            auth_time: Some(now_utc),
            player_names: None,
            auth: Some(
                hasher
                    .finalize()
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect(),
            ),
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
            Ok(Ok(Some(message))) => {
                if message.message_type == "reply_create_room" && message.accept.unwrap() == 0 {
                    weak_app
                        .upgrade_in_event_loop(move |handle| {
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
                                handle.get_netplay_rom_path(),
                                message.player_name.as_ref().unwrap().into(),
                                session.port.unwrap(),
                                ui::GameSettings {
                                    overclock: overclock.parse().unwrap(),
                                    disable_expansion_pak: disable_expansion_pak.parse().unwrap(),
                                    cheats: serde_json::from_str(cheats).unwrap(),
                                    load_savestate_slot: None,
                                },
                                handle.get_netplay_peer_addr(),
                                &handle,
                            );
                        })
                        .unwrap();
                } else {
                    weak_app
                        .upgrade_in_event_loop(move |handle| {
                            handle.set_netplay_pending_session(false);
                            if let Some(message) = message.message {
                                handle.invoke_show_message(message.into(), true);
                            }
                        })
                        .unwrap();
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(err)) => {
                panic!("netplay_read_receiver error: {err}");
            }
            Err(_) => {
                weak_app
                    .upgrade_in_event_loop(move |handle| {
                        handle.set_netplay_pending_session(false);
                        handle.invoke_show_message("Server did not respond".into(), true);
                    })
                    .unwrap();
            }
        }
    });
}

fn join_session(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    player_name: String,
    game_hash: String,
    password: String,
    room_port: i32,
    weak_app: slint::Weak<AppWindow>,
) {
    tokio::spawn(async move {
        let now_utc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        let hasher = Sha256::new().chain_update(&now_utc).chain_update(EMU_NAME);

        let join_room = NetplayMessage {
            message_type: "request_join_room".to_string(),
            player_name: Some(player_name),
            client_sha: Some(env!("GIT_DESCRIBE").to_string()),
            netplay_version: Some(NETPLAY_VERSION),
            emulator: Some(EMU_NAME.to_string()),
            accept: None,
            message: None,
            rooms: None,
            player_names: None,
            auth_time: Some(now_utc),
            auth: Some(
                hasher
                    .finalize()
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect(),
            ),
            room: Some(NetplayRoom {
                room_name: None,
                password: Some(password),
                game_name: None,
                md5: Some(game_hash),
                protected: None,
                port: Some(room_port),
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
            Ok(Ok(Some(message))) => {
                if message.message_type == "reply_join_room" && message.accept.unwrap() == 0 {
                    weak_app
                        .upgrade_in_event_loop(move |handle| {
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
                                handle.get_netplay_rom_path(),
                                message.player_name.as_ref().unwrap().into(),
                                session.port.unwrap(),
                                ui::GameSettings {
                                    overclock: overclock.parse().unwrap(),
                                    disable_expansion_pak: disable_expansion_pak.parse().unwrap(),
                                    cheats: serde_json::from_str(cheats).unwrap(),
                                    load_savestate_slot: None,
                                },
                                handle.get_netplay_peer_addr(),
                                &handle,
                            );
                        })
                        .unwrap();
                } else {
                    weak_app
                        .upgrade_in_event_loop(move |handle| {
                            handle.set_netplay_pending_session(false);
                            if let Some(message) = message.message {
                                handle.invoke_show_message(message.into(), true);
                            }
                        })
                        .unwrap();
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(err)) => {
                panic!("netplay_read_receiver error: {err}");
            }
            Err(_) => {
                weak_app
                    .upgrade_in_event_loop(move |handle| {
                        handle.set_netplay_pending_session(false);
                        handle.invoke_show_message("Server did not respond".into(), true);
                    })
                    .unwrap();
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn setup_wait_window(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    session_name: slint::SharedString,
    game_name: slint::SharedString,
    rom_path: slint::SharedString,
    player_name: slint::SharedString,
    port: i32,
    game_settings: ui::GameSettings,
    peer_addr: slint::SharedString,
    app: &AppWindow,
) {
    let local_player = player_name.clone();

    let mut socket_addr: std::net::SocketAddr = peer_addr.to_string().parse().unwrap();
    socket_addr.set_port(port as u16);
    app.set_netplay_session_name(session_name);
    app.set_netplay_game_name(game_name);
    app.set_netplay_rom_path(rom_path);
    app.set_netplay_port(port);
    app.set_netplay_ping("Unknown".into());

    let sender = netplay_write_sender.clone();
    app.on_netplay_send_chat_message(move |message| {
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
    app.on_netplay_begin_game(move || {
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

    let ping_message = NetplayMessage {
        message_type: "request_ping".to_string(),
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

    netplay_write_sender
        .send(Some(ping_message.clone()))
        .unwrap();

    let weak_app = app.as_weak();
    tokio::spawn(async move {
        loop {
            match netplay_read_receiver.recv().await {
                Ok(Some(response)) => match response.message_type.as_str() {
                    "reply_ping" => {
                        if let Some(message) = response.message {
                            weak_app
                                .upgrade_in_event_loop(move |handle| {
                                    handle.set_netplay_ping((message + " ms").into());
                                })
                                .unwrap();
                        }
                        let ping_message = ping_message.clone();
                        let ping_writer = netplay_write_sender.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            let _ = ping_writer.send(Some(ping_message));
                        });
                    }
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

                        weak_app
                            .upgrade_in_event_loop(move |handle| {
                                #[allow(clippy::regex_creation_in_loops)]
                                let re = regex::Regex::new(r"<[^>]*>").unwrap();
                                let motd = re
                                    .replace_all(response.message.unwrap().as_str(), "")
                                    .into_owned();
                                handle.set_netplay_motd(motd.into());
                            })
                            .unwrap();
                    }
                    "reply_players" => {
                        let local_player = local_player.clone();
                        weak_app
                            .upgrade_in_event_loop(move |handle| {
                                if let Some(player_names) = response.player_names {
                                    if local_player == player_names[0] {
                                        handle.set_netplay_can_start(true);
                                    } else {
                                        handle.set_netplay_can_start(false);
                                    }

                                    handle.set_netplay_players(slint::ModelRc::from(
                                        std::rc::Rc::new(slint::VecModel::from(
                                            player_names
                                                .iter()
                                                .map(|x| x.into())
                                                .collect::<Vec<slint::SharedString>>(),
                                        )),
                                    ));
                                }
                            })
                            .unwrap();
                    }
                    "reply_chat_message" => {
                        weak_app
                            .upgrade_in_event_loop(move |handle| {
                                let mut chat_text = handle.get_netplay_chat_text();
                                chat_text.push_str(&format!("{}\n", response.message.unwrap()));
                                handle.set_netplay_chat_text(chat_text);
                            })
                            .unwrap();
                    }
                    "reply_begin_game" => {
                        if response.accept.unwrap() == 0 {
                            let weak_app2 = weak_app.clone();
                            weak_app
                                .upgrade_in_event_loop(move |handle| {
                                    let _ = netplay_write_sender.send(None);

                                    let mut player_number = 4;
                                    let players = handle.get_netplay_players();
                                    for (i, player) in players.iter().enumerate() {
                                        if player == local_player {
                                            player_number = i;
                                        }
                                    }
                                    if player_number > 3 {
                                        panic!("Could not determine player number");
                                    }

                                    run_rom(
                                        handle.get_netplay_rom_path().as_str().into(),
                                        ui::GameSettings {
                                            overclock: game_settings.overclock,
                                            disable_expansion_pak: game_settings
                                                .disable_expansion_pak,
                                            cheats: game_settings.cheats,
                                            load_savestate_slot: None,
                                        },
                                        Some(NetplayDevice {
                                            peer_addr: socket_addr,
                                            player_number: player_number as u8,
                                        }),
                                        weak_app2,
                                    );
                                    handle.invoke_netplay_close();
                                })
                                .unwrap();
                            return;
                        } else {
                            let local_player = local_player.clone();
                            weak_app
                                .upgrade_in_event_loop(move |handle| {
                                    if let Some(p0) = handle.get_netplay_players().row_data(0) {
                                        if p0 == local_player {
                                            handle.set_netplay_can_start(true);
                                        } else {
                                            handle.set_netplay_can_start(false);
                                        }
                                    }
                                    if let Some(message) = response.message {
                                        handle.invoke_show_message(message.into(), true);
                                    }
                                })
                                .unwrap();
                        }
                    }
                    _ => {
                        eprintln!("Unknown netplay message type: {}", response.message_type);
                    }
                },
                Ok(None) => {
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    panic!("netplay_read_receiver lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break; // exit the loop if the receiver is closed
                }
            }
        }
    });
    app.set_show_netplay_wait_room(true);
    app.set_show_netplay_create_room(false);
    app.set_show_netplay_join_room(false);
    app.invoke_close_message();
}

fn setup_join_window(
    app: &AppWindow,
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
) {
    app.set_netplay_pending_refresh(true);
    populate_server_names(app.as_weak(), true);

    let weak = app.as_weak();
    app.on_netplay_refresh_sessions(move || {
        update_sessions(weak.clone());
    });
    let weak = app.as_weak();
    app.on_netplay_join_session(
        move |player_name, game_hash, password, room_url, room_port| {
            let _ = netplay_write_sender.send(None); // close current websocket if any
            manage_websocket(
                room_url.to_string(),
                netplay_read_sender.clone(),
                netplay_write_receiver.resubscribe(),
                weak.clone(),
            );

            join_session(
                netplay_write_sender.clone(),
                netplay_read_receiver.resubscribe(),
                player_name.to_string(),
                game_hash.to_string(),
                password.to_string(),
                room_port,
                weak.clone(),
            );
        },
    );

    app.set_show_netplay_join_room(true);
}

pub fn netplay_window(app: &AppWindow) {
    let (netplay_read_sender, netplay_read_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    ) = tokio::sync::broadcast::channel(5);

    let (netplay_write_sender, netplay_write_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    ) = tokio::sync::broadcast::channel(5);

    let weak_app = app.as_weak();
    let write_sender_create = netplay_write_sender.clone();
    let read_receiver_create = netplay_read_receiver.resubscribe();
    let read_sender_create = netplay_read_sender.clone();
    let write_receiver_create = netplay_write_receiver.resubscribe();
    app.on_create_session_button_clicked(move || {
        let write_sender = write_sender_create.clone();
        let read_receiver = read_receiver_create.resubscribe();
        let read_sender = read_sender_create.clone();
        let write_receiver = write_receiver_create.resubscribe();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                save_settings(&handle);
                setup_create_window(
                    &handle,
                    ui::GameSettings {
                        overclock: handle.get_overclock_n64_cpu(),
                        disable_expansion_pak: handle.get_disable_expansion_pak(),
                        cheats: std::collections::HashMap::new(), // not used here
                        load_savestate_slot: None,
                    },
                    write_sender,
                    read_receiver,
                    read_sender,
                    write_receiver,
                );
            })
            .unwrap();
    });

    let weak_app = app.as_weak();
    let write_sender_join = netplay_write_sender.clone();
    let read_sender_join = netplay_read_sender.clone();
    app.on_join_session_button_clicked(move || {
        let write_sender = write_sender_join.clone();
        let read_receiver = netplay_read_receiver.resubscribe();
        let read_sender = read_sender_join.clone();
        let write_receiver = netplay_write_receiver.resubscribe();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                save_settings(&handle);
                setup_join_window(
                    &handle,
                    write_sender,
                    read_receiver,
                    read_sender,
                    write_receiver,
                );
            })
            .unwrap();
    });

    let weak_app = app.as_weak();
    app.on_netplay_select_rom(move |rom_dir| {
        select_rom(weak_app.clone(), rom_dir);
    });

    let weak_app = app.as_weak();
    let write_sender = netplay_write_sender.clone();
    let read_sender = netplay_read_sender.clone();
    app.on_netplay_close(move || {
        weak_app
            .upgrade_in_event_loop(move |handle| {
                handle.set_show_netplay_wait_room(false);
                handle.set_show_netplay_create_room(false);
                handle.set_show_netplay_join_room(false);
                handle.invoke_clear_netplay_data();
                handle.invoke_close_message();
            })
            .unwrap();
        let _ = write_sender.send(None); // close current websocket if any
        let _ = read_sender.send(None); // close current receiver if any
    });

    app.on_netplay_discord_button_clicked(move || {
        open_uri("https://discord.gg/JyW6ZgBUyS");
    });
    app.on_netplay_feedback_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64/discussions/453");
    });
}
