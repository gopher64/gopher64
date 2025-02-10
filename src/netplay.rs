use crate::{device, ui};
use eframe::egui;
use std::io::{Read, Write};
//UDP packet formats
const UDP_SEND_KEY_INFO: u8 = 0;
const UDP_RECEIVE_KEY_INFO: u8 = 1;
const UDP_REQUEST_KEY_INFO: u8 = 2;
const UDP_RECEIVE_KEY_INFO_GRATUITOUS: u8 = 3;
const UDP_SYNC_DATA: u8 = 4;

//TCP packet formats
const TCP_SEND_SAVE: u8 = 1;
//const TCP_RECEIVE_SAVE: u8 = 2;
//const TCP_SEND_SETTINGS: u8 = 3;
//const TCP_RECEIVE_SETTINGS: u8 = 4;
const TCP_REGISTER_PLAYER: u8 = 5;
const TCP_GET_REGISTRATION: u8 = 6;
const TCP_DISCONNECT_NOTICE: u8 = 7;
const TCP_RECEIVE_SAVE_WITH_SIZE: u8 = 8;

const CS4: u32 = 32;

pub struct Netplay {
    udp_socket: std::net::UdpSocket,
    tcp_stream: std::net::TcpStream,
    pub player_number: u8,
    pub player_data: [PlayerData; 4],
    vi_counter: u32,
    status: u8,
    buffer_target: u8,
    pub fast_forward: bool,
    pub error_notifier: tokio::sync::mpsc::Sender<String>,
    pub gui_ctx: egui::Context,
}

pub struct PlayerData {
    lag: u8,
    count: u32,
    pub reg_id: u32,
    input_events: std::collections::HashMap<u32, InputEvent>,
}

struct InputEvent {
    input: u32,
    plugin: u8,
}

fn send_error(netplay: &mut Netplay, error: String) {
    netplay.error_notifier.try_send(error).unwrap();

    netplay.gui_ctx.request_repaint(); // this is so the window pops up right away
}

pub fn send_save(netplay: &mut Netplay, save_type: &str, save_data: &[u8], size: usize) {
    let mut request: Vec<u8> = [TCP_SEND_SAVE].to_vec();
    request.extend_from_slice(save_type.as_bytes());
    request.push(0); // null terminate string
    request.extend_from_slice(&(size as u32).to_be_bytes());

    if size > 0 {
        request.extend(save_data.to_owned());
    }
    netplay.tcp_stream.write_all(&request).unwrap();
}

pub fn receive_save(netplay: &mut Netplay, save_type: &str, save_data: &mut Vec<u8>) {
    let mut request: Vec<u8> = [TCP_RECEIVE_SAVE_WITH_SIZE].to_vec();
    request.extend_from_slice(save_type.as_bytes());
    request.push(0); // null terminate string
    netplay.tcp_stream.write_all(&request).unwrap();

    let mut size: [u8; 4] = [0; 4];
    netplay.tcp_stream.read_exact(&mut size).unwrap();
    let mut response: Vec<u8> = vec![0; u32::from_be_bytes(size) as usize];
    netplay.tcp_stream.read_exact(&mut response).unwrap();
    *save_data = response;
}

pub fn send_sync_check(device: &mut device::Device) {
    let netplay = device.netplay.as_mut().unwrap();
    if netplay.vi_counter % 600 == 0 {
        let mut request: Vec<u8> = [UDP_SYNC_DATA].to_vec();
        request.extend_from_slice(&(netplay.vi_counter).to_be_bytes());

        for i in 0..device::cop0::COP0_REGS_COUNT as usize {
            request.extend_from_slice(&(device.cpu.cop0.regs[i] as u32).to_be_bytes());
        }

        netplay.udp_socket.send(&request).unwrap();
    }
    netplay.vi_counter = netplay.vi_counter.wrapping_add(1);
}

pub fn send_input(netplay: &Netplay, input: (u32, bool)) {
    let mut request: Vec<u8> = [UDP_SEND_KEY_INFO].to_vec();
    request.push(netplay.player_number);
    request.extend_from_slice(
        &(netplay.player_data[netplay.player_number as usize].count).to_be_bytes(),
    );
    request.extend_from_slice(&(input.0).to_be_bytes());
    request.push(input.1 as u8);
    netplay.udp_socket.send(&request).unwrap();
}

pub fn get_input(device: &mut device::Device, channel: usize) -> (u32, bool) {
    let netplay = device.netplay.as_mut().unwrap();
    let mut input = None;

    let timeout = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut request_timer = std::time::Instant::now() - std::time::Duration::from_millis(5);
    while input.is_none() {
        process_incoming(netplay); // we execute process_incoming before request_input so that we send an accurate buffer count
        if std::time::Instant::now() > request_timer {
            // sends a request packet every 5ms
            request_input(netplay, channel);
            request_timer = std::time::Instant::now() + std::time::Duration::from_millis(5);
        }
        input = netplay.player_data[channel]
            .input_events
            .remove(&netplay.player_data[channel].count);

        if std::time::Instant::now() > timeout {
            send_error(
                netplay,
                "Timed out waiting for input. Lost connection to server".to_string(),
            );
            input = Some(InputEvent {
                input: 0,
                plugin: 0,
            });
            device.cpu.running = false;
        }
    }

    netplay.fast_forward = netplay.player_data[channel].lag > 0
        && netplay.player_data[channel].input_events.len() as u8 > netplay.buffer_target;

    netplay.player_data[channel].count = netplay.player_data[channel].count.wrapping_add(1);
    (
        input.as_ref().unwrap().input,
        input.as_ref().unwrap().plugin != 0,
    )
}

fn request_input(netplay: &Netplay, channel: usize) {
    let mut request: Vec<u8> = [UDP_REQUEST_KEY_INFO].to_vec();
    request.push(channel as u8); //The player we need input for
    request.extend_from_slice(
        &(netplay.player_data[netplay.player_number as usize].reg_id).to_be_bytes(),
    );
    request.extend_from_slice(&(netplay.player_data[channel].count).to_be_bytes());
    request.push(0); //spectator mode
    request.push(netplay.player_data[channel].input_events.len() as u8);
    netplay.udp_socket.send(&request).unwrap();
}

fn process_incoming(netplay: &mut Netplay) {
    let mut buf: [u8; 1024] = [0; 1024];
    while let Ok(_incoming) = netplay.udp_socket.recv(&mut buf) {
        match buf[0] {
            UDP_RECEIVE_KEY_INFO | UDP_RECEIVE_KEY_INFO_GRATUITOUS => {
                let player = buf[1] as usize;
                //current_status is a status update from the server
                //it will let us know if another player has disconnected, or the games have desynced
                let current_status = buf[2];
                if buf[0] == UDP_RECEIVE_KEY_INFO {
                    netplay.player_data[player].lag = buf[3];
                }
                if current_status != netplay.status {
                    if ((current_status & 0x1) ^ (netplay.status & 0x1)) != 0 {
                        send_error(
                            netplay,
                            format!("Players have desynced at VI {}", netplay.vi_counter),
                        );
                    }
                    for dis in 1..5 {
                        if ((current_status & (0x1 << dis)) ^ (netplay.status & (0x1 << dis))) != 0
                        {
                            send_error(netplay, format!("Player {} has disconnected", dis));
                        }
                    }
                    netplay.status = current_status;
                }

                let mut buffer_offset = 5;
                for _i in 0..buf[4] {
                    let count = u32::from_be_bytes(
                        buf[buffer_offset..buffer_offset + 4].try_into().unwrap(),
                    );
                    buffer_offset += 4;

                    if (count.wrapping_sub(netplay.player_data[player].count)) > (u32::MAX / 2) {
                        //event doesn't need to be recorded
                        buffer_offset += 5;
                        continue;
                    }

                    let input = u32::from_be_bytes(
                        buf[buffer_offset..buffer_offset + 4].try_into().unwrap(),
                    );
                    buffer_offset += 4;
                    let plugin = buf[buffer_offset];
                    buffer_offset += 1;
                    let input_event = InputEvent { input, plugin };
                    netplay.player_data[player]
                        .input_events
                        .insert(count, input_event);
                }
            }
            _ => {
                panic! {"unknown UDP packet"}
            }
        }
    }
}

pub fn init(
    mut peer_addr: std::net::SocketAddr,
    session: ui::gui::gui_netplay::NetplayRoom,
    player_number: u8,
    error_notifier: tokio::sync::mpsc::Sender<String>,
    gui_ctx: egui::Context,
) -> Netplay {
    peer_addr.set_port(session.port.unwrap() as u16);
    let udp_socket;
    if peer_addr.is_ipv4() {
        udp_socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
            .expect("couldn't bind to address");
        socket2::SockRef::from(&udp_socket)
            .set_tos(CS4 << 2)
            .unwrap();
    } else {
        udp_socket = std::net::UdpSocket::bind((std::net::Ipv6Addr::UNSPECIFIED, 0))
            .expect("couldn't bind to address");
        #[cfg(not(target_os = "windows"))]
        socket2::SockRef::from(&udp_socket)
            .set_tclass_v6(CS4 << 2)
            .unwrap();
    };

    udp_socket.connect(peer_addr).unwrap();
    udp_socket.set_nonblocking(true).unwrap();

    let mut stream = std::net::TcpStream::connect(peer_addr).unwrap();

    let regid = (player_number + 1) as u32;
    let mut request: [u8; 8] = [
        TCP_REGISTER_PLAYER,
        player_number,
        0, //plugin/pak
        0, //rawdata
        0, //regid (u32)
        0, //regid (u32)
        0, //regid (u32)
        0, //regid (u32)
    ];

    request[4..8].copy_from_slice(&regid.to_be_bytes());
    stream.write_all(&request).unwrap();

    let mut response: [u8; 2] = [0, 0];
    stream.read_exact(&mut response).unwrap();
    if response[0] != 1 {
        panic!("Failed to register player");
    }
    let buffer_target = response[1];

    let request: [u8; 1] = [TCP_GET_REGISTRATION];
    stream.write_all(&request).unwrap();
    let mut response: [u8; 24] = [0; 24];
    stream.read_exact(&mut response).unwrap();

    let mut reg_id: [u32; 4] = [0; 4];
    for i in 0..4 {
        // reg_id of 0 means no player connected
        reg_id[i] = u32::from_be_bytes(response[(i * 6)..(i * 6) + 4].try_into().unwrap());
    }
    Netplay {
        udp_socket,
        tcp_stream: stream,
        player_number,
        vi_counter: 0,
        status: 0,
        buffer_target,
        fast_forward: false,
        error_notifier,
        gui_ctx,
        player_data: [
            PlayerData {
                lag: 0,
                count: 0,
                reg_id: reg_id[0],
                input_events: std::collections::HashMap::new(),
            },
            PlayerData {
                lag: 0,
                count: 0,
                reg_id: reg_id[1],
                input_events: std::collections::HashMap::new(),
            },
            PlayerData {
                lag: 0,
                count: 0,
                reg_id: reg_id[2],
                input_events: std::collections::HashMap::new(),
            },
            PlayerData {
                lag: 0,
                count: 0,
                reg_id: reg_id[3],
                input_events: std::collections::HashMap::new(),
            },
        ],
    }
}

pub fn close(device: &mut device::Device) {
    let netplay = device.netplay.as_mut().unwrap();
    let regid = (netplay.player_number + 1) as u32;
    let mut request: [u8; 5] = [TCP_DISCONNECT_NOTICE, 0, 0, 0, 0];
    request[1..5].copy_from_slice(&regid.to_be_bytes());

    netplay.tcp_stream.write_all(&request).unwrap();
    netplay.tcp_stream.flush().unwrap();
    netplay
        .tcp_stream
        .shutdown(std::net::Shutdown::Both)
        .unwrap();
}
