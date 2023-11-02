use core::mem;

use super::MsgSendFn;
use crate::ffi;
use crate::runtime::Imp;
use crate::{Encode, Encoding};

/// Structures 1 or 2 bytes in size are placed in EAX.
/// Structures 4 or 8 bytes in size are placed in: EAX and EDX.
/// Structures of other sizes are placed at the address supplied by the caller.
///
/// <https://developer.apple.com/library/mac/documentation/DeveloperTools/Conceptual/LowLevelABI/130-IA-32_Function_Calling_Conventions/IA32.html>
unsafe impl<T: Encode> MsgSendFn for T {
    const MSG_SEND: Imp = {
        // See https://github.com/apple-oss-distributions/objc4/blob/objc4-818.2/runtime/message.h#L156-L172
        if let Encoding::Float | Encoding::Double | Encoding::LongDouble = T::ENCODING {
            ffi::objc_msgSend_fpret
        } else if let 0 | 1 | 2 | 4 | 8 = mem::size_of::<T>() {
            ffi::objc_msgSend
        } else {
            ffi::objc_msgSend_stret
        }
    };
    const MSG_SEND_SUPER: Imp = {
        if let 0 | 1 | 2 | 4 | 8 = mem::size_of::<T>() {
            ffi::objc_msgSendSuper
        } else {
            ffi::objc_msgSendSuper_stret
        }
    };
}
