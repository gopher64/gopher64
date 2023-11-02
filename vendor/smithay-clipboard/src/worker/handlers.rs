#![macro_use]

use std::fs::File;
use std::io::Result;
use std::os::unix::io::FromRawFd;
use std::sync::mpsc::Sender;

use sctk::reexports::client::protocol::wl_keyboard::Event as KeyboardEvent;
use sctk::reexports::client::protocol::wl_pointer::Event as PointerEvent;
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::DispatchData;

use super::dispatch_data::ClipboardDispatchData;

/// Macro to handle load for selection and primary clipboards.
macro_rules! handle_load {
    ($env:ident, $sel_ty:ident, $seat:ident, $queue:ident, $tx:ident ) => {
        let result = $env.$sel_ty(&$seat, |device| {
            let (mut reader, mime_type) = match device.with_selection(|offer| {
                // Check that we have an offer.
                let offer = match offer {
                    Some(offer) => offer,
                    None => return None,
                };

                // Check that we can work with advertised mime type and pick the one
                // that suits us more.
                let mime_type = match offer.with_mime_types(MimeType::find_allowed) {
                    Some(mime_type) => mime_type,
                    None => return None,
                };

                // Request given the mime type.
                let reader = offer.receive(mime_type.to_string()).ok()?;
                Some((reader, mime_type))
            }) {
                Some((reader, mime_type)) => (reader, mime_type),
                None => {
                    handlers::reply_error(&$tx, "offer receive failed.");
                    return ();
                }
            };

            // If we fail here, it means that we likely won't be able
            // to read clipboard anyway, so return and reply to prevent block/crash.
            if $queue.sync_roundtrip(&mut (), |_, _, _| unreachable!()).is_err() {
                handlers::reply_error(&$tx, "failed to access clipboard.");
                return;
            };

            let mut contents = String::new();
            let result = reader.read_to_string(&mut contents).map(|_| {
                if mime_type == MimeType::Utf8String {
                    mime::normalize_to_lf(contents)
                } else {
                    contents
                }
            });

            let _ = $tx.send(result);
        });

        // Send back that we've failed to load data from the clipboard.
        if result.is_err() {
            handlers::reply_error(&$tx, "failed to access clipboard.");
        }
    };
}

/// Macro to handle store for selection and primary clipboards.
macro_rules! handle_store {
    ($env:ident,
     $sel_source:ident, $sel_device:ident, $event_ty:ident,
     $seat:ident, $serial:ident, $queue:ident, $contents:ident) => {
        let data_source = $env.$sel_source(
            vec![MimeType::TextPlainUtf8.to_string(), MimeType::Utf8String.to_string()],
            move |event, _| {
                if let $event_ty::Send { mut pipe, .. } = event {
                    // If we fail to write here, it means that other side closed the pipe, thus
                    // we can't do anything about it.
                    let _ = write!(pipe, "{}", $contents);
                }
            },
        );

        let _ = $env.$sel_device(&$seat, |device| {
            device.set_selection(&Some(data_source), $serial);

            let _ = $queue.sync_roundtrip(&mut (), |_, _, _| unreachable!());
        });
    };
}

/// Reply an error to a clipboard master.
pub fn reply_error(tx: &Sender<Result<String>>, description: &str) {
    let _ = tx.send(Err(std::io::Error::new(std::io::ErrorKind::Other, description)));
}

/// Update seat and serial on pointer events.
pub fn pointer_handler(seat: WlSeat, event: PointerEvent, mut dispatch_data: DispatchData) {
    let dispatch_data = match dispatch_data.get::<ClipboardDispatchData>() {
        Some(dispatch_data) => dispatch_data,
        None => return,
    };
    match event {
        PointerEvent::Enter { serial, .. } => {
            dispatch_data.set_last_observed_seat(seat, serial);
        }
        PointerEvent::Button { serial, .. } => {
            dispatch_data.set_last_observed_seat(seat, serial);
        }
        _ => {}
    }
}

/// Update seat and serial on keyboard events.
pub fn keyboard_handler(seat: WlSeat, event: KeyboardEvent, mut dispatch_data: DispatchData) {
    let dispatch_data = match dispatch_data.get::<ClipboardDispatchData>() {
        Some(dispatch_data) => dispatch_data,
        None => return,
    };
    match event {
        KeyboardEvent::Enter { serial, .. } => {
            dispatch_data.set_last_observed_seat(seat, serial);
        }
        KeyboardEvent::Key { serial, .. } => {
            dispatch_data.set_last_observed_seat(seat, serial);
        }
        KeyboardEvent::Leave { .. } => {
            dispatch_data.remove_observed_seat(&seat);
        }
        KeyboardEvent::Keymap { fd, .. } => {
            // Prevent fd leaking.
            let _ = unsafe { File::from_raw_fd(fd) };
        }
        _ => {}
    }
}
