use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{
    NSCopying, NSDictionary, NSMutableAttributedString, NSMutableCopying, NSObject, NSString,
};
use crate::rc::{DefaultId, Id, Shared};
use crate::runtime::Object;
use crate::{extern_class, extern_methods, msg_send_id, ClassType};

extern_class!(
    /// A string that has associated attributes for portions of its text.
    ///
    /// Examples of attributes could be: Visual style, hyperlinks, or
    /// accessibility data.
    ///
    /// Conceptually, each UTF-16 code unit in an attributed string has its
    /// own collection of attributes - most often though
    ///
    /// Only the most basic functionality is defined here, the `AppKit`
    /// framework contains most of the extension methods.
    ///
    /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsattributedstring?language=objc).
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSAttributedString;

    unsafe impl ClassType for NSAttributedString {
        type Super = NSObject;
    }
);

// SAFETY: `NSAttributedString` is immutable and `NSMutableAttributedString`
// can only be mutated from `&mut` methods.
unsafe impl Sync for NSAttributedString {}
unsafe impl Send for NSAttributedString {}

// Same reasoning as `NSString`.
impl UnwindSafe for NSAttributedString {}
impl RefUnwindSafe for NSAttributedString {}

/// Attributes that you can apply to text in an attributed string.
pub type NSAttributedStringKey = NSString;

extern_methods!(
    /// Creating attributed strings.
    unsafe impl NSAttributedString {
        /// Construct an empty attributed string.
        pub fn new() -> Id<Self, Shared> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        /// Creates a new attributed string from the given string and attributes.
        ///
        /// The attributes are associated with every UTF-16 code unit in the
        /// string.
        #[doc(alias = "initWithString:")]
        pub fn new_with_attributes(
            string: &NSString,
            // TODO: Mutability of the dictionary should be (Shared, Shared)
            attributes: &NSDictionary<NSAttributedStringKey, Object>,
        ) -> Id<Self, Shared> {
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithString: string, attributes: attributes]
            }
        }

        /// Creates a new attributed string without any attributes.
        #[doc(alias = "initWithString:")]
        pub fn from_nsstring(string: &NSString) -> Id<Self, Shared> {
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithString: string]
            }
        }
    }

    /// Querying.
    unsafe impl NSAttributedString {
        // TODO: Lifetimes?
        pub fn string(&self) -> Id<NSString, Shared> {
            unsafe { msg_send_id![self, string] }
        }

        /// Alias for `self.string().len_utf16()`.
        #[doc(alias = "length")]
        #[sel(length)]
        pub fn len_utf16(&self) -> usize;

        // /// TODO
        // ///
        // /// See [Apple's documentation on Effective and Maximal Ranges][doc].
        // ///
        // /// [doc]: https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/AttributedStrings/Tasks/AccessingAttrs.html#//apple_ref/doc/uid/20000161-SW2
        // #[doc(alias = "attributesAtIndex:effectiveRange:")]
        // pub fn attributes_in_effective_range(
        //     &self,
        //     index: usize,
        //     range: Range<usize>,
        // ) -> Id<Self, Shared> {
        //     let range = NSRange::from(range);
        //     todo!()
        // }
        //
        // attributesAtIndex:longestEffectiveRange:inRange:

        // TODO: attributedSubstringFromRange:
        // TODO: enumerateAttributesInRange:options:usingBlock:
    }
);

impl DefaultId for NSAttributedString {
    type Ownership = Shared;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

unsafe impl NSCopying for NSAttributedString {
    type Ownership = Shared;
    type Output = NSAttributedString;
}

unsafe impl NSMutableCopying for NSAttributedString {
    type Output = NSMutableAttributedString;
}

impl alloc::borrow::ToOwned for NSAttributedString {
    type Owned = Id<NSAttributedString, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

impl fmt::Debug for NSAttributedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use -[NSAttributedString description] since it is pretty good
        let obj: &NSObject = self;
        fmt::Debug::fmt(obj, f)
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use alloc::{format, vec};

    use super::*;
    use crate::rc::{autoreleasepool, Owned};

    #[test]
    fn test_new() {
        let s = NSAttributedString::new();
        assert_eq!(&s.string().to_string(), "");
    }

    #[test]
    fn test_string_bound_to_attributed() {
        let attr_s = {
            let source = NSString::from_str("Hello world!");
            NSAttributedString::from_nsstring(&source)
        };
        let s = autoreleasepool(|_| attr_s.string());
        assert_eq!(s.len(), 12);
    }

    #[test]
    fn test_from_nsstring() {
        let s = NSAttributedString::from_nsstring(&NSString::from_str("abc"));
        assert_eq!(&s.string().to_string(), "abc");
    }

    #[test]
    fn test_copy() {
        let s1 = NSAttributedString::from_nsstring(&NSString::from_str("abc"));
        let s2 = s1.copy();
        // NSAttributedString performs this optimization in GNUStep's runtime,
        // but not in Apple's; so we don't test for it!
        // assert_eq!(Id::as_ptr(&s1), Id::as_ptr(&s2));
        assert!(s2.is_kind_of::<NSAttributedString>());

        let s3 = s1.mutable_copy();
        assert_ne!(Id::as_ptr(&s1), Id::as_ptr(&s3).cast());
        assert!(s3.is_kind_of::<NSMutableAttributedString>());
    }

    #[test]
    fn test_debug() {
        let s = NSAttributedString::from_nsstring(&NSString::from_str("abc"));
        let expected = if cfg!(feature = "gnustep-1-7") {
            "abc{}"
        } else {
            "abc{\n}"
        };
        assert_eq!(format!("{:?}", s), expected);

        let obj: Id<Object, Owned> = unsafe { Id::cast(NSObject::new()) };
        let ptr: *const Object = &*obj;
        let s = NSAttributedString::new_with_attributes(
            &NSString::from_str("abc"),
            &NSDictionary::from_keys_and_objects(&[&*NSString::from_str("test")], vec![obj]),
        );
        let expected = if cfg!(feature = "gnustep-1-7") {
            format!("abc{{test = \"<NSObject: {:?}>\"; }}", ptr)
        } else {
            format!("abc{{\n    test = \"<NSObject: {:?}>\";\n}}", ptr)
        };
        assert_eq!(format!("{:?}", s), expected);
    }
}
