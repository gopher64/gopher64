use core::fmt;
use core::hint::unreachable_unchecked;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{NSCopying, NSDictionary, NSObject, NSString};
use crate::exception::Exception;
use crate::rc::{Id, Shared};
use crate::runtime::Object;
use crate::{extern_class, extern_methods, msg_send_id, sel, ClassType};

extern_class!(
    /// A special condition that interrupts the normal flow of program
    /// execution.
    ///
    /// Exceptions can be thrown and caught using the `objc2::exception`
    /// module.
    ///
    /// See also [Apple's documentation][doc].
    ///
    /// [doc]: https://developer.apple.com/documentation/foundation/nsexception?language=objc
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSException;

    unsafe impl ClassType for NSException {
        type Super = NSObject;
    }
);

// SAFETY: Exception objects are immutable data containers, and documented as
// thread safe.
unsafe impl Sync for NSException {}
unsafe impl Send for NSException {}

impl UnwindSafe for NSException {}
impl RefUnwindSafe for NSException {}

type NSExceptionName = NSString;
extern_methods!(
    unsafe impl NSException {
        /// Create a new [`NSException`] object.
        ///
        /// Returns `None` if the exception couldn't be created (example: If the
        /// process is out of memory).
        pub fn new(
            name: &NSExceptionName,
            reason: Option<&NSString>,
            user_info: Option<&NSDictionary<Object, Object>>,
        ) -> Option<Id<Self, Shared>> {
            let obj = unsafe { msg_send_id![Self::class(), alloc] };
            unsafe { msg_send_id![obj, initWithName: name, reason: reason, userInfo: user_info] }
        }

        #[sel(raise)]
        unsafe fn raise_raw(&self);

        /// Raises the exception, causing program flow to jump to the local
        /// exception handler.
        ///
        /// This is equivalent to using `objc2::exception::throw`.
        ///
        ///
        /// # Safety
        ///
        /// Same as `objc2::exception::throw`.
        pub unsafe fn raise(&self) -> ! {
            // SAFETY: We only create `Shared` NSExceptions, so it is safe to give
            // to the place where `@catch` receives it.
            unsafe { self.raise_raw() };
            // SAFETY: `raise` will throw an exception, or abort if something
            // unexpected happened.
            unsafe { unreachable_unchecked() }
        }

        /// A that uniquely identifies the type of exception.
        ///
        /// See [Apple's documentation][doc] for some of the different values this
        /// can take.
        ///
        /// [doc]: https://developer.apple.com/documentation/foundation/nsexceptionname?language=objc
        pub fn name(&self) -> Id<NSExceptionName, Shared> {
            // Nullability not documented, but a name is expected in most places.
            unsafe { msg_send_id![self, name] }
        }

        /// A human-readable message summarizing the reason for the exception.
        pub fn reason(&self) -> Option<Id<NSString, Shared>> {
            unsafe { msg_send_id![self, reason] }
        }

        /// Application-specific data pertaining to the exception.
        pub fn user_info(&self) -> Option<Id<NSDictionary<Object, Object>, Shared>> {
            unsafe { msg_send_id![self, userInfo] }
        }

        /// Convert this into an [`Exception`] object.
        pub fn into_exception(this: Id<Self, Shared>) -> Id<Exception, Shared> {
            // SAFETY: Downcasting to "subclass"
            unsafe { Id::cast(this) }
        }

        pub(crate) fn is_nsexception(obj: &Exception) -> bool {
            if obj.class().responds_to(sel!(isKindOfClass:)) {
                // SAFETY: We only use `isKindOfClass:` on NSObject
                let obj: *const Exception = obj;
                let obj = unsafe { obj.cast::<NSObject>().as_ref().unwrap() };
                obj.is_kind_of::<Self>()
            } else {
                false
            }
        }

        /// Create this from an [`Exception`] object.
        ///
        /// This should be considered a hint; it may return `Err` in very, very
        /// few cases where the object is actually an instance of `NSException`.
        pub fn from_exception(
            obj: Id<Exception, Shared>,
        ) -> Result<Id<Self, Shared>, Id<Exception, Shared>> {
            if Self::is_nsexception(&obj) {
                // SAFETY: Just checked the object is an NSException
                Ok(unsafe { Id::cast::<Self>(obj) })
            } else {
                Err(obj)
            }
        }
    }
);

unsafe impl NSCopying for NSException {
    type Ownership = Shared;
    type Output = NSException;
}

impl alloc::borrow::ToOwned for NSException {
    type Owned = Id<NSException, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

impl fmt::Debug for NSException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let obj: &Object = self.as_ref();
        write!(f, "{:?} '{}'", obj, self.name())?;
        if let Some(reason) = self.reason() {
            write!(f, " reason:{}", reason)?;
        } else {
            write!(f, " reason:(NULL)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn create_and_query() {
        let exc = NSException::new(
            &NSString::from_str("abc"),
            Some(&NSString::from_str("def")),
            None,
        )
        .unwrap();

        assert_eq!(exc.name(), NSString::from_str("abc"));
        assert_eq!(exc.reason().unwrap(), NSString::from_str("def"));
        assert!(exc.user_info().is_none());

        let debug = format!("<NSException: {:p}> 'abc' reason:def", exc);
        assert_eq!(format!("{:?}", exc), debug);

        let description = if cfg!(feature = "gnustep-1-7") {
            format!("<NSException: {:p}> NAME:abc REASON:def", exc)
        } else {
            "def".into()
        };

        let exc: &NSObject = &exc;
        assert_eq!(format!("{:?}", exc), description);
    }

    #[test]
    #[should_panic = "'abc' reason:def"]
    fn unwrap() {
        let exc = NSException::new(
            &NSString::from_str("abc"),
            Some(&NSString::from_str("def")),
            None,
        )
        .unwrap();

        let _: () = Err(exc).unwrap();
    }

    // Further tests in `tests::exception`
}
