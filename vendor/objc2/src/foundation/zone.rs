use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};

use crate::ffi;
#[cfg(feature = "gnustep-1-7")]
use crate::Encode;
use crate::{Encoding, RefEncode};

/// A type used to identify and manage memory zones.
///
/// Zones are ignored on all newer platforms, you should very rarely need to
/// use this, but may be useful if you need to implement `copyWithZone:` or
/// `allocWithZone:`.
///
/// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nszone?language=objc).
pub struct NSZone {
    // Use `objc_object` to mark the types as !Send, !Sync and UnsafeCell.
    //
    // This works since `objc_object` is a ZST
    _inner: ffi::objc_object,
}

impl fmt::Debug for NSZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<NSZone {:p}>", self)
    }
}

// Note: We don't know anything about the internals of `NSZone`, so best not
// to make it `Send` and `Sync` for now.

impl UnwindSafe for NSZone {}
impl RefUnwindSafe for NSZone {}

unsafe impl RefEncode for NSZone {
    #[cfg(feature = "apple")]
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("_NSZone", &[]));
    #[cfg(feature = "gnustep-1-7")]
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct(
        "_NSZone",
        &[
            // Functions
            Encoding::Pointer(&Encoding::Unknown),
            Encoding::Pointer(&Encoding::Unknown),
            Encoding::Pointer(&Encoding::Unknown),
            Encoding::Pointer(&Encoding::Unknown),
            Encoding::Pointer(&Encoding::Unknown),
            Encoding::Pointer(&Encoding::Unknown),
            // Stats
            Encoding::Pointer(&Encoding::Unknown),
            // Zone granularity
            usize::ENCODING,
            // Name of zone
            Encoding::Object,
            // Next zone - note that the contents of this doesn't matter,
            // since this is nested far enough that the encoding string just
            // ends up ignoring it.
            Encoding::Pointer(&Encoding::Struct("_NSZone", &[])),
        ],
    ));
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use core::ptr;

    use super::*;
    use crate::foundation::NSObject;
    use crate::msg_send_id;
    use crate::rc::{Allocated, Id, Owned};
    use crate::ClassType;

    #[test]
    fn alloc_with_zone() {
        let zone: *const NSZone = ptr::null();
        let _obj: Id<Allocated<NSObject>, Owned> =
            unsafe { msg_send_id![NSObject::class(), allocWithZone: zone] };
    }

    #[test]
    fn verify_encoding() {
        let expected = if cfg!(all(feature = "gnustep-1-7", target_pointer_width = "64")) {
            "^{_NSZone=^?^?^?^?^?^?^?Q@^{_NSZone}}"
        } else if cfg!(all(
            feature = "gnustep-1-7",
            not(target_pointer_width = "64")
        )) {
            "^{_NSZone=^?^?^?^?^?^?^?I@^{_NSZone}}"
        } else {
            "^{_NSZone=}"
        };
        assert_eq!(NSZone::ENCODING_REF.to_string(), expected);
    }
}
