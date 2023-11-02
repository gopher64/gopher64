use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{NSCopying, NSObject, NSString};
use crate::rc::{DefaultId, Id, Shared};
use crate::{extern_class, extern_methods, msg_send_id, ClassType, Encode, Encoding, RefEncode};

extern_class!(
    /// A universally unique value.
    ///
    /// Can be used to identify types, interfaces, and other items.
    ///
    /// Conversion methods to/from UUIDs from the `uuid` crate can be
    /// enabled with the `uuid` crate feature.
    ///
    /// macOS: This is only available on 10.8 and above.
    ///
    /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsuuid?language=objc).
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSUUID;

    unsafe impl ClassType for NSUUID {
        type Super = NSObject;
    }
);

/// The headers describe `initWithUUIDBytes:` and `getUUIDBytes:` as
/// taking `uuid_t`, but something fishy is going on, in reality they
/// expect a reference to these!
///
/// Hence we create this newtype to change the encoding.
#[repr(transparent)]
struct UuidBytes([u8; 16]);

unsafe impl RefEncode for UuidBytes {
    const ENCODING_REF: Encoding = Encoding::Array(16, &u8::ENCODING);
}

// SAFETY: `NSUUID` is immutable.
unsafe impl Sync for NSUUID {}
unsafe impl Send for NSUUID {}

impl UnwindSafe for NSUUID {}
impl RefUnwindSafe for NSUUID {}

extern_methods!(
    unsafe impl NSUUID {
        pub fn new_v4() -> Id<Self, Shared> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        /// The 'nil UUID'.
        pub fn nil() -> Id<Self, Shared> {
            Self::from_bytes([0; 16])
        }

        pub fn from_bytes(bytes: [u8; 16]) -> Id<Self, Shared> {
            let bytes = UuidBytes(bytes);
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithUUIDBytes: &bytes]
            }
        }

        pub fn from_string(string: &NSString) -> Option<Id<Self, Shared>> {
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithUUIDString: string]
            }
        }

        #[sel(getUUIDBytes:)]
        fn get_bytes_raw(&self, bytes: &mut UuidBytes);

        pub fn as_bytes(&self) -> [u8; 16] {
            let mut bytes = UuidBytes([0; 16]);
            self.get_bytes_raw(&mut bytes);
            bytes.0
        }

        pub fn string(&self) -> Id<NSString, Shared> {
            unsafe { msg_send_id![self, UUIDString] }
        }
    }
);

impl fmt::Display for NSUUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.string(), f)
    }
}

impl fmt::Debug for NSUUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The `uuid` crate does `Debug` and `Display` equally, and so do we
        fmt::Display::fmt(&self.string(), f)
    }
}

// UUID `compare:` is broken for some reason?

// impl PartialOrd for NSUUID {
//     #[inline]
//     fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
//         Some(self.cmp(other))
//     }
// }

// impl Ord for NSUUID {
//     fn cmp(&self, other: &Self) -> cmp::Ordering {
//         let res: NSComparisonResult = unsafe { msg_send![self, compare: other] };
//         res.into()
//     }
// }

/// Conversion methods to/from `uuid` crate.
#[cfg(feature = "uuid")]
impl NSUUID {
    pub fn from_uuid(uuid: uuid::Uuid) -> Id<Self, Shared> {
        Self::from_bytes(uuid.into_bytes())
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        uuid::Uuid::from_bytes(self.as_bytes())
    }
}

impl DefaultId for NSUUID {
    type Ownership = Shared;
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::nil()
    }
}

unsafe impl NSCopying for NSUUID {
    type Ownership = Shared;
    type Output = NSUUID;
}

impl alloc::borrow::ToOwned for NSUUID {
    type Owned = Id<NSUUID, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn test_new() {
        let uuid1 = NSUUID::new_v4();
        let uuid2 = NSUUID::new_v4();
        assert_ne!(uuid1, uuid2, "Statistically very unlikely");
    }

    #[test]
    fn test_bytes() {
        let uuid = NSUUID::from_bytes([10; 16]);
        assert_eq!(uuid.as_bytes(), [10; 16]);
    }

    #[test]
    fn display_debug() {
        let uuid = NSUUID::from_bytes([10; 16]);
        let expected = "0A0A0A0A-0A0A-0A0A-0A0A-0A0A0A0A0A0A";
        assert_eq!(format!("{}", uuid), expected);
        assert_eq!(format!("{:?}", uuid), expected);
    }

    // #[test]
    // fn test_compare() {
    //     let uuid1 = NSUUID::from_bytes([10; 16]);
    //     let uuid2 = NSUUID::from_bytes([9; 16]);
    //     assert!(uuid1 > uuid2);
    // }

    #[cfg(feature = "uuid")]
    #[test]
    fn test_convert_roundtrip() {
        let nsuuid1 = NSUUID::new_v4();
        let uuid = nsuuid1.as_uuid();
        let nsuuid2 = NSUUID::from_uuid(uuid);
        assert_eq!(nsuuid1, nsuuid2);
    }
}
