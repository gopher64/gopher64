use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{NSObject, NSString};
use crate::rc::{Id, Shared};
use crate::{extern_class, extern_methods, msg_send_id, ClassType};

extern_class!(
    /// A collection of information about the current process.
    ///
    /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsprocessinfo?language=objc).
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSProcessInfo;

    unsafe impl ClassType for NSProcessInfo {
        type Super = NSObject;
    }
);

// SAFETY: The documentation explicitly states:
// > NSProcessInfo is thread-safe in macOS 10.7 and later.
unsafe impl Send for NSProcessInfo {}
unsafe impl Sync for NSProcessInfo {}

impl UnwindSafe for NSProcessInfo {}
impl RefUnwindSafe for NSProcessInfo {}

extern_methods!(
    unsafe impl NSProcessInfo {
        pub fn process_info() -> Id<NSProcessInfo, Shared> {
            unsafe { msg_send_id![Self::class(), processInfo] }
        }

        pub fn process_name(&self) -> Id<NSString, Shared> {
            unsafe { msg_send_id![self, processName] }
        }

        // TODO: This contains a lot more important functionality!
    }
);

impl fmt::Debug for NSProcessInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NSProcessInfo")
            .field("process_name", &self.process_name())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn test_debug() {
        let info = NSProcessInfo::process_info();
        let expected = format!(
            "NSProcessInfo {{ process_name: {:?}, .. }}",
            info.process_name()
        );
        assert_eq!(format!("{:?}", info), expected);
    }
}
