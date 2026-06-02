use crate::device;
use crate::ui;
use crate::ui::gui::{AppWindow, open_uri, run_rom, save_settings};
use futures::{SinkExt, StreamExt};
use slint::ComponentHandle;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Utf8Bytes;
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
struct NetplaySession {
    protected: bool,
    password: Option<String>,
    game_name: Option<String>,
    motd: Option<String>,
    game_checksum: Option<String>,
    client_version: Option<String>,
    features: Option<std::collections::HashMap<String, String>>,
    players: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NetplayMessage {
    message_type: MessageType,
    sessions: std::collections::HashMap<String, NetplaySession>,
    message: Option<String>,
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
    let weak = app.as_weak();
    app.on_netplay_create_session(
        move |session_name, player_name, game_name, game_hash, game_cheats, password| {
            let _ = netplay_write_sender.send(None); // close current websocket if any
            manage_websocket(
                netplay_read_sender.clone(),
                netplay_write_receiver.resubscribe(),
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
        },
    );

    app.set_show_netplay_create_session(true);
}

fn manage_websocket(
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
) {
    let server_url = "ws://127.0.0.1:45000";
    tokio::spawn(async move {
        if let Ok(Ok((socket, _response))) = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio_tungstenite::connect_async(server_url),
        )
        .await
        {
            let (mut write, mut read) = socket.split();
            tokio::spawn(async move {
                while let Some(Ok(response)) = read.next().await {
                    if let Ok(message) =
                        postcard::from_bytes::<NetplayMessage>(&response.into_data())
                    {
                        let _ = netplay_read_sender.send(Some(message));
                    }
                }
            });
            tokio::spawn(async move {
                loop {
                    match netplay_write_receiver.recv().await {
                        Ok(Some(response)) => {
                            write
                                .send(Message::Binary(Bytes::from(
                                    postcard::to_stdvec(&response).unwrap(),
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
                write
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Normal,
                        reason: Utf8Bytes::from(""),
                    })))
                    .await
                    .unwrap();
            });
        }
    });
}

fn update_sessions(netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>) {
    let request_sessions = NetplayMessage {
        message_type: MessageType::RequestListSessions,
        sessions: std::collections::HashMap::new(),
        message: None,
    };

    netplay_write_sender.send(Some(request_sessions)).unwrap();
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
        let mut features = std::collections::HashMap::default();

        if !game_cheats.is_empty() {
            features.insert("cheats".to_string(), game_cheats);
        }

        features.insert("overclock".to_string(), game_settings.overclock.to_string());
        features.insert(
            "disable_expansion_pak".to_string(),
            game_settings.disable_expansion_pak.to_string(),
        );

        let session = NetplaySession {
            protected: false,
            password: Some(password),
            game_name: Some(game_name),
            motd: None,
            game_checksum: Some(game_hash),
            client_version: Some(env!("GIT_DESCRIBE").to_string()),
            features: Some(features),
            players: vec![player_name],
        };
        let create_session = NetplayMessage {
            message_type: MessageType::RequestCreateSession,
            sessions: std::collections::HashMap::from([(session_name, session)]),
            message: None,
        };

        netplay_write_sender.send(Some(create_session)).unwrap();

        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
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
                                session_name.into(),
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
    session_name: String,
    player_name: String,
    game_hash: String,
    password: String,
    weak_app: slint::Weak<AppWindow>,
) {
    tokio::spawn(async move {
        let session = NetplaySession {
            protected: false,
            password: Some(password),
            game_name: None,
            motd: None,
            game_checksum: Some(game_hash),
            client_version: Some(env!("GIT_DESCRIBE").to_string()),
            features: None,
            players: vec![player_name],
        };
        let join_session = NetplayMessage {
            message_type: MessageType::RequestJoinSession,
            sessions: std::collections::HashMap::from([(session_name, session)]),
            message: None,
        };

        netplay_write_sender.send(Some(join_session)).unwrap();

        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
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
                                session_name.into(),
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
                            slint::SharedString::from(if remote_session.protected {
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
                    weak_app
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

                            handle.set_netplay_current_session(-1);
                            handle.set_netplay_pending_refresh(false);
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
    game_settings: ui::GameSettings,
    app: &AppWindow,
) {
    app.set_netplay_session_name(session_name);
    app.set_netplay_game_name(game_name);
    app.set_netplay_rom_path(rom_path);

    let sender = netplay_write_sender.clone();
    app.on_netplay_send_chat_message(move |message| {
        let send_chat = NetplayMessage {
            message_type: MessageType::SendChatMessage,
            sessions: std::collections::HashMap::new(),
            message: Some(message.into()),
        };
        sender.send(Some(send_chat)).unwrap();
    });

    let sender = netplay_write_sender.clone();
    app.on_netplay_begin_game(move || {
        let begin_game = NetplayMessage {
            message_type: MessageType::RequestBeginGame,
            sessions: std::collections::HashMap::new(),
            message: None,
        };
        sender.send(Some(begin_game)).unwrap();
    });

    let weak_app = app.as_weak();
    tokio::spawn(async move {
        loop {
            match netplay_read_receiver.recv().await {
                Ok(Some(response)) => match response.message_type {
                    MessageType::ResponseSession => {
                        weak_app
                            .upgrade_in_event_loop(move |handle| {
                                let player_names =
                                    response.sessions.iter().next().unwrap().1.players.clone();

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
                                    let _ = netplay_write_sender.send(None);

                                    run_rom(
                                        handle.get_netplay_rom_path().as_str().into(),
                                        ui::GameSettings {
                                            overclock: game_settings.overclock,
                                            disable_expansion_pak: game_settings
                                                .disable_expansion_pak,
                                            cheats: game_settings.cheats,
                                            load_savestate_slot: None,
                                        },
                                        None,
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
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    netplay_read_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    netplay_read_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
) {
    let _ = netplay_write_sender.send(None); // close current websocket if any
    manage_websocket(
        netplay_read_sender.clone(),
        netplay_write_receiver.resubscribe(),
    );

    app.set_netplay_pending_refresh(true);

    let write_sender = netplay_write_sender.clone();
    app.on_netplay_refresh_sessions(move || {
        update_sessions(write_sender.clone());
    });

    app.invoke_netplay_refresh_sessions();

    let weak = app.as_weak();
    app.on_netplay_join_session(move |session_name, player_name, game_hash, password| {
        join_session(
            netplay_write_sender.clone(),
            netplay_read_receiver.resubscribe(),
            session_name.to_string(),
            player_name.to_string(),
            game_hash.to_string(),
            password.to_string(),
            weak.clone(),
        );
    });

    app.set_show_netplay_join_session(true);
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
                        cheats: rustc_hash::FxHashMap::default(), // not used here
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
                handle.set_show_netplay_wait_session(false);
                handle.set_show_netplay_create_session(false);
                handle.set_show_netplay_join_session(false);
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
