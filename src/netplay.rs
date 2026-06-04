use crate::device;
use crate::savestates;
use crate::ui;
use sha2::digest::Digest;

pub struct GgrsConfig;
impl ggrs::Config for GgrsConfig {
    type Input = ui::input::InputData;
    type InputPredictor = ggrs::PredictRepeatLast;
    type State = i32;
    type Address = matchbox_socket::PeerId;
}

pub struct MatchboxSocket(matchbox_socket::WebRtcSocket);

impl ggrs::NonBlockingSocket<matchbox_socket::PeerId> for MatchboxSocket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &matchbox_socket::PeerId) {
        let encoded = postcard::to_stdvec(msg).expect("serialization failed");
        let channel = self.0.get_channel_mut(0).unwrap();
        if channel.config().max_retransmits != Some(0) || channel.config().ordered {
            eprintln!("Sending GGRS traffic over reliable channel");
        }
        channel.send(encoded.into(), *addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(matchbox_socket::PeerId, ggrs::Message)> {
        let channel = self.0.get_channel_mut(0).unwrap();
        channel
            .receive()
            .iter()
            .filter_map(|(peer, packet)| {
                let msg = postcard::from_bytes::<ggrs::Message>(packet).ok()?;
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
    pub data: std::collections::HashMap<String, Vec<u8>>,
    pub inputs: Vec<(ui::input::InputData, ggrs::InputStatus)>,
    pub requests: std::collections::VecDeque<ggrs::GgrsRequest<GgrsConfig>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NetplayMessage {
    name: String,
    data: Vec<u8>,
}

fn send_message(netplay: &mut Netplay, message: NetplayMessage) {
    let data = postcard::to_stdvec(&message).unwrap();
    for peer in netplay.peers.iter() {
        netplay.reliable_channel.send(data.clone().into(), *peer);
    }
}

fn receive_message(netplay: &mut Netplay, name: &str) -> Vec<u8> {
    while !netplay.data.contains_key(name) {
        let messages = netplay.reliable_channel.receive();
        for (_, data) in messages {
            let message = postcard::from_bytes::<NetplayMessage>(&data).unwrap();
            netplay.data.insert(message.name, message.data);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    netplay.data.remove(name).unwrap()
}

fn send_player_number(
    channel: &mut matchbox_socket::WebRtcChannel,
    peers: Vec<matchbox_socket::PeerId>,
    player_number: usize,
) {
    let message = NetplayMessage {
        name: "player_number".to_string(),
        data: player_number.to_be_bytes().to_vec(),
    };
    let data = postcard::to_stdvec(&message).unwrap();
    for peer in peers {
        channel.send(data.clone().into(), peer);
    }
}

fn get_player_numbers(
    channel: &mut matchbox_socket::WebRtcChannel,
    local_player_number: usize,
    number_of_peers: usize,
) -> std::collections::BTreeMap<usize, Option<matchbox_socket::PeerId>> {
    let mut player_numbers = std::collections::BTreeMap::new();
    player_numbers.insert(local_player_number, None);
    let mut messages = vec![];
    while messages.len() < number_of_peers {
        messages.extend(channel.receive());
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    for (peer, data) in messages {
        let message = postcard::from_bytes::<NetplayMessage>(&data).unwrap();
        player_numbers.insert(
            usize::from_be_bytes(message.data.try_into().unwrap()),
            Some(peer),
        );
    }
    player_numbers
}

pub fn send_rtc(netplay: &mut Netplay, rtc: i64) {
    let message = NetplayMessage {
        name: "rtc".to_string(),
        data: rtc.to_be_bytes().to_vec(),
    };
    send_message(netplay, message);
}

pub fn receive_rtc(netplay: &mut Netplay) -> i64 {
    let message = receive_message(netplay, "rtc");

    i64::from_be_bytes(message.try_into().unwrap())
}

pub fn send_rng(netplay: &mut Netplay, seed: u64) {
    let message = NetplayMessage {
        name: "rng".to_string(),
        data: seed.to_be_bytes().to_vec(),
    };
    send_message(netplay, message);
}

pub fn receive_rng(netplay: &mut Netplay) -> u64 {
    let message = receive_message(netplay, "rng");
    u64::from_be_bytes(message.try_into().unwrap())
}

pub fn send_save(netplay: &mut Netplay, save_type: &str, save_data: &[u8]) {
    let message = NetplayMessage {
        name: save_type.to_string(),
        data: save_data.to_vec(),
    };
    send_message(netplay, message);
}

pub fn receive_save(netplay: &mut Netplay, save_type: &str, save_data: &mut Vec<u8>) {
    let message = receive_message(netplay, save_type);
    *save_data = message;
}

pub fn pending_frames(netplay: &Netplay) -> usize {
    netplay
        .requests
        .iter()
        .filter(|r| matches!(r, ggrs::GgrsRequest::AdvanceFrame { .. }))
        .count()
}

pub fn process_requests(
    device: &mut device::Device,
) -> Vec<(ui::input::InputData, ggrs::InputStatus)> {
    loop {
        if let Some(request) = device.netplay.as_mut().unwrap().requests.pop_front() {
            match request {
                ggrs::GgrsRequest::SaveGameState { cell, frame } => {
                    savestates::create_savestate(device, true, Some(frame));

                    let mut hasher = sha2::Sha256::new();
                    for reg in device.cpu.cop0.regs.as_ref() {
                        hasher.update(reg.to_be_bytes());
                    }
                    let hash = u128::from_be_bytes(hasher.finalize()[..16].try_into().unwrap());
                    cell.save(frame, Some(frame), Some(hash));
                }
                ggrs::GgrsRequest::LoadGameState { cell, frame: _ } => {
                    if let Some(frame) = cell.load() {
                        savestates::load_savestate(device, true, Some(frame));
                    }
                }
                ggrs::GgrsRequest::AdvanceFrame { inputs } => {
                    return inputs;
                }
            }
        } else {
            let netplay = device.netplay.as_mut().unwrap();
            netplay.session.poll_remote_clients();
            advance_frame(device);
        }
    }
}

pub fn process_netplay(
    device: &mut device::Device,
) -> Vec<(ui::input::InputData, ggrs::InputStatus)> {
    let netplay = device.netplay.as_mut().unwrap();

    netplay.session.poll_remote_clients();
    for event in netplay.session.events() {
        match event {
            ggrs::GgrsEvent::Synchronizing { .. } => {}
            ggrs::GgrsEvent::Synchronized { .. } => {}
            ggrs::GgrsEvent::Disconnected { .. } => {
                ui::video::onscreen_message(
                    "Peer disconnected",
                    ui::video::MESSAGE_LENGTH_MESSAGE_LONG,
                );
            }
            ggrs::GgrsEvent::NetworkInterrupted { .. } => {
                println!("network interrupted");
            }
            ggrs::GgrsEvent::NetworkResumed { .. } => {
                println!("network resumed");
            }
            ggrs::GgrsEvent::WaitRecommendation { skip_frames } => {
                println!("wait recommendation: skip_frames={}", skip_frames);
            }
            ggrs::GgrsEvent::DesyncDetected { .. } => {
                ui::video::onscreen_message(
                    "Desync detected",
                    ui::video::MESSAGE_LENGTH_MESSAGE_LONG,
                );
            }
        }
    }

    advance_frame(device);
    process_requests(device)
}

fn advance_frame(device: &mut device::Device) {
    let netplay = device.netplay.as_mut().unwrap();
    let local_input = ui::input::get(&mut device.ui, 0, device.frame_counter);
    let local_handle = *netplay.session.local_player_handles().first().unwrap();
    netplay
        .session
        .add_local_input(local_handle, local_input)
        .unwrap();
    match netplay.session.advance_frame() {
        Ok(requests) => {
            netplay.requests.extend(requests);
        }
        Err(ggrs::GgrsError::PredictionThreshold) => {
            println!("prediction threshold reached");
        }
        Err(e) => panic!("{e}"),
    }
}

pub fn init(
    server_addr: String,
    player_number: usize,
    number_of_players: usize,
    input_delay: usize,
) -> Netplay {
    let (socket, loop_fut) =
        matchbox_socket::WebRtcSocketBuilder::new(format!("ws://{}", server_addr))
            .add_unreliable_channel()
            .add_reliable_channel()
            .build();
    tokio::spawn(async move {
        if let Err(e) = loop_fut.await {
            eprintln!("WebRTC loop failed: {}", e);
        }
    });
    let mut matchbox_socket = MatchboxSocket(socket);
    let mut reliable_channel = matchbox_socket.0.take_channel(1).unwrap();
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
        if peers.len() == number_of_players - 1 {
            send_player_number(&mut reliable_channel, peers.clone(), player_number);
            let player_numbers =
                get_player_numbers(&mut reliable_channel, player_number, peers.len());
            session_builder = session_builder
                .with_num_players(number_of_players)
                .unwrap()
                .with_input_delay(input_delay)
                .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 60 });
            for (i, peer) in player_numbers.iter() {
                if let Some(peer) = peer {
                    session_builder = session_builder
                        .add_player(ggrs::PlayerType::Remote(*peer), *i)
                        .unwrap();
                } else {
                    session_builder = session_builder
                        .add_player(ggrs::PlayerType::Local, *i)
                        .unwrap();
                }
            }

            break;
        }
        if now.elapsed() > timeout {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let mut session = session_builder.start_p2p_session(matchbox_socket).unwrap();

    let now = std::time::Instant::now();
    while session.current_state() != ggrs::SessionState::Running && now.elapsed() < timeout {
        session.poll_remote_clients();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Netplay {
        session,
        reliable_channel,
        peers,
        player_number,
        connected: [
            number_of_players > 0,
            number_of_players > 1,
            number_of_players > 2,
            number_of_players > 3,
        ],
        data: std::collections::HashMap::new(),
        inputs: Vec::new(),
        requests: std::collections::VecDeque::new(),
    }
}
