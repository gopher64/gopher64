use crate::device;
use crate::ui;
use std::io::{Read, Write};
//UDP packet formats
//const UDP_SEND_KEY_INFO: u8 = 0;
//const UDP_RECEIVE_KEY_INFO: u8 = 1;
//const UDP_REQUEST_KEY_INFO: u8 = 2;
//const UDP_RECEIVE_KEY_INFO_GRATUITOUS: u8 = 3;
//const UDP_SYNC_DATA: u8 = 4;

//TCP packet formats
//const TCP_SEND_SAVE: u8 = 1;
//const TCP_RECEIVE_SAVE: u8 = 2;
//const TCP_SEND_SETTINGS: u8 = 3;
//const TCP_RECEIVE_SETTINGS: u8 = 4;
const TCP_REGISTER_PLAYER: u8 = 5;
const TCP_GET_REGISTRATION: u8 = 6;
const TCP_DISCONNECT_NOTICE: u8 = 7;

pub struct Netplay {
    // udp_socket: std::net::UdpSocket,
    tcp_stream: std::net::TcpStream,
    player_number: u8,
}

pub fn init(
    mut peer_addr: std::net::SocketAddr,
    session: ui::gui::gui_netplay::NetplayRoom,
    player_number: u8,
) -> Netplay {
    peer_addr.set_port(session.port.unwrap() as u16);
    let udp_socket = if peer_addr.is_ipv4() {
        std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
            .expect("couldn't bind to address")
    } else {
        std::net::UdpSocket::bind((std::net::Ipv6Addr::UNSPECIFIED, 0))
            .expect("couldn't bind to address")
    };
    udp_socket.connect(peer_addr).unwrap();

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
    let _buffer_target = response[1];

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
        // udp_socket,
        tcp_stream: stream,
        player_number,
    }
}

pub fn close(device: &mut device::Device) {
    let netplay = device.netplay.as_mut().unwrap();
    let regid = (netplay.player_number + 1) as u32;
    let mut request: [u8; 5] = [TCP_DISCONNECT_NOTICE, 0, 0, 0, 0];
    request[1..5].copy_from_slice(&regid.to_be_bytes());

    netplay.tcp_stream.write_all(&request).unwrap();
}
