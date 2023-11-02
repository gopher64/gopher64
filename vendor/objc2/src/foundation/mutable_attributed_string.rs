use core::fmt;

use super::{NSAttributedString, NSCopying, NSMutableCopying, NSObject, NSString};
use crate::rc::{DefaultId, Id, Owned, Shared};
use crate::{extern_class, extern_methods, msg_send_id, ClassType};

extern_class!(
    /// A mutable string that has associated attributes.
    ///
    /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsmutableattributedstring?language=objc).
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSMutableAttributedString;

    unsafe impl ClassType for NSMutableAttributedString {
        #[inherits(NSObject)]
        type Super = NSAttributedString;
    }
);

extern_methods!(
    /// Creating mutable attributed strings.
    unsafe impl NSMutableAttributedString {
        /// Construct an empty mutable attributed string.
        pub fn new() -> Id<Self, Owned> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        // TODO: new_with_attributes

        #[doc(alias = "initWithString:")]
        pub fn from_nsstring(string: &NSString) -> Id<Self, Owned> {
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithString: string]
            }
        }

        #[doc(alias = "initWithAttributedString:")]
        pub fn from_attributed_nsstring(attributed_string: &NSAttributedString) -> Id<Self, Owned> {
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithAttributedString: attributed_string]
            }
        }
    }

    /// Modifying the attributed string.
    unsafe impl NSMutableAttributedString {
        // TODO
        // - mutableString
        // - replaceCharactersInRange:withString:
        // - setAttributes:range:

        /// Replaces the entire attributed string.
        #[doc(alias = "setAttributedString:")]
        #[sel(setAttributedString:)]
        pub fn replace(&mut self, attributed_string: &NSAttributedString);
    }
);

impl DefaultId for NSMutableAttributedString {
    type Ownership = Owned;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

unsafe impl NSCopying for NSMutableAttributedString {
    type Ownership = Shared;
    type Output = NSAttributedString;
}

unsafe impl NSMutableCopying for NSMutableAttributedString {
    type Output = NSMutableAttributedString;
}

impl alloc::borrow::ToOwned for NSMutableAttributedString {
    type Owned = Id<NSMutableAttributedString, Owned>;
    fn to_owned(&self) -> Self::Owned {
        self.mutable_copy()
    }
}

impl fmt::Debug for NSMutableAttributedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn test_new() {
        let s = NSAttributedString::new();
        assert_eq!(&s.string().to_string(), "");
    }

    #[test]
    fn test_copy() {
        let s1 = NSMutableAttributedString::from_nsstring(&NSString::from_str("abc"));
        let s2 = s1.copy();
        assert_ne!(Id::as_ptr(&s1).cast(), Id::as_ptr(&s2));
        assert!(s2.is_kind_of::<NSAttributedString>());

        let s3 = s1.mutable_copy();
        assert_ne!(Id::as_ptr(&s1), Id::as_ptr(&s3));
        assert!(s3.is_kind_of::<NSMutableAttributedString>());
    }
}
