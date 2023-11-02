use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{NSCopying, NSDictionary, NSObject, NSString};
use crate::rc::{Id, Shared};
use crate::{extern_class, extern_methods, msg_send_id, ns_string, ClassType};

extern_class!(
    /// A representation of the code and resources stored in a bundle
    /// directory on disk.
    ///
    /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsbundle?language=objc).
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSBundle;

    unsafe impl ClassType for NSBundle {
        type Super = NSObject;
    }
);

// SAFETY: Bundles are documented as thread-safe.
unsafe impl Sync for NSBundle {}
unsafe impl Send for NSBundle {}

impl UnwindSafe for NSBundle {}
impl RefUnwindSafe for NSBundle {}

extern_methods!(
    unsafe impl NSBundle {
        pub fn main() -> Id<Self, Shared> {
            unsafe { msg_send_id![Self::class(), mainBundle] }
        }

        pub fn info(&self) -> Id<NSDictionary<NSString, NSObject>, Shared> {
            unsafe { msg_send_id![self, infoDictionary] }
        }

        pub fn name(&self) -> Option<Id<NSString, Shared>> {
            self.info().get(ns_string!("CFBundleName")).map(|name| {
                let ptr: *const NSObject = name;
                let ptr: *const NSString = ptr.cast();
                // SAFETY: TODO
                let name = unsafe { ptr.as_ref().unwrap_unchecked() };
                name.copy()
            })
        }
    }
);

impl fmt::Debug for NSBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to NSObject
        (**self).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use std::println;

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore = "varies between platforms")]
    fn try_running_functions() {
        // This is mostly empty since cargo doesn't bundle the application
        // before executing.
        let bundle = NSBundle::main();
        println!("{:?}", bundle);
        assert_eq!(format!("{:?}", bundle.info()), "{}");
        assert_eq!(bundle.name(), None);
    }
}
