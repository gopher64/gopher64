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

pub struct MatchboxChannel {
    channel: matchbox_socket::WebRtcChannel,
    disconnected_peers: tokio::sync::mpsc::Sender<matchbox_socket::PeerId>,
}

impl ggrs::NonBlockingSocket<matchbox_socket::PeerId> for MatchboxChannel {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &matchbox_socket::PeerId) {
        let encoded = postcard::to_stdvec(msg).expect("serialization failed");
        if self.channel.try_send(encoded.into(), *addr).is_err() {
            self.disconnected_peers.try_send(*addr).unwrap();
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(matchbox_socket::PeerId, ggrs::Message)> {
        self.channel
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
    pub input_delay: usize,
    pub messages: std::collections::HashMap<String, Vec<u8>>,
    pub received_data: std::collections::VecDeque<Vec<u8>>,
    pub inputs: Vec<(ui::input::InputData, ggrs::InputStatus)>,
    pub requests: std::collections::VecDeque<ggrs::GgrsRequest<GgrsConfig>>,
    pub incoming_message: Vec<u8>,
    pub disconnected_peers: tokio::sync::mpsc::Receiver<matchbox_socket::PeerId>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NetplayMessage {
    name: String,
    data: Vec<u8>,
}

fn send_message(netplay: &mut Netplay, message: NetplayMessage) {
    let data = postcard::to_stdvec(&message).unwrap();
    let chunks = data.chunks(16384).collect::<Vec<&[u8]>>();
    for peer in netplay.peers.iter() {
        for chunk in chunks.iter() {
            if let Err(e) = netplay
                .reliable_channel
                .try_send(chunk.to_vec().into(), *peer)
            {
                eprintln!("Failed to send message: {}", e);
            }
        }
    }
}

fn process_reliable_messages(netplay: &mut Netplay) {
    netplay.received_data.extend(
        netplay
            .reliable_channel
            .receive()
            .iter()
            .map(|(_, data)| data.to_vec()),
    );

    while !netplay.received_data.is_empty() {
        if let Some(data) = netplay.received_data.pop_front() {
            netplay.incoming_message.extend(data);

            if let Ok(decoded_message) =
                postcard::from_bytes::<NetplayMessage>(&netplay.incoming_message)
            {
                netplay
                    .messages
                    .insert(decoded_message.name, decoded_message.data);
                netplay.incoming_message.clear();
                check_input_delay(netplay);
            }
        }
    }
}

fn receive_message(netplay: &mut Netplay, name: &str) -> Vec<u8> {
    let timeout = std::time::Duration::from_secs(10);
    let now = std::time::Instant::now();

    loop {
        process_reliable_messages(netplay);
        if let Some(data) = netplay.messages.remove(name) {
            return data;
        }

        if now.elapsed() > timeout {
            panic!("Could not receive message for {name}");
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
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
        if let Err(e) = channel.try_send(data.clone().into(), peer) {
            eprintln!("Failed to send message: {}", e);
        }
    }
}

fn get_player_numbers(
    channel: &mut matchbox_socket::WebRtcChannel,
    player_numbers: &mut std::collections::BTreeMap<usize, Option<matchbox_socket::PeerId>>,
) {
    for (peer, data) in channel.receive() {
        let message = postcard::from_bytes::<NetplayMessage>(&data).unwrap();
        if message.name == "player_number" {
            player_numbers.insert(
                usize::from_be_bytes(message.data.try_into().unwrap()),
                Some(peer),
            );
        }
    }
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

pub fn send_input_delay(netplay: &mut Netplay, input_delay: usize) {
    let message = NetplayMessage {
        name: "input_delay".to_string(),
        data: input_delay.to_be_bytes().to_vec(),
    };
    send_message(netplay, message);
    change_input_delay(netplay, input_delay);
}

fn change_input_delay(netplay: &mut Netplay, input_delay: usize) {
    netplay.input_delay = input_delay;
    for handle in netplay.session.local_player_handles() {
        if let Err(e) = netplay.session.set_input_delay(handle, input_delay) {
            eprintln!("Error setting input delay: {}", e);
        } else {
            ui::video::onscreen_message(
                &format!("Input delay: {}", input_delay),
                ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
            );
        }
    }
}

fn check_input_delay(netplay: &mut Netplay) {
    if let Some(data) = netplay.messages.remove("input_delay") {
        let input_delay = usize::from_be_bytes(data.try_into().unwrap());
        if input_delay != netplay.input_delay {
            change_input_delay(netplay, input_delay);
        }
    }
}

fn pending_frames(netplay: &Netplay) -> usize {
    netplay
        .requests
        .iter()
        .filter(|r| matches!(r, ggrs::GgrsRequest::AdvanceFrame { .. }))
        .count()
}

pub fn in_rollback(netplay: Option<&Netplay>) -> bool {
    if let Some(netplay) = netplay {
        pending_frames(netplay) != 0
    } else {
        false
    }
}

fn process_disconnected_peers(netplay: &mut Netplay) {
    while let Ok(addr) = netplay.disconnected_peers.try_recv() {
        for handle in netplay.session.handles_by_address(addr) {
            if let Err(e) = netplay.session.disconnect_player(handle) {
                eprintln!("Error disconnecting player: {}", e);
            } else {
                ui::video::onscreen_message(
                    &format!("Player {} disconnected", handle + 1),
                    ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
                );
            }
        }
    }
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
            // unsafe { sdl3_sys::events::SDL_PumpEvents() }; // so the screen doesn't freeze
            process_netplay(device);
        }
    }
}

fn process_netplay(device: &mut device::Device) {
    let netplay = device.netplay.as_mut().unwrap();
    process_disconnected_peers(netplay);

    netplay.session.poll_remote_clients();
    for event in netplay.session.events() {
        match event {
            ggrs::GgrsEvent::Synchronizing { .. } => {}
            ggrs::GgrsEvent::Synchronized { .. } => {}
            ggrs::GgrsEvent::Disconnected { .. } => {
                ui::video::onscreen_message(
                    "Lost connection to peer",
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
                eprintln!("desync detected");
                ui::video::onscreen_message(
                    "Desync detected",
                    ui::video::MESSAGE_LENGTH_MESSAGE_LONG,
                );
            }
        }
    }

    process_reliable_messages(netplay);
    advance_frame(device);
}

fn advance_frame(device: &mut device::Device) {
    let netplay = device.netplay.as_mut().unwrap();
    let local_input = ui::input::get(&mut device.ui, 0, device.speed_limiter.frame_counter);
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
    pal: bool,
) -> Option<Netplay> {
    let (mut socket, loop_fut) = matchbox_socket::WebRtcSocketBuilder::new(server_addr)
        .add_unreliable_channel()
        .add_reliable_channel()
        .build();
    tokio::spawn(async move {
        if let Err(e) = loop_fut.await {
            eprintln!("WebRTC loop failed: {}", e);
        }
    });

    let (disconnected_peers_tx, disconnected_peers_rx) = tokio::sync::mpsc::channel(100);
    let matchbox_channel = MatchboxChannel {
        channel: socket.take_channel(0).unwrap(),
        disconnected_peers: disconnected_peers_tx,
    };
    let mut reliable_channel = socket.take_channel(1).unwrap();
    let now = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);
    let mut player_numbers = std::collections::BTreeMap::new();
    player_numbers.insert(player_number, None);
    loop {
        socket.update_peers();
        let peers = socket
            .connected_peers()
            .collect::<Vec<matchbox_socket::PeerId>>();

        send_player_number(&mut reliable_channel, peers, player_number);
        get_player_numbers(&mut reliable_channel, &mut player_numbers);
        if player_numbers.len() == number_of_players {
            break;
        }

        if now.elapsed() > timeout {
            eprintln!("Could not connect to netplay peers");
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let mut session_builder = ggrs::SessionBuilder::<GgrsConfig>::new()
        .with_num_players(number_of_players)
        .unwrap()
        .with_input_delay(input_delay)
        .with_fps(if pal { 50 } else { 60 })
        .unwrap()
        .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 60 })
        .with_max_prediction_window(0)
        .with_disconnect_timeout(std::time::Duration::from_secs(if cfg!(debug_assertions) {
            10
        } else {
            5
        }));

    let mut peers = vec![];
    for (i, peer) in player_numbers.iter() {
        if let Some(peer) = peer {
            session_builder = session_builder
                .add_player(ggrs::PlayerType::Remote(*peer), *i)
                .unwrap();
            peers.push(*peer);
        } else {
            session_builder = session_builder
                .add_player(ggrs::PlayerType::Local, *i)
                .unwrap();
        }
    }

    if matchbox_channel.channel.config().max_retransmits != Some(0)
        || matchbox_channel.channel.config().ordered
    {
        eprintln!("Sending GGRS traffic over reliable channel");
    }

    let mut session = session_builder.start_p2p_session(matchbox_channel).unwrap();

    let now = std::time::Instant::now();
    while session.current_state() != ggrs::SessionState::Running {
        session.poll_remote_clients();
        if now.elapsed() > timeout {
            eprintln!("Could not start netplay session");
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Some(Netplay {
        disconnected_peers: disconnected_peers_rx,
        incoming_message: vec![],
        input_delay,
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
        inputs: Vec::new(),
        requests: std::collections::VecDeque::new(),
        received_data: std::collections::VecDeque::new(),
        messages: std::collections::HashMap::new(),
    })
}
