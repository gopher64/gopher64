use crate::device;
use crate::ui;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

pub struct Netplay {
    pub player_number: usize,
    pub connected: [bool; 4],
    pub socket: tokio_tungstenite::tungstenite::WebSocket<
        tokio_tungstenite::tungstenite::stream::MaybeTlsStream<std::net::TcpStream>,
    >,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum MessageType {
    Authenticate,
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
    netplay
        .socket
        .send(tokio_tungstenite::tungstenite::Message::Binary(data.into()))
        .unwrap();
}

fn receive_message(netplay: &mut Netplay) -> NetplayMessage {
    let message = netplay.socket.read().unwrap();
    let message = postcard::from_bytes::<NetplayMessage>(&message.into_data()).unwrap();
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

pub fn init(_session_name: String, player_number: usize) -> Netplay {
    let (mut socket, response) =
        tokio_tungstenite::tungstenite::connect("ws://localhost:45000").expect("Can't connect");

    let status = response.status();
    if status != StatusCode::OK {
        ui::video::onscreen_message(
            "Failed to connect to netplay server",
            ui::video::MESSAGE_LENGTH_MESSAGE_LONG,
        );
    } else {
        let authenticate = NetplayMessage {
            message_type: MessageType::Authenticate,
            name: "authenticate".to_string(),
            data: ui::netplay::get_auth_token().into(),
        };
        let data = postcard::to_stdvec(&authenticate).unwrap();
        socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(data.into()))
            .unwrap();
    }
    Netplay {
        player_number,
        connected: [false; 4],
        socket,
    }
}

pub fn close(device: &mut device::Device) {
    device
        .netplay
        .as_mut()
        .unwrap()
        .socket
        .close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: Utf8Bytes::from(""),
        }))
        .unwrap();
}
