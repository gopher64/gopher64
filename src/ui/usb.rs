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

fn respond_to_handshake(usb_tx: &tokio::sync::mpsc::UnboundedSender<UsbData>, data: Vec<u8>) {
    if let Ok(data) = String::from_utf8(data)
        && data == "N64"
    {
        ui::usb::send_to_usb(
            usb_tx,
            ui::usb::UsbData {
                data: b"N64".to_vec(),
                data_type: DATATYPE_TCPTEST,
                data_size: 3,
            },
        );
    }
}

async fn handle_connection(
    conn: tokio::net::TcpStream,
    mut shutdown_rx: tokio::sync::watch::Receiver<()>,
    usb_rx: &mut tokio::sync::mpsc::UnboundedReceiver<UsbData>,
    usb_tx: tokio::sync::mpsc::UnboundedSender<UsbData>,
    cart_tx: tokio::sync::mpsc::UnboundedSender<UsbData>,
) {
    let (mut incoming, mut outgoing) = conn.into_split();

    while usb_rx.try_recv().is_ok() {} // drain stale USB messages

    let mut reader = tokio::spawn(async move {
        let mut incoming_buffer = vec![0u8; 4096];
        let mut usb_buffer: Vec<u8> = vec![];
        loop {
            match incoming.read(&mut incoming_buffer).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    usb_buffer.extend_from_slice(&incoming_buffer[0..n]);
                    // A single read can contain several packets, handle all complete ones
                    while usb_buffer.len() >= 8 {
                        let d_type = u32::from_be_bytes(usb_buffer[0..4].try_into().unwrap());
                        let d_size = u32::from_be_bytes(usb_buffer[4..8].try_into().unwrap());
                        let length = d_size as usize;
                        if usb_buffer.len() < 8 + length {
                            break;
                        }
                        let data: Vec<u8> = usb_buffer.drain(..8 + length).skip(8).collect();
                        match d_type {
                            DATATYPE_TCPTEST => respond_to_handshake(&usb_tx, data),
                            DATATYPE_ROMUPLOAD => {} // The ROM is booted by the user in the emulator, ignore uploads
                            _ => {
                                let _ = cart_tx.send(UsbData {
                                    data,
                                    data_type: d_type,
                                    data_size: d_size,
                                });
                            }
                        }
                    }
                }
            }
        }
    });

    loop {
        tokio::select! {
            data = usb_rx.recv() => {
                match data {
                    Some(data) => {
                        let mut output: Vec<u8> = vec![];
                        output.extend_from_slice(&data.data_type.to_be_bytes());
                        output.extend_from_slice(&data.data_size.to_be_bytes());
                        output.extend_from_slice(&data.data);
                        if outgoing.write_all(&output).await.is_err() {
                            break;
                        }
                    }
                    None => {
                      break;
                    }
                }
            }
            _ = &mut reader => {
                // client disconnected
                break;
            }
            _ = shutdown_rx.changed() => {
                break;
            }
        }
    }
    reader.abort();
}

pub fn init() -> (
    Option<tokio::sync::watch::Sender<()>>,
    Option<tokio::task::JoinHandle<()>>,
    ui::Usb,
) {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(());
    let (usb_tx, mut usb_rx): (
        tokio::sync::mpsc::UnboundedSender<UsbData>,
        tokio::sync::mpsc::UnboundedReceiver<UsbData>,
    ) = tokio::sync::mpsc::unbounded_channel();
    let (cart_tx, cart_rx): (
        tokio::sync::mpsc::UnboundedSender<UsbData>,
        tokio::sync::mpsc::UnboundedReceiver<UsbData>,
    ) = tokio::sync::mpsc::unbounded_channel();

    let usb_tx_clone = usb_tx.clone();
    let handle = tokio::spawn(async move {
        if let Ok(listener) = tokio::net::TcpListener::bind("localhost:48646").await {
            loop {
                tokio::select! {
                    res = listener.accept() => {
                        if let Ok((c,_)) = res {
                            handle_connection(c,shutdown_rx.clone(),&mut usb_rx,usb_tx.clone(),cart_tx.clone()).await;
                        } else {
                            break;
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        break;
                    }
                }
            }
        } else {
            eprintln!("Could not bind to port 48646");
        }
    });
    (
        Some(shutdown_tx),
        Some(handle),
        ui::Usb {
            usb_tx: Some(usb_tx_clone),
            cart_rx: Some(cart_rx),
        },
    )
}

pub fn close(
    shutdown_tx: Option<tokio::sync::watch::Sender<()>>,
    usb_handle: Option<tokio::task::JoinHandle<()>>,
) {
    if let Some(mut handle) = usb_handle
        && let Some(shutdown_tx) = shutdown_tx
    {
        let _ = shutdown_tx.send(());
        tokio::task::spawn(async move {
            if tokio::time::timeout(std::time::Duration::from_secs(1), &mut handle)
                .await
                .is_err()
            {
                handle.abort();
            }
        });
    }
}

pub fn send_to_usb(usb_tx: &tokio::sync::mpsc::UnboundedSender<UsbData>, buffer: UsbData) {
    usb_tx.send(buffer).unwrap();
}
