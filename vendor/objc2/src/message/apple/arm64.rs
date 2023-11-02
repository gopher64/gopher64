use super::MsgSendFn;
use crate::ffi;
use crate::runtime::Imp;
use crate::Encode;

/// `objc_msgSend_stret` is not even available in arm64.
///
/// <https://twitter.com/gparker/status/378079715824660480>
unsafe impl<T: Encode> MsgSendFn for T {
    const MSG_SEND: Imp = ffi::objc_msgSend;
    const MSG_SEND_SUPER: Imp = ffi::objc_msgSendSuper;
}
