use crate::ui;

pub struct GgrsConfig;
impl ggrs::Config for GgrsConfig {
    type Input = u64;
    type InputPredictor = ggrs::PredictRepeatLast;
    type State = u64;
    type Address = matchbox_socket::PeerId;
}

pub struct MatchboxSocket(matchbox_socket::WebRtcSocket);

impl ggrs::NonBlockingSocket<matchbox_socket::PeerId> for MatchboxSocket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &matchbox_socket::PeerId) {
        let encoded = postcard::to_stdvec(msg).expect("serialization failed");
        let channel = self.0.get_channel_mut(0).unwrap();
        channel.send(encoded.into(), *addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(matchbox_socket::PeerId, ggrs::Message)> {
        let channel = self.0.get_channel_mut(0).unwrap();
        channel
            .receive()
            .iter()
            .filter_map(|(peer, packet)| {
                let msg = postcard::from_bytes::<ggrs::Message>(&packet).ok()?;
                Some((*peer, msg))
            })
            .collect()
    }
}

pub struct Netplay {
    pub session: ggrs::P2PSession<GgrsConfig>,
    pub reliable_channel: matchbox_socket::WebRtcChannel,
    pub peers: Vec<matchbox_socket::PeerId>,
    pub player_number: usize,
    pub connected: [bool; 4],
    pub data: std::collections::VecDeque<Vec<u8>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum MessageType {
    Register,
    SendData,
    ReceiveData,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NetplayMessage {
    message_type: MessageType,
    name: String,
    data: Vec<u8>,
}

fn send_message(netplay: &mut Netplay, message: NetplayMessage) {
    let data = postcard::to_stdvec(&message).unwrap();
    for peer in netplay.peers.iter() {
        netplay.reliable_channel.send(data.clone().into(), *peer);
    }
}

fn receive_message(netplay: &mut Netplay) -> NetplayMessage {
    while netplay.data.len() == 0 {
        let messages = netplay.reliable_channel.receive();
        netplay
            .data
            .extend(messages.iter().map(|(_, data)| data.to_vec()));
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let data = netplay.data.pop_front().unwrap();
    let message = postcard::from_bytes::<NetplayMessage>(&data).unwrap();
    message
}

pub fn send_rtc(netplay: &mut Netplay, rtc: i64) {
    let message = NetplayMessage {
        message_type: MessageType::SendData,
        name: "rtc".to_string(),
        data: rtc.to_be_bytes().to_vec(),
    };
    send_message(netplay, message);
}

pub fn receive_rtc(netplay: &mut Netplay) -> i64 {
    let message = NetplayMessage {
        message_type: MessageType::ReceiveData,
        name: "rtc".to_string(),
        data: vec![],
    };
    send_message(netplay, message);

    let message = receive_message(netplay);

    i64::from_be_bytes(message.data.try_into().unwrap())
}

pub fn send_rng(netplay: &mut Netplay, seed: u64) {
    let message = NetplayMessage {
        message_type: MessageType::SendData,
        name: "rng".to_string(),
        data: seed.to_be_bytes().to_vec(),
    };
    send_message(netplay, message);
}

pub fn receive_rng(netplay: &mut Netplay) -> u64 {
    let message = NetplayMessage {
        message_type: MessageType::ReceiveData,
        name: "rng".to_string(),
        data: vec![],
    };
    send_message(netplay, message);

    let message = receive_message(netplay);
    u64::from_be_bytes(message.data.try_into().unwrap())
}

pub fn send_save(netplay: &mut Netplay, save_type: &str, save_data: &[u8]) {
    let message = NetplayMessage {
        message_type: MessageType::SendData,
        name: save_type.to_string(),
        data: save_data.to_vec(),
    };
    send_message(netplay, message);
}

pub fn receive_save(netplay: &mut Netplay, save_type: &str, save_data: &mut Vec<u8>) {
    let message = NetplayMessage {
        message_type: MessageType::ReceiveData,
        name: save_type.to_string(),
        data: vec![],
    };
    send_message(netplay, message);

    let message = receive_message(netplay);
    *save_data = message.data;
}

pub fn process_netplay(netplay: &mut Netplay) {
    netplay.session.poll_remote_clients();
}

pub fn init(server_addr: String, player_number: usize, number_of_players: usize) -> Netplay {
    let (socket, loop_fut) = matchbox_socket::WebRtcSocketBuilder::new(server_addr)
        .add_unreliable_channel()
        .add_reliable_channel()
        .build();
    tokio::spawn(async move {
        if let Err(e) = loop_fut.await {
            eprintln!("WebRTC loop failed: {}", e);
        }
    });
    let mut matchbox_socket = MatchboxSocket(socket);
    let reliable_channel = matchbox_socket.0.take_channel(1).unwrap();
    let now = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);
    let mut peers = vec![];
    let mut session_builder = ggrs::SessionBuilder::<GgrsConfig>::new();
    loop {
        matchbox_socket.0.update_peers();
        peers = matchbox_socket
            .0
            .connected_peers()
            .collect::<Vec<matchbox_socket::PeerId>>();
        if peers.len() == number_of_players {
            session_builder = session_builder
                .with_num_players(number_of_players)
                .unwrap()
                .with_input_delay(2)
                .add_player(ggrs::PlayerType::Local, 0)
                .unwrap();
            for (i, peer) in peers.iter().enumerate() {
                session_builder = session_builder
                    .add_player(ggrs::PlayerType::Remote(*peer), i + 1)
                    .unwrap();
            }

            break;
        }
        if now.elapsed() > timeout {
            ui::video::onscreen_message(
                "Failed to connect to netplay server",
                ui::video::MESSAGE_LENGTH_MESSAGE_LONG,
            );
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let session = session_builder.start_p2p_session(matchbox_socket).unwrap();
    Netplay {
        session,
        reliable_channel,
        peers,
        player_number,
        connected: [false; 4],
        data: std::collections::VecDeque::new(),
    }
}
