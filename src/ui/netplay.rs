use crate::device;
use crate::netplay::RtcIceServerConfig;
use crate::ui;
use crate::ui::gui::{AppWindow, open_uri, run_rom, save_settings};
use futures::{SinkExt, StreamExt};
use slint::ComponentHandle;
use slint::Model;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum MessageType {
    RequestCreateSession,
    RequestJoinSession,
    RequestListSessions,
    RequestUpdateSession,
    RequestBeginGame,
    ResponseBeginGame,
    ResponseSession,
    ResponseListSessions,
    SendChatMessage,
    ReceiveChatMessage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerAddresses {
    pub lobby: String,
    pub game: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NetplaySession {
    password: Option<String>,
    game_name: Option<String>,
    motd: Option<String>,
    game_checksum: Option<String>,
    client_version: Option<String>,
    features: Option<std::collections::HashMap<String, String>>,
    players: Vec<String>,
    server_address: Option<ServerAddresses>,
    input_delay: Option<usize>,
    ice_server_config: Option<RtcIceServerConfig>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NetplayLobbyMessage {
    message_type: MessageType,
    sessions: std::collections::HashMap<String, NetplaySession>,
    message: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum PingType {
    Ping,
    Pong,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NetplayPingMessage {
    message_type: PingType,
    timestamp: u128,
    num_of_peers: usize,
}

fn select_rom(weak: slint::Weak<AppWindow>, rom_dir: slint::SharedString) {
    let select_rom = ui::gui::select_rom(rom_dir);
    tokio::spawn(async move {
        if let Some(file) = select_rom.await {
            if let Some(rom_contents) = device::get_rom_contents(&file) {
                let hash = device::cart::rom::calculate_hash(&rom_contents);
                let mut game_name = ui::storage::get_game_name(&rom_contents);
                let pal = device::cart::rom::is_system_pal(&rom_contents);
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
                    handle.set_netplay_game_pal(pal);
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

fn setup_callbacks(
    app: &AppWindow,
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    close_ping_tx: tokio::sync::broadcast::Sender<()>,
    close_ping_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let weak = app.as_weak();
    let write_sender_create_session = netplay_write_sender.clone();
    let netplay_read_receiver_create_session = netplay_read_receiver.resubscribe();
    let netplay_write_receiver_create_session = netplay_write_receiver.resubscribe();
    let netplay_read_sender_create_session = netplay_read_sender.clone();
    let close_ping_rx_create_session = close_ping_rx.resubscribe();
    app.on_netplay_create_session(
        move |session_name,
              player_name,
              game_name,
              game_hash,
              game_cheats,
              password,
              overclock,
              disable_expansion_pak| {
            manage_websocket(
                netplay_read_sender_create_session.clone(),
                netplay_write_receiver_create_session.resubscribe(),
            );

            create_session(
                write_sender_create_session.clone(),
                netplay_read_receiver_create_session.resubscribe(),
                close_ping_rx_create_session.resubscribe(),
                session_name.to_string(),
                player_name.to_string(),
                game_name.to_string(),
                game_hash.to_string(),
                game_cheats.to_string(),
                password.to_string(),
                overclock,
                disable_expansion_pak,
                weak.clone(),
            );
        },
    );

    let write_sender_chat_message = netplay_write_sender.clone();
    app.on_netplay_send_chat_message(move |message| {
        let send_chat = NetplayLobbyMessage {
            message_type: MessageType::SendChatMessage,
            sessions: std::collections::HashMap::new(),
            message: Some(message.into()),
        };
        write_sender_chat_message.send(Some(send_chat)).unwrap();
    });

    let write_sender_begin_game = netplay_write_sender.clone();
    app.on_netplay_begin_game(move |chosen_input_delay| {
        let begin_game = NetplayLobbyMessage {
            message_type: MessageType::RequestBeginGame,
            sessions: std::collections::HashMap::new(),
            message: Some(chosen_input_delay.to_string()),
        };
        write_sender_begin_game.send(Some(begin_game)).unwrap();
    });

    let write_sender_refresh_sessions = netplay_write_sender.clone();
    app.on_netplay_refresh_sessions(move || {
        update_sessions(write_sender_refresh_sessions.clone());
    });

    let write_sender_join_session = netplay_write_sender.clone();
    app.on_netplay_join_session(move |session_name, player_name, game_hash, password| {
        join_session(
            write_sender_join_session.clone(),
            session_name.to_string(),
            player_name.to_string(),
            game_hash.to_string(),
            password.to_string(),
        );
    });

    let weak_app = app.as_weak();
    app.on_create_session_button_clicked(move || {
        weak_app
            .upgrade_in_event_loop(move |handle| {
                save_settings(&handle);
                setup_create_window(&handle);
            })
            .unwrap();
    });

    let weak_app = app.as_weak();
    let netplay_write_sender_join_session_button = netplay_write_sender.clone();
    let netplay_read_receiver_join_session_button = netplay_read_receiver.resubscribe();
    let netplay_read_sender_join_session_button = netplay_read_sender.clone();
    let netplay_write_receiver_join_session_button = netplay_write_receiver.resubscribe();
    let close_ping_rx_join_session_button = close_ping_rx.resubscribe();
    app.on_join_session_button_clicked(move || {
        let netplay_write_sender = netplay_write_sender_join_session_button.clone();
        let netplay_read_receiver = netplay_read_receiver_join_session_button.resubscribe();
        let netplay_read_sender = netplay_read_sender_join_session_button.clone();
        let netplay_write_receiver = netplay_write_receiver_join_session_button.resubscribe();
        let close_ping_rx = close_ping_rx_join_session_button.resubscribe();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                save_settings(&handle);
                setup_join_window(
                    &handle,
                    netplay_write_sender,
                    netplay_read_receiver,
                    netplay_read_sender,
                    netplay_write_receiver,
                    close_ping_rx,
                );
            })
            .unwrap();
    });

    let weak_app = app.as_weak();
    app.on_netplay_select_rom(move |rom_dir| {
        select_rom(weak_app.clone(), rom_dir);
    });

    let weak_app = app.as_weak();
    app.on_netplay_close(move || {
        weak_app
            .upgrade_in_event_loop(move |handle| {
                handle.set_show_netplay_wait_session(false);
                handle.set_show_netplay_create_session(false);
                handle.set_show_netplay_join_session(false);
                handle.invoke_clear_netplay_data();
                handle.invoke_close_message();
            })
            .unwrap();
        let _ = netplay_write_sender.send(None); // close current websocket if any
        let _ = netplay_read_sender.send(None); // close current receiver if any
        let _ = close_ping_tx.send(()); // close ping
    });

    app.on_netplay_discord_button_clicked(move || {
        open_uri("https://discord.gg/JyW6ZgBUyS");
    });
    app.on_netplay_feedback_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64/discussions/453");
    });
}

fn setup_create_window(app: &AppWindow) {
    app.set_show_netplay_create_session(true);
}

fn manage_websocket(
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    mut netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
) {
    let mut request = std::env::var("NETPLAY_SERVER_URL")
        .unwrap_or("wss://netplay.gopher64.com".to_string())
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("Authorization", env!("NETPLAY_ID").parse().unwrap());
    tokio::spawn(async move {
        match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio_tungstenite::connect_async(request),
        )
        .await
        {
            Ok(Ok((socket, _response))) => {
                let (mut write, mut read) = socket.split();

                tokio::spawn(async move {
                    while let Some(Ok(response)) = read.next().await {
                        match response {
                            Message::Binary(data) => {
                                let decoded_response =
                                    postcard::from_bytes::<NetplayLobbyMessage>(&data);
                                match decoded_response {
                                    Ok(message) => {
                                        let _ = netplay_read_sender.send(Some(message));
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to parse message: {}", e);
                                    }
                                }
                            }
                            Message::Close(_) => {
                                return;
                            }
                            _ => {}
                        }
                    }
                });
                tokio::spawn(async move {
                    loop {
                        match netplay_write_receiver.recv().await {
                            Ok(Some(response)) => {
                                if let Err(e) = write
                                    .send(Message::Binary(Bytes::from(
                                        postcard::to_stdvec(&response).unwrap(),
                                    )))
                                    .await
                                {
                                    eprintln!("Failed to send message: {}", e);
                                }
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
                    if let Err(e) = write
                        .send(Message::Close(Some(CloseFrame {
                            code: CloseCode::Normal,
                            reason: Utf8Bytes::from(""),
                        })))
                        .await
                    {
                        eprintln!("Failed to send close message: {}", e);
                    }
                });
            }
            Ok(Err(e)) => {
                eprintln!("Failed to connect to netplay: {}", e);
            }
            Err(e) => {
                eprintln!("Failed to connect to netplay: {}", e);
            }
        }
    });
}

fn update_sessions(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
) {
    let request_sessions = NetplayLobbyMessage {
        message_type: MessageType::RequestListSessions,
        sessions: std::collections::HashMap::new(),
        message: None,
    };

    netplay_write_sender.send(Some(request_sessions)).unwrap();
}

#[allow(clippy::too_many_arguments)]
fn create_session(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    close_ping_rx: tokio::sync::broadcast::Receiver<()>,
    session_name: String,
    player_name: String,
    game_name: String,
    game_hash: String,
    game_cheats: String,
    password: String,
    overclock: bool,
    disable_expansion_pak: bool,
    weak_app: slint::Weak<AppWindow>,
) {
    tokio::spawn(async move {
        let mut features = std::collections::HashMap::default();

        if !game_cheats.is_empty() {
            features.insert("cheats".to_string(), game_cheats);
        }

        features.insert("overclock".to_string(), overclock.to_string());
        features.insert(
            "disable_expansion_pak".to_string(),
            disable_expansion_pak.to_string(),
        );

        let session = NetplaySession {
            password: if password.is_empty() {
                None
            } else {
                Some(password)
            },
            game_name: Some(game_name),
            motd: None,
            game_checksum: Some(game_hash),
            client_version: Some(env!("GIT_DESCRIBE").to_string()),
            features: Some(features),
            players: vec![player_name],
            server_address: None,
            input_delay: None,
            ice_server_config: None,
        };
        let create_session = NetplayLobbyMessage {
            message_type: MessageType::RequestCreateSession,
            sessions: std::collections::HashMap::from([(session_name, session)]),
            message: None,
        };

        netplay_write_sender.send(Some(create_session)).unwrap();

        match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            netplay_read_receiver.recv(),
        )
        .await
        {
            Ok(Ok(Some(message))) => {
                if message.message_type == MessageType::ResponseSession && message.message.is_none()
                {
                    weak_app
                        .upgrade_in_event_loop(move |handle| {
                            let (session_name, session) = message.sessions.iter().next().unwrap();
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
                                close_ping_rx,
                                session.server_address.as_ref().unwrap().clone(),
                                session_name.into(),
                                session.ice_server_config.clone(),
                                session.game_name.as_ref().unwrap().into(),
                                handle.get_netplay_rom_path(),
                                ui::GameSettings {
                                    overclock: overclock.parse().unwrap(),
                                    disable_expansion_pak: disable_expansion_pak.parse().unwrap(),
                                    cheats: serde_json::from_str(cheats).unwrap(),
                                    load_savestate_slot: None,
                                },
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
            Err(e) => {
                weak_app
                    .upgrade_in_event_loop(move |handle| {
                        handle.set_netplay_pending_session(false);
                        handle.invoke_show_message(
                            format!("Server did not respond: {e}").into(),
                            true,
                        );
                    })
                    .unwrap();
            }
        }
    });
}

fn join_session(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    session_name: String,
    player_name: String,
    game_hash: String,
    password: String,
) {
    let session = NetplaySession {
        password: if password.is_empty() {
            None
        } else {
            Some(password)
        },
        game_name: None,
        motd: None,
        game_checksum: Some(game_hash),
        client_version: Some(env!("GIT_DESCRIBE").to_string()),
        features: None,
        players: vec![player_name],
        server_address: None,
        input_delay: None,
        ice_server_config: None,
    };
    let join_session = NetplayLobbyMessage {
        message_type: MessageType::RequestJoinSession,
        sessions: std::collections::HashMap::from([(session_name, session)]),
        message: None,
    };

    netplay_write_sender.send(Some(join_session)).unwrap();
}

fn update_ping(
    server_addr: String,
    ice_server_config: Option<RtcIceServerConfig>,
    mut close_ping_rx: tokio::sync::broadcast::Receiver<()>,
    weak_app: slint::Weak<AppWindow>,
) {
    let mut builder =
        matchbox_socket::WebRtcSocketBuilder::new(server_addr).add_unreliable_channel();
    if let Some(ice_server_config) = ice_server_config {
        builder = builder.ice_server(matchbox_socket::RtcIceServerConfig {
            urls: ice_server_config.urls,
            username: ice_server_config.username,
            credential: ice_server_config.credential,
        });
    } else {
        eprintln!("Using default ICE config");
    }
    let (mut socket, loop_fut) = builder.build();

    tokio::spawn(async move {
        if let Err(e) = loop_fut.await {
            eprintln!("WebRTC loop failed: {}", e);
        }
    });
    let channel = socket.take_channel(0).unwrap();
    let (mut write, mut read) = channel.split();
    let mut write_clone = write.clone();
    let mut close_ping_rx_clone = close_ping_rx.resubscribe();
    let weak_app_clone = weak_app.clone();
    tokio::spawn(async move {
        loop {
            socket.update_peers();
            let peers_count = socket.connected_peers().count();
            weak_app_clone
                .upgrade_in_event_loop(move |handle| {
                    handle.set_netplay_connected_peers_count(peers_count as i32);
                })
                .unwrap();
            for peer in socket.connected_peers() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let ping_message = NetplayPingMessage {
                    message_type: PingType::Ping,
                    timestamp: now,
                    num_of_peers: peers_count,
                };
                let data = postcard::to_stdvec(&ping_message).unwrap();
                let _ = write.send((peer, data.into())).await;
            }
            tokio::select! {
                result = close_ping_rx_clone.recv() => {
                    match result {
                        Ok(()) => {
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            panic!("close_ping_rx lagged");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
        }
    });
    tokio::spawn(async move {
        let mut pings = vec![];
        loop {
            tokio::select! {
                result = close_ping_rx.recv() => {
                    match result {
                        Ok(()) => {
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            panic!("close_ping_rx lagged");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                Some((peer, data)) = read.next() => {
                    let decoded_message = postcard::from_bytes::<NetplayPingMessage>(&data);
                    match decoded_message {
                        Ok(message) => match message.message_type {
                            PingType::Ping => {
                                let pong_message = NetplayPingMessage {
                                    message_type: PingType::Pong,
                                    timestamp: message.timestamp,
                                    num_of_peers: message.num_of_peers,
                                };
                                let data = postcard::to_stdvec(&pong_message).unwrap();
                                let _ = write_clone.send((peer, data.into())).await;
                            }
                            PingType::Pong => {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis();
                                let ping = (now - message.timestamp) as f64 / 2.0; // calculate one-way latency
                                pings.push(ping);
                                if pings.len() > message.num_of_peers * 4 {
                                    // once we have enough samples, average the 3 highest values
                                    pings.sort_by(f64::total_cmp);
                                    let ping_avg = pings.iter().rev().take(3).sum::<f64>() / 3.0;
                                    pings.clear();
                                    weak_app
                                        .upgrade_in_event_loop(move |handle| {
                                            let refresh_rate = if handle.get_netplay_game_pal() {
                                                50.0
                                            } else {
                                                60.0
                                            };
                                            let latency_frames = (ping_avg / (1000.0 / refresh_rate)).ceil() as i32;
                                            let recommendation =
                                                (latency_frames + 1).min(16);

                                            if handle.get_netplay_recommended_delay() == 0 {
                                                handle.set_netplay_input_delay(recommendation);
                                            }
                                            handle.set_netplay_recommended_delay(recommendation);
                                        })
                                        .unwrap();
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to parse message: {}", e);
                        }
                    }
                }
            }
        }
    });
}

fn update_ice_config(ice_server_config: Option<RtcIceServerConfig>) {
    tokio::spawn(async move {
        let ice_config_path = ui::get_dirs().cache_dir.join("ice_config.json");
        if let Some(ice_server_config) = ice_server_config {
            tokio::fs::write(
                &ice_config_path,
                serde_json::to_vec(&ice_server_config).unwrap(),
            )
            .await
            .unwrap();
        } else {
            let _ = tokio::fs::remove_file(&ice_config_path).await;
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn setup_wait_window(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    close_ping_rx: tokio::sync::broadcast::Receiver<()>,
    server_addresses: ServerAddresses,
    session_name: slint::SharedString,
    ice_server_config: Option<RtcIceServerConfig>,
    game_name: slint::SharedString,
    rom_path: slint::SharedString,
    game_settings: ui::GameSettings,
    app: &AppWindow,
) {
    update_ice_config(ice_server_config.clone());
    update_ping(
        server_addresses.lobby.clone(),
        ice_server_config,
        close_ping_rx,
        app.as_weak(),
    );

    app.set_netplay_session_name(session_name);
    app.set_netplay_game_name(game_name);
    app.set_netplay_rom_path(rom_path);
    app.set_netplay_can_start(true);

    let request_update = NetplayLobbyMessage {
        message_type: MessageType::RequestUpdateSession,
        sessions: std::collections::HashMap::new(),
        message: None,
    };
    netplay_write_sender.send(Some(request_update)).unwrap();

    let weak_app = app.as_weak();
    tokio::spawn(async move {
        loop {
            match netplay_read_receiver.recv().await {
                Ok(Some(response)) => match response.message_type {
                    MessageType::ResponseSession => {
                        weak_app
                            .upgrade_in_event_loop(move |handle| {
                                let session = response.sessions.iter().next().unwrap().1;
                                let player_names = session.players.clone();

                                handle.set_netplay_motd(session.motd.as_ref().unwrap().into());

                                handle.set_netplay_players(slint::ModelRc::from(std::rc::Rc::new(
                                    slint::VecModel::from(
                                        player_names
                                            .iter()
                                            .map(|x| x.into())
                                            .collect::<Vec<slint::SharedString>>(),
                                    ),
                                )));
                            })
                            .unwrap();
                    }
                    MessageType::ReceiveChatMessage => {
                        weak_app
                            .upgrade_in_event_loop(move |handle| {
                                let mut chat_text = handle.get_netplay_chat_text();
                                chat_text.push_str(&format!("{}\n", response.message.unwrap()));
                                handle.set_netplay_chat_text(chat_text);
                            })
                            .unwrap();
                    }
                    MessageType::ResponseBeginGame => {
                        if response.message.is_none() {
                            let weak_app2 = weak_app.clone();
                            weak_app
                                .upgrade_in_event_loop(move |handle| {
                                    let player_name = handle.get_netplay_player_name();
                                    let players = handle.get_netplay_players();
                                    let input_delay = response
                                        .sessions
                                        .iter()
                                        .next()
                                        .unwrap()
                                        .1
                                        .input_delay
                                        .unwrap();
                                    let player_number =
                                        players.iter().position(|x| x == player_name).unwrap();
                                    run_rom(
                                        handle.get_netplay_rom_path().as_str().into(),
                                        ui::GameSettings {
                                            overclock: game_settings.overclock,
                                            disable_expansion_pak: game_settings
                                                .disable_expansion_pak,
                                            cheats: game_settings.cheats,
                                            load_savestate_slot: None,
                                        },
                                        Some(ui::gui::NetplayDevice {
                                            server_addr: server_addresses.game.clone(),
                                            player_number,
                                            number_of_players: players.row_count(),
                                            input_delay,
                                        }),
                                        weak_app2,
                                    );
                                    handle.invoke_netplay_close();
                                })
                                .unwrap();
                            return;
                        } else {
                            weak_app
                                .upgrade_in_event_loop(move |handle| {
                                    handle.set_netplay_can_start(true);
                                    if let Some(message) = response.message {
                                        handle.invoke_show_message(message.into(), true);
                                    }
                                })
                                .unwrap();
                        }
                    }
                    _ => {
                        eprintln!("Unknown netplay message type: {:?}", response.message_type);
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
    app.set_show_netplay_wait_session(true);
    app.set_show_netplay_create_session(false);
    app.set_show_netplay_join_session(false);
    app.invoke_close_message();
}

fn setup_join_window(
    app: &AppWindow,
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
    netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    close_ping_rx: tokio::sync::broadcast::Receiver<()>,
) {
    manage_websocket(
        netplay_read_sender.clone(),
        netplay_write_receiver.resubscribe(),
    );

    app.set_netplay_pending_refresh(true);

    let weak = app.as_weak();
    let mut read_receiver = netplay_read_receiver.resubscribe();
    let write_sender = netplay_write_sender.clone();
    tokio::spawn(async move {
        loop {
            match read_receiver.recv().await {
                Ok(Some(message)) => {
                    if message.message_type == MessageType::ResponseSession
                        && message.message.is_none()
                    {
                        let sender = write_sender.clone();
                        let receiver = read_receiver.resubscribe();
                        weak.upgrade_in_event_loop(move |handle| {
                            let (session_name, session) = message.sessions.iter().next().unwrap();
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
                                sender,
                                receiver,
                                close_ping_rx.resubscribe(),
                                session.server_address.as_ref().unwrap().clone(),
                                session_name.into(),
                                session.ice_server_config.clone(),
                                session.game_name.as_ref().unwrap().into(),
                                handle.get_netplay_rom_path(),
                                ui::GameSettings {
                                    overclock: overclock.parse().unwrap(),
                                    disable_expansion_pak: disable_expansion_pak.parse().unwrap(),
                                    cheats: serde_json::from_str(cheats).unwrap(),
                                    load_savestate_slot: None,
                                },
                                &handle,
                            );
                        })
                        .unwrap();
                        return;
                    } else if message.message_type == MessageType::ResponseListSessions
                        && message.message.is_none()
                    {
                        let mut sessions = vec![];
                        for (session_name, remote_session) in message.sessions {
                            let mut session = vec![];

                            session.push(slint::StandardListViewItem::from(
                                slint::SharedString::from(session_name),
                            ));
                            session.push(slint::StandardListViewItem::from(
                                slint::SharedString::from(remote_session.game_name.unwrap()),
                            ));
                            session.push(slint::StandardListViewItem::from(
                                slint::SharedString::from(if remote_session.password.is_some() {
                                    "True"
                                } else {
                                    "False"
                                }),
                            ));
                            session.push(slint::StandardListViewItem::from(
                                slint::SharedString::from(
                                    if remote_session
                                        .features
                                        .unwrap_or_default()
                                        .contains_key("cheats")
                                    {
                                        "True"
                                    } else {
                                        "False"
                                    },
                                ),
                            ));
                            sessions.push(session);
                        }
                        weak.upgrade_in_event_loop(move |handle| {
                            let sessions_vec = slint::VecModel::default();
                            for session in sessions.iter() {
                                sessions_vec.push(slint::ModelRc::from(std::rc::Rc::new(
                                    slint::VecModel::from(session.to_vec()),
                                )));
                            }
                            handle.set_netplay_sessions(slint::ModelRc::from(std::rc::Rc::new(
                                sessions_vec,
                            )));

                            handle.set_netplay_current_session(-1);
                            handle.set_netplay_pending_refresh(false);
                        })
                        .unwrap();
                    } else {
                        weak.upgrade_in_event_loop(move |handle| {
                            handle.set_netplay_pending_session(false);
                            if let Some(message) = message.message {
                                handle.invoke_show_message(message.into(), true);
                            }
                        })
                        .unwrap();
                    }
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    weak.upgrade_in_event_loop(move |handle| {
                        handle.set_netplay_pending_session(false);
                        handle.invoke_show_message(
                            format!("Server did not respond: {e}").into(),
                            true,
                        );
                    })
                    .unwrap();
                }
            }
        }
    });

    app.set_show_netplay_join_session(true);
    app.invoke_netplay_refresh_sessions();
}

pub fn netplay_window(app: &AppWindow) {
    let (netplay_read_sender, netplay_read_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    ) = tokio::sync::broadcast::channel(5);

    let (netplay_write_sender, netplay_write_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayLobbyMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayLobbyMessage>>,
    ) = tokio::sync::broadcast::channel(5);

    let (close_ping_tx, close_ping_rx): (
        tokio::sync::broadcast::Sender<()>,
        tokio::sync::broadcast::Receiver<()>,
    ) = tokio::sync::broadcast::channel(5);

    setup_callbacks(
        app,
        netplay_write_sender.clone(),
        netplay_read_receiver.resubscribe(),
        netplay_read_sender.clone(),
        netplay_write_receiver.resubscribe(),
        close_ping_tx.clone(),
        close_ping_rx.resubscribe(),
    );
}
