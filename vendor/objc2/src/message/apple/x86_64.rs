use core::mem;

use super::MsgSendFn;
use crate::ffi;
use crate::runtime::Imp;
use crate::{Encode, Encoding};

/// If the size of an object is larger than two eightbytes, it has class
/// MEMORY. If the type has class MEMORY, then the caller provides space for
/// the return value and passes the address of this storage.
///
/// <https://www.uclibc.org/docs/psABI-x86_64.pdf>
unsafe impl<T: Encode> MsgSendFn for T {
    const MSG_SEND: Imp = {
        // See https://github.com/apple-oss-distributions/objc4/blob/objc4-818.2/runtime/message.h#L156-L172
        if let Encoding::LongDouble = T::ENCODING {
            ffi::objc_msgSend_fpret
        } else if let Encoding::LongDoubleComplex = T::ENCODING {
            ffi::objc_msgSend_fp2ret
        } else if mem::size_of::<T>() <= 16 {
            ffi::objc_msgSend
        } else {
            ffi::objc_msgSend_stret
        }
    };
    const MSG_SEND_SUPER: Imp = {
        if mem::size_of::<T>() <= 16 {
            ffi::objc_msgSendSuper
        } else {
            ffi::objc_msgSendSuper_stret
        }
    };
}
