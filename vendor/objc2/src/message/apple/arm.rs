use core::mem;

use super::MsgSendFn;
use crate::ffi;
use crate::runtime::Imp;
use crate::{Encode, Encoding};

/// Double-word sized fundamental data types don't use stret, but any
/// composite type larger than 4 bytes does.
///
/// <https://web.archive.org/web/20191016000656/http://infocenter.arm.com/help/topic/com.arm.doc.ihi0042f/IHI0042F_aapcs.pdf>
/// <https://developer.arm.com/documentation/ihi0042/latest>
unsafe impl<T: Encode> MsgSendFn for T {
    const MSG_SEND: Imp = {
        if let Encoding::LongLong | Encoding::ULongLong | Encoding::Double = T::ENCODING {
            ffi::objc_msgSend
        } else if mem::size_of::<T>() <= 4 {
            ffi::objc_msgSend
        } else {
            ffi::objc_msgSend_stret
        }
    };
    const MSG_SEND_SUPER: Imp = {
        if let Encoding::LongLong | Encoding::ULongLong | Encoding::Double = T::ENCODING {
            ffi::objc_msgSendSuper
        } else if mem::size_of::<T>() <= 4 {
            ffi::objc_msgSendSuper
        } else {
            ffi::objc_msgSendSuper_stret
        }
    };
}
