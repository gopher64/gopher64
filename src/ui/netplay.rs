use crate::device;
use crate::ui;
use crate::ui::gui::{NetplayCreate, NetplayDialog, NetplayJoin};
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
    fn set_game_name(&self, ping: slint::SharedString);
    fn set_game_hash(&self, ping: slint::SharedString);
    fn refresh_sessions(&self, _server: slint::SharedString) {
        // Default implementation does nothing
    }
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
}

fn populate_server_names<T: ComponentHandle + NetplayPages + 'static>(weak: slint::Weak<T>) {
    let task = reqwest::get("https://m64p.s3.amazonaws.com/servers.json");
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

fn select_rom<T: ComponentHandle + NetplayPages + 'static>(weak: slint::Weak<T>) {
    let select_rom = rfd::AsyncFileDialog::new()
        .set_title("Select ROM")
        .pick_file();
    tokio::spawn(async move {
        if let Some(file) = select_rom.await {
            if let Some(rom_contents) = device::get_rom_contents(file.path()) {
                let hash = device::cart::rom::calculate_hash(&rom_contents);
                let game_name = ui::storage::get_game_name(&rom_contents);
                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_game_name(game_name.into());
                    handle.set_game_hash(hash.into());
                })
                .unwrap();
            } else {
                weak.upgrade_in_event_loop(move |handle| {
                    let message_dialog = NetplayDialog::new().unwrap();
                    let weak_dialog = message_dialog.as_weak();
                    message_dialog.on_close_clicked(move || {
                        weak_dialog.unwrap().window().hide().unwrap();
                    });
                    message_dialog.set_text("Could not read ROM".into());
                    message_dialog.show().unwrap();

                    handle.set_game_name("".into());
                    handle.set_game_hash("".into());
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

pub fn setup_create_window(create_window: &NetplayCreate, overclock_setting: bool) {
    let (netplay_read_sender, netplay_read_receiver): (
        tokio::sync::broadcast::Sender<NetplayMessage>,
        tokio::sync::broadcast::Receiver<NetplayMessage>,
    ) = tokio::sync::broadcast::channel(1);

    let (netplay_write_sender, netplay_write_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    ) = tokio::sync::broadcast::channel(1);

    populate_server_names(create_window.as_weak());
    let weak = create_window.as_weak();
    create_window.on_get_ping(move |server_url| {
        update_ping(weak.clone(), server_url.to_string());
    });
    let weak = create_window.as_weak();
    create_window.on_select_rom(move || {
        select_rom(weak.clone());
    });

    let netplay_write_sender_on_create_session = netplay_write_sender.clone();
    let netplay_read_receiver_on_create_session = netplay_read_receiver.resubscribe();
    let weak = create_window.as_weak();
    create_window.on_create_session(
        move |server_url, session_name, player_name, game_name, game_hash, password| {
            netplay_write_sender_on_create_session.send(None).unwrap(); // close current websocket if any
            manage_websocket(
                server_url.to_string(),
                netplay_read_sender.clone(),
                netplay_write_receiver.resubscribe(),
            );

            create_session(
                netplay_write_sender_on_create_session.clone(),
                netplay_read_receiver_on_create_session.resubscribe(),
                session_name.to_string(),
                player_name.to_string(),
                game_name.to_string(),
                game_hash.to_string(),
                password.to_string(),
                overclock_setting,
                weak.clone(),
            );
        },
    );

    create_window.show().unwrap();
}

fn manage_websocket(
    server_url: String,
    netplay_read_sender: tokio::sync::broadcast::Sender<NetplayMessage>,
    mut netplay_write_receiver: tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
) {
    tokio::spawn(async move {
        if let Ok(Ok((socket, _response))) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            tokio_tungstenite::connect_async(server_url.clone()),
        )
        .await
        {
            let (mut write, mut read) = socket.split();
            tokio::spawn(async move {
                while let Some(Ok(response)) = read.next().await {
                    if let Ok(message) = serde_json::from_slice(&response.into_data()) {
                        netplay_read_sender.send(message).unwrap();
                    }
                }
            });
            tokio::spawn(async move {
                while let Ok(Some(response)) = netplay_write_receiver.recv().await {
                    write
                        .send(Message::Binary(Bytes::from(
                            serde_json::to_vec(&response).unwrap(),
                        )))
                        .await
                        .unwrap();
                }
                write.close().await.unwrap();
            });
        }
    });
}

fn show_netplay_error(message: String) {
    let message_dialog = NetplayDialog::new().unwrap();
    let weak_dialog = message_dialog.as_weak();
    message_dialog.on_close_clicked(move || {
        weak_dialog.unwrap().window().hide().unwrap();
    });
    message_dialog.set_text(message.into());
    message_dialog.show().unwrap();
}

fn clear_sessions(handle: &NetplayJoin, message: Option<String>) {
    handle.set_sessions(slint::ModelRc::default());
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

        if let Ok(Ok(message)) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            netplay_read_receiver.recv(),
        )
        .await
        {
            if message.accept.unwrap() == 0 {
                if let Some(rooms) = message.rooms {
                    weak.upgrade_in_event_loop(move |handle| {
                        let sessions_vec = slint::VecModel::default();
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
                            let session_model: std::rc::Rc<
                                slint::VecModel<slint::StandardListViewItem>,
                            > = std::rc::Rc::new(session_vec);
                            sessions_vec.push(slint::ModelRc::from(session_model));
                        }
                        let rooms_model: std::rc::Rc<
                            slint::VecModel<slint::ModelRc<slint::StandardListViewItem>>,
                        > = std::rc::Rc::new(sessions_vec);
                        handle.set_sessions(slint::ModelRc::from(rooms_model));
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
        } else {
            weak.upgrade_in_event_loop(move |handle| {
                clear_sessions(&handle, Some("Server did not respond".to_string()));
            })
            .unwrap();
        }
    });
}

fn create_session(
    netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    mut netplay_read_receiver: tokio::sync::broadcast::Receiver<NetplayMessage>,
    session_name: String,
    player_name: String,
    game_name: String,
    game_hash: String,
    password: String,
    overclock: bool,
    weak: slint::Weak<NetplayCreate>,
) {
    tokio::spawn(async move {
        let now_utc = chrono::Utc::now().timestamp_millis().to_string();
        let hasher = Sha256::new().chain_update(&now_utc).chain_update(EMU_NAME);
        let mut features = std::collections::HashMap::new();
        features.insert("overclock".to_string(), overclock.to_string());

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

        if let Ok(Ok(message)) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            netplay_read_receiver.recv(),
        )
        .await
        {
            if message.accept.unwrap() == 0 {
                // move to waiting room
            } else {
                weak.upgrade_in_event_loop(move |_handle| {
                    if let Some(message) = message.message {
                        show_netplay_error(message);
                    }
                })
                .unwrap();
            }
        } else {
            weak.upgrade_in_event_loop(move |_handle| {
                show_netplay_error("Server did not respond".to_string());
            })
            .unwrap();
        }
    });
}

fn join_session(
    _netplay_write_sender: tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    _netplay_read_receiver: tokio::sync::broadcast::Receiver<NetplayMessage>,
) {
}

pub fn setup_join_window(join_window: &NetplayJoin) {
    let (netplay_read_sender, netplay_read_receiver): (
        tokio::sync::broadcast::Sender<NetplayMessage>,
        tokio::sync::broadcast::Receiver<NetplayMessage>,
    ) = tokio::sync::broadcast::channel(1);

    let (netplay_write_sender, netplay_write_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    ) = tokio::sync::broadcast::channel(1);

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
    join_window.on_select_rom(move || {
        select_rom(weak.clone());
    });
    let netplay_write_sender_on_close_requested = netplay_write_sender.clone();
    join_window.window().on_close_requested(move || {
        netplay_write_sender_on_close_requested.send(None).unwrap(); // close current websocket if any
        slint::CloseRequestResponse::HideWindow
    });

    let weak = join_window.as_weak();
    let netplay_write_sender_on_refresh_session = netplay_write_sender.clone();
    let netplay_read_receiver_on_refresh_session = netplay_read_receiver.resubscribe();
    join_window.on_refresh_session(move |server_url| {
        netplay_write_sender_on_refresh_session.send(None).unwrap(); // close current websocket if any
        manage_websocket(
            server_url.to_string(),
            netplay_read_sender.clone(),
            netplay_write_receiver.resubscribe(),
        );
        update_sessions(
            netplay_write_sender_on_refresh_session.clone(),
            netplay_read_receiver_on_refresh_session.resubscribe(),
            weak.clone(),
        );
    });
    let netplay_write_sender_on_join_session = netplay_write_sender.clone();
    let netplay_read_receiver_on_join_session = netplay_read_receiver.resubscribe();
    join_window.on_join_session(move || {
        join_session(
            netplay_write_sender_on_join_session.clone(),
            netplay_read_receiver_on_join_session.resubscribe(),
        );
    });

    join_window.show().unwrap();
}
