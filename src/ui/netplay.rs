use slint::{ComponentHandle, Model};

use crate::ui::gui::{NetplayCreate, NetplayDialog, NetplayJoin};
use futures::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
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

pub fn setup_create_window(create_window: &NetplayCreate) {
    let weak = create_window.as_weak();
    populate_server_names(weak);
    let weak2 = create_window.as_weak();
    create_window.on_get_ping(move |server_url| {
        update_ping(weak2.clone(), server_url.to_string());
    });

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

fn clear_sessions(handle: &NetplayJoin, message: Option<String>) {
    handle.set_sessions(slint::ModelRc::default());
    handle.set_current_session(-1);
    if let Some(message) = message {
        let message_dialog = NetplayDialog::new().unwrap();
        let weak_dialog = message_dialog.as_weak();
        message_dialog.on_close_clicked(move || {
            weak_dialog.unwrap().window().hide().unwrap();
        });
        message_dialog.set_text(message.into());
        message_dialog.show().unwrap();
    }
}

fn update_sessions(
    server_url: String,
    netplay_write_sender: &tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
    netplay_read_sender: &tokio::sync::broadcast::Sender<NetplayMessage>,
    netplay_write_receiver: &tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    netplay_read_receiver: &tokio::sync::broadcast::Receiver<NetplayMessage>,
    weak: slint::Weak<NetplayJoin>,
) {
    netplay_write_sender.send(None).unwrap(); // close current websocket if any
    manage_websocket(
        server_url,
        netplay_read_sender.clone(),
        netplay_write_receiver.resubscribe(),
    );

    let mut receiver = netplay_read_receiver.resubscribe();
    let writer = netplay_write_sender.clone();
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

        writer.send(Some(request_rooms)).unwrap();

        if let Ok(Ok(message)) =
            tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv()).await
        {
            if message.accept.unwrap() == 0 {
                if let Some(rooms) = message.rooms {
                    weak.upgrade_in_event_loop(move |handle| {
                        let sessions_vec = slint::VecModel::default();
                        for room in rooms {
                            let session_vec = slint::VecModel::default();
                            let mut room_name = slint::StandardListViewItem::default();
                            room_name.text = room.room_name.unwrap().into();
                            session_vec.push(room_name);
                            let mut game_name = slint::StandardListViewItem::default();
                            game_name.text = room.game_name.unwrap().into();
                            session_vec.push(game_name);
                            let mut password_protected = slint::StandardListViewItem::default();
                            password_protected.text = if room.protected.unwrap() {
                                "True".into()
                            } else {
                                "False".into()
                            };
                            session_vec.push(password_protected);
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

pub fn setup_join_window(join_window: &NetplayJoin) {
    let (netplay_read_sender, netplay_read_receiver): (
        tokio::sync::broadcast::Sender<NetplayMessage>,
        tokio::sync::broadcast::Receiver<NetplayMessage>,
    ) = tokio::sync::broadcast::channel(1);

    let (netplay_write_sender, netplay_write_receiver): (
        tokio::sync::broadcast::Sender<Option<NetplayMessage>>,
        tokio::sync::broadcast::Receiver<Option<NetplayMessage>>,
    ) = tokio::sync::broadcast::channel(1);

    let weak = join_window.as_weak();
    populate_server_names(weak);
    let weak2 = join_window.as_weak();
    join_window.on_get_ping(move |server_url| {
        update_ping(weak2.clone(), server_url.to_string());
        weak2
            .upgrade_in_event_loop(move |handle| {
                handle.invoke_refresh_session(server_url);
            })
            .unwrap();
    });

    let netplay_write_sender_closed = netplay_write_sender.clone();
    join_window.window().on_close_requested(move || {
        netplay_write_sender_closed.send(None).unwrap(); // close current websocket if any
        slint::CloseRequestResponse::HideWindow
    });

    let weak3 = join_window.as_weak();
    join_window.on_refresh_session(move |server_url| {
        update_sessions(
            server_url.to_string(),
            &netplay_write_sender,
            &netplay_read_sender,
            &netplay_write_receiver,
            &netplay_read_receiver,
            weak3.clone(),
        );
    });

    join_window.show().unwrap();
}
