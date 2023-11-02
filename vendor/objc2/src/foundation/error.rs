use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{NSCopying, NSDictionary, NSObject, NSString};
use crate::ffi::NSInteger;
use crate::rc::{Id, Shared};
use crate::{extern_class, extern_methods, msg_send_id, ClassType};

extern_class!(
    /// Information about an error condition including a domain, a
    /// domain-specific error code, and application-specific information.
    ///
    /// See also Apple's [documentation on error handling][err], and their
    /// NSError [API reference][api].
    ///
    /// [err]: https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ErrorHandlingCocoa/ErrorHandling/ErrorHandling.html#//apple_ref/doc/uid/TP40001806-CH201-SW1
    /// [api]: https://developer.apple.com/documentation/foundation/nserror?language=objc
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSError;

    unsafe impl ClassType for NSError {
        type Super = NSObject;
    }
);

// SAFETY: Error objects are immutable data containers.
unsafe impl Sync for NSError {}
unsafe impl Send for NSError {}

impl UnwindSafe for NSError {}
impl RefUnwindSafe for NSError {}

pub type NSErrorUserInfoKey = NSString;
pub type NSErrorDomain = NSString;

extern_methods!(
    /// Creation methods.
    unsafe impl NSError {
        /// Construct a new [`NSError`] with the given code in the given domain.
        pub fn new(code: NSInteger, domain: &NSString) -> Id<Self, Shared> {
            unsafe { Self::with_user_info(code, domain, None) }
        }

        // TODO: Figure out safety of `user_info` dict!
        unsafe fn with_user_info(
            code: NSInteger,
            domain: &NSString,
            user_info: Option<&NSDictionary<NSErrorUserInfoKey, NSObject>>,
        ) -> Id<Self, Shared> {
            // SAFETY: `domain` and `user_info` are copied to the error object, so
            // even if the `&NSString` came from a `&mut NSMutableString`, we're
            // still good!
            unsafe {
                msg_send_id![
                    msg_send_id![Self::class(), alloc],
                    initWithDomain: domain,
                    code: code,
                    userInfo: user_info,
                ]
            }
        }
    }

    /// Accessor methods.
    unsafe impl NSError {
        pub fn domain(&self) -> Id<NSString, Shared> {
            unsafe { msg_send_id![self, domain] }
        }

        #[sel(code)]
        pub fn code(&self) -> NSInteger;

        pub fn user_info(&self) -> Option<Id<NSDictionary<NSErrorUserInfoKey, NSObject>, Shared>> {
            unsafe { msg_send_id![self, userInfo] }
        }

        pub fn localized_description(&self) -> Id<NSString, Shared> {
            // TODO: For some reason this leaks a lot?
            let obj: Option<_> = unsafe { msg_send_id![self, localizedDescription] };
            obj.expect(
                "unexpected NULL localized description; a default should have been generated!",
            )
        }

        // TODO: localizedRecoveryOptions
        // TODO: localizedRecoverySuggestion
        // TODO: localizedFailureReason
        // TODO: helpAnchor
        // TODO: +setUserInfoValueProviderForDomain:provider:
        // TODO: +userInfoValueProviderForDomain:

        // TODO: recoveryAttempter
        // TODO: attemptRecoveryFromError:...

        // TODO: Figure out if this is a good design, or if we should do something
        // differently (like a Rusty name for the function, or putting a bunch of
        // statics in a module instead)?
        #[allow(non_snake_case)]
        pub fn NSLocalizedDescriptionKey() -> &'static NSErrorUserInfoKey {
            extern "C" {
                #[link_name = "NSLocalizedDescriptionKey"]
                static VALUE: &'static NSErrorUserInfoKey;
            }
            unsafe { VALUE }
        }

        // TODO: Other NSErrorUserInfoKey values
        // TODO: NSErrorDomain values
    }
);

impl fmt::Debug for NSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NSError")
            .field("domain", &self.domain())
            .field("code", &self.code())
            .field("user_info", &self.user_info())
            .finish()
    }
}

impl fmt::Display for NSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.localized_description())
    }
}

impl std::error::Error for NSError {}

unsafe impl NSCopying for NSError {
    type Ownership = Shared;
    type Output = Self;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    use crate::ns_string;

    #[test]
    fn custom_domain() {
        let error = NSError::new(42, ns_string!("MyDomain"));
        assert_eq!(error.code(), 42);
        assert_eq!(&*error.domain(), ns_string!("MyDomain"));
        let expected = if cfg!(feature = "apple") {
            "The operation couldn’t be completed. (MyDomain error 42.)"
        } else {
            "MyDomain 42"
        };
        assert_eq!(format!("{}", error), expected);
    }

    #[test]
    fn basic() {
        let error = NSError::new(-999, ns_string!("NSURLErrorDomain"));
        let expected = if cfg!(feature = "apple") {
            "The operation couldn’t be completed. (NSURLErrorDomain error -999.)"
        } else {
            "NSURLErrorDomain -999"
        };
        assert_eq!(format!("{}", error), expected);
    }
}
