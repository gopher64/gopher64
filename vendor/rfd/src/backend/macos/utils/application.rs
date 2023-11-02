use objc::{class, msg_send, sel, sel_impl};

use objc_foundation::{object_struct, INSArray, INSObject, NSArray};
use objc_id::{Id, ShareId, Shared};

use super::NSWindow;

pub trait INSApplication: INSObject {
    fn shared_application() -> Id<Self> {
        let app = unsafe { msg_send![class!(NSApplication), sharedApplication] };
        unsafe { Id::from_ptr(app) }
    }

    fn is_running(&self) -> bool {
        unsafe { msg_send![self, isRunning] }
    }

    fn key_window(&self) -> Option<Id<NSWindow>> {
        let id: *mut NSWindow = unsafe { msg_send![self, keyWindow] };
        if id.is_null() {
            None
        } else {
            Some(unsafe { Id::from_ptr(id) })
        }
    }

    fn main_window(&self) -> Option<Id<NSWindow>> {
        let id: *mut NSWindow = unsafe { msg_send![self, mainWindow] };
        if id.is_null() {
            None
        } else {
            Some(unsafe { Id::from_ptr(id) })
        }
    }

    fn windows(&self) -> Id<NSArray<NSWindow, Shared>> {
        let id = unsafe { msg_send![self, windows] };
        let id: Id<NSArray<NSWindow, Shared>> = unsafe { Id::from_ptr(id) };
        id
    }

    fn get_window(&self) -> Option<ShareId<NSWindow>> {
        if let Some(key_window) = self.main_window() {
            Some(key_window.share())
        } else {
            let windows = self.windows();
            if windows.count() > 0 {
                let window = windows.shared_object_at(0);
                Some(window)
            } else {
                None
            }
        }
    }
}

object_struct!(NSApplication);
impl INSApplication for NSApplication {}
