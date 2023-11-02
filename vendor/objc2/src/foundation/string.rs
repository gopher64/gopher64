use alloc::borrow::ToOwned;
use core::cmp;
use core::ffi::c_void;
use core::fmt;
use core::panic::RefUnwindSafe;
use core::panic::UnwindSafe;
use core::ptr::NonNull;
use core::slice;
use core::str;
use std::os::raw::c_char;

use super::{NSComparisonResult, NSCopying, NSMutableCopying, NSMutableString, NSObject};
use crate::rc::{autoreleasepool, AutoreleasePool, DefaultId, Id, Shared};
use crate::runtime::{Class, Object};
use crate::{extern_class, extern_methods, msg_send, msg_send_id, ClassType};

#[cfg(feature = "apple")]
const UTF8_ENCODING: usize = 4;
#[cfg(feature = "gnustep-1-7")]
const UTF8_ENCODING: i32 = 4;

extern_class!(
    /// An immutable, plain-text Unicode string object.
    ///
    /// Can be created statically using the [`ns_string!`] macro.
    ///
    /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsstring?language=objc).
    ///
    /// [`ns_string!`]: crate::ns_string
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSString;
    // TODO: Use isEqualToString: for comparison (instead of just isEqual:)
    // The former is more performant

    // TODO: Check if performance of NSSelectorFromString is worthwhile

    unsafe impl ClassType for NSString {
        type Super = NSObject;
    }
);

// SAFETY: `NSString` is immutable and `NSMutableString` can only be mutated
// from `&mut` methods.
unsafe impl Sync for NSString {}
unsafe impl Send for NSString {}

// Even if an exception occurs inside a string method, the state of the string
// (should) still be perfectly safe to access.
impl UnwindSafe for NSString {}
impl RefUnwindSafe for NSString {}

extern_methods!(
    unsafe impl NSString {
        /// Construct an empty NSString.
        pub fn new() -> Id<Self, Shared> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        /// Create a new string by appending the given string to self.
        ///
        ///
        /// # Example
        ///
        /// ```
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        /// use objc2::ns_string;
        /// let error_tag = ns_string!("Error: ");
        /// let error_string = ns_string!("premature end of file.");
        /// let error_message = error_tag.concat(error_string);
        /// assert_eq!(&*error_message, ns_string!("Error: premature end of file."));
        /// ```
        #[doc(alias = "stringByAppendingString")]
        #[doc(alias = "stringByAppendingString:")]
        pub fn concat(&self, other: &Self) -> Id<Self, Shared> {
            // SAFETY: The other string is non-null, and won't be retained
            // by the function.
            unsafe { msg_send_id![self, stringByAppendingString: other] }
        }

        /// Create a new string by appending the given string, separated by
        /// a path separator.
        ///
        /// This is similar to [`Path::join`][std::path::Path::join].
        ///
        /// Note that this method only works with file paths (not, for
        /// example, string representations of URLs).
        ///
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        /// use objc2::ns_string;
        ///
        /// let extension = ns_string!("scratch.tiff");
        /// assert_eq!(&*ns_string!("/tmp").join_path(extension), ns_string!("/tmp/scratch.tiff"));
        /// assert_eq!(&*ns_string!("/tmp/").join_path(extension), ns_string!("/tmp/scratch.tiff"));
        /// assert_eq!(&*ns_string!("/").join_path(extension), ns_string!("/scratch.tiff"));
        /// assert_eq!(&*ns_string!("").join_path(extension), ns_string!("scratch.tiff"));
        /// ```
        #[doc(alias = "stringByAppendingPathComponent")]
        #[doc(alias = "stringByAppendingPathComponent:")]
        pub fn join_path(&self, other: &Self) -> Id<Self, Shared> {
            // SAFETY: Same as `Self::concat`.
            unsafe { msg_send_id![self, stringByAppendingPathComponent: other] }
        }

        /// The number of UTF-8 code units in `self`.
        #[doc(alias = "lengthOfBytesUsingEncoding")]
        #[doc(alias = "lengthOfBytesUsingEncoding:")]
        pub fn len(&self) -> usize {
            unsafe { msg_send![self, lengthOfBytesUsingEncoding: UTF8_ENCODING] }
        }

        /// The number of UTF-16 code units in the string.
        ///
        /// See also [`NSString::len`].
        #[doc(alias = "length")]
        #[sel(length)]
        pub fn len_utf16(&self) -> usize;

        pub fn is_empty(&self) -> bool {
            // TODO: lengthOfBytesUsingEncoding: might sometimes return 0 for
            // other reasons, so this is not really correct!
            self.len() == 0
        }

        /// Get the [`str`](`prim@str`) representation of this string if it can be
        /// done efficiently.
        ///
        /// Returns [`None`] if the internal storage does not allow this to be
        /// done efficiently. Use [`NSString::as_str`] or `NSString::to_string`
        /// if performance is not an issue.
        #[doc(alias = "CFStringGetCStringPtr")]
        #[allow(unused)]
        // TODO: Finish this
        fn as_str_wip(&self) -> Option<&str> {
            type CFStringEncoding = u32;
            #[allow(non_upper_case_globals)]
            // https://developer.apple.com/documentation/corefoundation/cfstringbuiltinencodings/kcfstringencodingutf8?language=objc
            const kCFStringEncodingUTF8: CFStringEncoding = 0x08000100;
            extern "C" {
                // https://developer.apple.com/documentation/corefoundation/1542133-cfstringgetcstringptr?language=objc
                fn CFStringGetCStringPtr(s: &NSString, encoding: CFStringEncoding)
                    -> *const c_char;
            }
            let bytes = unsafe { CFStringGetCStringPtr(self, kCFStringEncodingUTF8) };
            NonNull::new(bytes as *mut u8).map(|bytes| {
                let len = self.len();
                let bytes: &[u8] = unsafe { slice::from_raw_parts(bytes.as_ptr(), len) };
                str::from_utf8(bytes).unwrap()
            })
        }

        /// Get an [UTF-16] string slice if it can be done efficiently.
        ///
        /// Returns [`None`] if the internal storage of `self` does not allow this
        /// to be returned efficiently.
        ///
        /// See [`as_str`](Self::as_str) for the UTF-8 equivalent.
        ///
        /// [UTF-16]: https://en.wikipedia.org/wiki/UTF-16
        #[allow(unused)]
        // TODO: Finish this
        fn as_utf16(&self) -> Option<&[u16]> {
            extern "C" {
                // https://developer.apple.com/documentation/corefoundation/1542939-cfstringgetcharactersptr?language=objc
                fn CFStringGetCharactersPtr(s: &NSString) -> *const u16;
            }
            let ptr = unsafe { CFStringGetCharactersPtr(self) };
            NonNull::new(ptr as *mut u16)
                .map(|ptr| unsafe { slice::from_raw_parts(ptr.as_ptr(), self.len_utf16()) })
        }

        /// Get the [`str`](`prim@str`) representation of this.
        ///
        /// TODO: Further explain this.
        #[doc(alias = "UTF8String")]
        pub fn as_str<'r, 's: 'r, 'p: 'r>(&'s self, pool: &'p AutoreleasePool) -> &'r str {
            // NOTE: Please keep up to date with `objc2::exception`!

            // This is necessary until `auto` types stabilizes.
            pool.__verify_is_inner();

            // The documentation on `UTF8String` is a bit sparse, but with
            // educated guesses and testing I've determined that NSString stores
            // a pointer to the string data, sometimes with an UTF-8 encoding,
            // (usual for ascii data), sometimes in other encodings (UTF-16?).
            //
            // `UTF8String` then checks the internal encoding:
            // - If the data is UTF-8 encoded, it returns the internal pointer.
            // - If the data is in another encoding, it creates a new allocation,
            //   writes the UTF-8 representation of the string into it,
            //   autoreleases the allocation and returns a pointer to it.
            //
            // So the lifetime of the returned pointer is either the same as the
            // NSString OR the lifetime of the innermost @autoreleasepool.
            //
            // https://developer.apple.com/documentation/foundation/nsstring/1411189-utf8string?language=objc
            let bytes: *const c_char = unsafe { msg_send![self, UTF8String] };
            let bytes: *const u8 = bytes.cast();
            let len = self.len();

            // SAFETY:
            // The held AutoreleasePool is the innermost, and the reference is
            // constrained both by the pool and the NSString.
            //
            // `len` is the length of the string in the UTF-8 encoding.
            //
            // `bytes` is a null-terminated C string (with length = len + 1), so
            // it is never a NULL pointer.
            let bytes: &'r [u8] = unsafe { slice::from_raw_parts(bytes, len) };

            // TODO: Always UTF-8, so should we use `from_utf8_unchecked`?
            str::from_utf8(bytes).unwrap()

            // NOTE: Please keep up to date with `objc2::exception`!
        }

        // TODO: Allow usecases where the NUL byte from `UTF8String` is kept?

        /// Creates an immutable `NSString` by copying the given string slice.
        ///
        /// Prefer using the [`ns_string!`] macro when possible.
        ///
        /// [`ns_string!`]: crate::ns_string
        #[doc(alias = "initWithBytes")]
        #[doc(alias = "initWithBytes:length:encoding:")]
        #[allow(clippy::should_implement_trait)] // Not really sure of a better name
        pub fn from_str(string: &str) -> Id<Self, Shared> {
            unsafe {
                let obj = from_str(Self::class(), string);
                Id::new(obj.cast()).unwrap()
            }
        }

        // TODO: initWithBytesNoCopy:, maybe add lifetime parameter to NSString?
        // See https://github.com/nvzqz/fruity/blob/320efcf715c2c5fbd2f3084f671f2be2e03a6f2b/src/foundation/ns_string/mod.rs#L350-L381
        // Might be quite difficult, as Objective-C code might assume the NSString
        // is always alive?
        // See https://github.com/drewcrawford/foundationr/blob/b27683417a35510e8e5d78a821f081905b803de6/src/nsstring.rs

        /// Whether the given string matches the beginning characters of this
        /// string.
        ///
        /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsstring/1410309-hasprefix?language=objc).
        #[doc(alias = "hasPrefix")]
        #[doc(alias = "hasPrefix:")]
        #[sel(hasPrefix:)]
        pub fn has_prefix(&self, prefix: &NSString) -> bool;

        /// Whether the given string matches the ending characters of this string.
        ///
        /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsstring/1416529-hassuffix?language=objc).
        #[doc(alias = "hasSuffix")]
        #[doc(alias = "hasSuffix:")]
        #[sel(hasSuffix:)]
        pub fn has_suffix(&self, suffix: &NSString) -> bool;

        // TODO: Other comparison methods:
        // - compare:options:
        // - compare:options:range:
        // - compare:options:range:locale:
        // - localizedCompare:
        // - caseInsensitiveCompare:
        // - localizedCaseInsensitiveCompare:
        // - localizedStandardCompare:
        #[sel(compare:)]
        fn compare(&self, other: &Self) -> NSComparisonResult;

        // pub fn from_nsrange(range: NSRange) -> Id<Self, Shared>
        // https://developer.apple.com/documentation/foundation/1415155-nsstringfromrange?language=objc
    }
);

pub(crate) fn from_str(cls: &Class, string: &str) -> *mut Object {
    let bytes: *const c_void = string.as_ptr().cast();
    unsafe {
        let obj: *mut Object = msg_send![cls, alloc];
        msg_send![
            obj,
            initWithBytes: bytes,
            length: string.len(),
            encoding: UTF8_ENCODING,
        ]
    }
}

impl PartialOrd for NSString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NSString {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.compare(other).into()
    }
}

// TODO: PartialEq and PartialOrd against &str
// See `fruity`'s implementation:
// https://github.com/nvzqz/fruity/blob/320efcf715c2c5fbd2f3084f671f2be2e03a6f2b/src/foundation/ns_string/mod.rs#L69-L163

impl DefaultId for NSString {
    type Ownership = Shared;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

unsafe impl NSCopying for NSString {
    type Ownership = Shared;
    type Output = NSString;
}

unsafe impl NSMutableCopying for NSString {
    type Output = NSMutableString;
}

impl ToOwned for NSString {
    type Owned = Id<NSString, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

impl fmt::Display for NSString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The call to `to_owned` is unfortunate, but is required to work
        // around `f` not being AutoreleaseSafe.
        // TODO: Fix this!
        let s = autoreleasepool(|pool| self.as_str(pool).to_owned());
        fmt::Display::fmt(&s, f)
    }
}

impl fmt::Debug for NSString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The call to `to_owned` is unfortunate, but is required to work
        // around `f` not being AutoreleaseSafe.
        // TODO: Fix this!
        let s = autoreleasepool(|pool| self.as_str(pool).to_owned());
        fmt::Debug::fmt(&s, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use core::ptr;

    #[test]
    fn test_equality() {
        let s1 = NSString::from_str("abc");
        let s2 = NSString::from_str("abc");
        assert_eq!(s1, s1);
        assert_eq!(s1, s2);

        let s3 = NSString::from_str("def");
        assert_ne!(s1, s3);
    }

    #[test]
    fn display_debug() {
        let s = NSString::from_str("xyz\"123");
        assert_eq!(format!("{}", s), "xyz\"123");
        assert_eq!(format!("{:?}", s), r#""xyz\"123""#);
    }

    #[test]
    fn test_empty() {
        let s1 = NSString::from_str("");
        let s2 = NSString::new();

        assert_eq!(s1.len(), 0);
        assert_eq!(s2.len(), 0);

        assert_eq!(s1, s2);

        autoreleasepool(|pool| {
            assert_eq!(s1.as_str(pool), "");
            assert_eq!(s2.as_str(pool), "");
        });
    }

    #[test]
    fn test_utf8() {
        let expected = "ประเทศไทย中华Việt Nam";
        let s = NSString::from_str(expected);
        assert_eq!(s.len(), expected.len());
        autoreleasepool(|pool| {
            assert_eq!(s.as_str(pool), expected);
        });
    }

    #[test]
    fn test_nul() {
        let expected = "\0";
        let s = NSString::from_str(expected);
        assert_eq!(s.len(), expected.len());
        autoreleasepool(|pool| {
            assert_eq!(s.as_str(pool), expected);
        });
    }

    #[test]
    fn test_interior_nul() {
        let expected = "Hello\0World";
        let s = NSString::from_str(expected);
        assert_eq!(s.len(), expected.len());
        autoreleasepool(|pool| {
            assert_eq!(s.as_str(pool), expected);
        });
    }

    #[test]
    fn test_copy() {
        let s1 = NSString::from_str("abc");
        let s2 = s1.copy();
        // An optimization that NSString makes, since it is immutable
        assert_eq!(Id::as_ptr(&s1), Id::as_ptr(&s2));
        assert!(s2.is_kind_of::<NSString>());

        let s3 = s1.mutable_copy();
        assert_ne!(Id::as_ptr(&s1), Id::as_ptr(&s3).cast());
        assert!(s3.is_kind_of::<NSMutableString>());
    }

    #[test]
    fn test_copy_nsstring_is_same() {
        let string1 = NSString::from_str("Hello, world!");
        let string2 = string1.copy();
        assert!(
            ptr::eq(&*string1, &*string2),
            "Cloned NSString didn't have the same address"
        );
    }

    #[test]
    /// Apparently NSString does this for some reason?
    fn test_strips_first_leading_zero_width_no_break_space() {
        let ns_string = NSString::from_str("\u{feff}");
        let expected = "";
        autoreleasepool(|pool| {
            assert_eq!(ns_string.as_str(pool), expected);
        });
        assert_eq!(ns_string.len(), 0);

        let s = "\u{feff}\u{feff}a\u{feff}";

        // Huh, this difference might be a GNUStep bug?
        #[cfg(feature = "apple")]
        let expected = "\u{feff}a\u{feff}";
        #[cfg(feature = "gnustep-1-7")]
        let expected = "a\u{feff}";

        let ns_string = NSString::from_str(s);
        autoreleasepool(|pool| {
            assert_eq!(ns_string.as_str(pool), expected);
        });
        assert_eq!(ns_string.len(), expected.len());
    }

    #[test]
    fn test_hash() {
        use core::hash::Hasher;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;

        let s1 = NSString::from_str("example string goes here");
        let s2 = NSString::from_str("example string goes here");

        let mut hashstate = DefaultHasher::new();
        let mut hashstate2 = DefaultHasher::new();

        s1.hash(&mut hashstate);
        s2.hash(&mut hashstate2);

        assert_eq!(hashstate.finish(), hashstate2.finish());
    }

    #[test]
    fn test_prefix_suffix() {
        let s = NSString::from_str("abcdef");
        let prefix = NSString::from_str("abc");
        let suffix = NSString::from_str("def");
        assert!(s.has_prefix(&prefix));
        assert!(s.has_suffix(&suffix));
        assert!(!s.has_prefix(&suffix));
        assert!(!s.has_suffix(&prefix));
    }

    #[test]
    #[allow(clippy::nonminimal_bool)]
    fn test_cmp() {
        let s1 = NSString::from_str("aa");
        assert!(s1 <= s1);
        assert!(s1 >= s1);
        let s2 = NSString::from_str("ab");
        assert!(s1 < s2);
        assert!(!(s1 > s2));
        assert!(s1 <= s2);
        assert!(!(s1 >= s2));
        let s3 = NSString::from_str("ba");
        assert!(s1 < s3);
        assert!(!(s1 > s3));
        assert!(s1 <= s3);
        assert!(!(s1 >= s3));
        assert!(s2 < s3);
        assert!(!(s2 > s3));
        assert!(s2 <= s3);
        assert!(!(s2 >= s3));

        let s = NSString::from_str("abc");
        let shorter = NSString::from_str("a");
        let longer = NSString::from_str("abcdef");
        assert!(s > shorter);
        assert!(s < longer);
    }
}
