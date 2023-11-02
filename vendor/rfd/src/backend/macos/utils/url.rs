use std::path::PathBuf;

use objc::{class, msg_send, sel, sel_impl};

use objc_foundation::{object_struct, INSObject, INSString, NSString};
use objc_id::Id;

pub trait INSURL: INSObject {
    fn file_url_with_path(s: &str, is_dir: bool) -> Id<Self> {
        let s = NSString::from_str(s);
        let ptr = unsafe { msg_send![class!(NSURL), fileURLWithPath: s isDirectory:is_dir] };
        unsafe { Id::from_retained_ptr(ptr) }
    }

    fn to_path_buf(&self) -> PathBuf {
        let s = unsafe { msg_send![self, path] };
        let s: Id<NSString> = unsafe { Id::from_ptr(s) };
        s.as_str().into()
    }
}

object_struct!(NSURL);
impl INSURL for NSURL {}
