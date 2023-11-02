use objc::{msg_send, sel, sel_impl};
use objc_id::Id;

use super::nil;
use objc_foundation::{object_struct, INSObject};

use raw_window_handle::RawWindowHandle;

pub trait INSWindow: INSObject {
    fn from_raw_window_handle(h: &RawWindowHandle) -> Id<Self> {
        match h {
            RawWindowHandle::AppKit(h) => {
                let id = h.ns_window as *mut Self;
                unsafe { Id::from_ptr(id) }
            }
            _ => unreachable!("unsupported window handle, expected: MacOS"),
        }
    }

    fn make_key_and_order_front(&self) {
        let _: () = unsafe { msg_send![self, makeKeyAndOrderFront: nil] };
    }
}

object_struct!(NSWindow);
impl INSWindow for NSWindow {}
