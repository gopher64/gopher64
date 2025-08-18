use crate::ui;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const DATATYPE_TCPTEST: u32 = 0x07;
const DATATYPE_ROMUPLOAD: u32 = 0x08;

#[derive(Clone, Debug)]
pub struct UsbData {
    pub data: Vec<u8>,
    pub data_type: u32,
    pub data_size: u32,
}

fn respond_to_handshake(usb_tx: &tokio::sync::broadcast::Sender<UsbData>, data: Vec<u8>) {
    if let Ok(data) = String::from_utf8(data)
        && data == "N64"
    {
        ui::usb::send_to_usb(
            usb_tx,
            ui::usb::UsbData {
                data: [b'N', b'6', b'4'].to_vec(),
                data_type: DATATYPE_TCPTEST,
                data_size: 3,
            },
        );
    }
}

fn upload_rom(
    weak: slint::Weak<ui::gui::AppWindow>,
    rom: Vec<u8>,
    usb_tx: &tokio::sync::broadcast::Sender<UsbData>,
    cart_rx: &tokio::sync::broadcast::Receiver<UsbData>,
) {
    let weak_clone = weak.clone();
    let usb_tx = usb_tx.clone();
    let cart_rx = cart_rx.resubscribe();

    weak.upgrade_in_event_loop(move |handle| {
        let dir = std::env::temp_dir();
        let rom_path = dir.join("rom_upload.bin");
        std::fs::write(rom_path.clone(), rom).unwrap();

        ui::gui::run_rom(
            ui::gui::GbPaths {
                rom: [None, None, None, None],
                ram: [None, None, None, None],
            },
            rom_path,
            ui::gui::GameSettings {
                fullscreen: handle.get_fullscreen(),
                overclock: handle.get_overclock_n64_cpu(),
                disable_expansion_pak: handle.get_disable_expansion_pak(),
                cheats: std::collections::HashMap::new(), // will be filled in later
            },
            ui::gui::VruChannel {
                vru_window_notifier: None,
                vru_word_receiver: None,
            },
            None,
            ui::Usb {
                usb_tx: Some(usb_tx),
                cart_rx: Some(cart_rx),
            },
            weak_clone,
        );
    })
    .unwrap();
}

async fn handle_connection(
    conn: tokio::net::TcpStream,
    mut shutdown_rx: tokio::sync::watch::Receiver<()>,
    mut usb_rx: tokio::sync::broadcast::Receiver<UsbData>,
    usb_tx: tokio::sync::broadcast::Sender<UsbData>,
    cart_tx: tokio::sync::broadcast::Sender<UsbData>,
    cart_rx: tokio::sync::broadcast::Receiver<UsbData>,
    weak: slint::Weak<ui::gui::AppWindow>,
) {
    let (mut incoming, mut outgoing) = conn.into_split();

    let mut shutdown_rx_clone = shutdown_rx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                data = usb_rx.recv() => {
                    match data {
                        Ok(data) => {
                            let mut output: Vec<u8> = vec![];
                            output.extend_from_slice(&data.data_type.to_be_bytes());
                            output.extend_from_slice(&data.data_size.to_be_bytes());
                            output.extend_from_slice(&data.data);
                            if outgoing.write_all(&output).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            panic!("usb_rx lagged");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                          break;
                        }
                    }
                }
                _ = shutdown_rx_clone.changed() => {
                    break;
                }
            }
        }
    });

    let mut incoming_buffer = vec![0u8; 4096];
    let mut data_type: Option<u32> = None;
    let mut data_size: Option<u32> = None;
    let mut usb_buffer: Vec<u8> = vec![];
    loop {
        tokio::select! {
            result = incoming.read(&mut incoming_buffer) => {
                match result {
                    Ok(0) => {
                        break;
                    }
                    Ok(n) => {
                        usb_buffer.extend_from_slice(&incoming_buffer[0..n]);
                        if data_type.is_none() {
                            if usb_buffer.len() < 4 {
                                continue;
                            } else {
                                data_type = Some(u32::from_be_bytes(usb_buffer[0..4].try_into().unwrap()));
                                usb_buffer.drain(0..4);
                            }
                        }
                        if data_type.is_some() && data_size.is_none() {
                            if usb_buffer.len() < 4 {
                                continue;
                            } else {
                                data_size = Some(u32::from_be_bytes(usb_buffer[0..4].try_into().unwrap()));
                                usb_buffer.drain(0..4);
                            }
                        }
                        if let Some(d_type) = data_type && let Some(d_size) = data_size {
                            let length = d_size as usize;
                            if usb_buffer.len() >= length {
                                let usb_data = UsbData {
                                    data: usb_buffer[0..length].to_vec(),
                                    data_type: d_type,
                                    data_size: d_size,
                                };
                                usb_buffer.drain(0..length);
                                if usb_data.data_type == DATATYPE_TCPTEST {
                                    respond_to_handshake(&usb_tx,usb_data.data);
                                } else if usb_data.data_type == DATATYPE_ROMUPLOAD {
                                    upload_rom(weak.clone(), usb_data.data,&usb_tx,&cart_rx);
                                } else {
                                    cart_tx.send(usb_data).unwrap();
                                }
                                data_type = None;
                                data_size = None;
                            }
                        }
                    }
                    Err(_e) => {
                        break;
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                break;
            }
        }
    }
}

pub fn init(
    weak: slint::Weak<ui::gui::AppWindow>,
) -> (Option<tokio::sync::watch::Sender<()>>, ui::Usb) {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(());
    let (usb_tx, usb_rx): (
        tokio::sync::broadcast::Sender<UsbData>,
        tokio::sync::broadcast::Receiver<UsbData>,
    ) = tokio::sync::broadcast::channel(1024);
    let (cart_tx, cart_rx): (
        tokio::sync::broadcast::Sender<UsbData>,
        tokio::sync::broadcast::Receiver<UsbData>,
    ) = tokio::sync::broadcast::channel(1024);

    let usb_tx_clone = usb_tx.clone();
    let cart_rx_clone = cart_rx.resubscribe();
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("localhost:64000")
            .await
            .unwrap();

        loop {
            tokio::select! {
                res = listener.accept() => {
                    if let Ok((c,_)) = res {
                        handle_connection(c,shutdown_rx.clone(),usb_rx.resubscribe(),usb_tx.clone(),cart_tx.clone(),cart_rx.resubscribe(),weak.clone()).await;
                    } else {
                        break;
                    }
                }
                _ = shutdown_rx.changed() => {
                    break;
                }
            }
        }
    });
    (
        Some(shutdown_tx),
        ui::Usb {
            usb_tx: Some(usb_tx_clone),
            cart_rx: Some(cart_rx_clone),
        },
    )
}

pub fn close(shutdown_tx: &tokio::sync::watch::Sender<()>) {
    let _ = shutdown_tx.send(());
}

pub fn send_to_usb(usb_tx: &tokio::sync::broadcast::Sender<UsbData>, buffer: UsbData) {
    usb_tx.send(buffer).unwrap();
}
