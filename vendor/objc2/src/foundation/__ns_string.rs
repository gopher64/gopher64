//! Macro for making a static NSString.
//!
//! This basically does what clang does, see:
//! - Apple: <https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CodeGenModule.cpp#L5057-L5249>
//! - GNUStep 2.0 (not yet supported): <https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CGObjCGNU.cpp#L973-L1118>
//! - Other (not yet supported): <https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CGObjCGNU.cpp#L2471-L2507>
//!
//! Note that this uses the `CFString` static, while `clang` has support for
//! generating a pure `NSString`. We don't support that yet (since I don't
//! know the use-case), but we definitely could!
//! See: <https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CGObjCMac.cpp#L2007-L2068>
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::foundation::NSString;
use crate::rc::Id;
use crate::runtime::Class;

// This is defined in CoreFoundation, but we don't emit a link attribute
// here because it is already linked via Foundation.
//
// Although this is a "private" (underscored) symbol, it is directly
// referenced in Objective-C binaries. So it's safe for us to reference.
extern "C" {
    pub static __CFConstantStringClassReference: Class;
}

/// Structure used to describe a constant `CFString`.
///
/// This struct is the same as [`CF_CONST_STRING`], which contains
/// [`CFRuntimeBase`]. While the documentation clearly says that the ABI of
/// `CFRuntimeBase` should not be relied on, we can rely on it as long as we
/// only do it with regards to `CFString` (because `clang` does this as well).
///
/// [`CFRuntimeBase`]: <https://github.com/apple-oss-distributions/CF/blob/CF-1153.18/CFRuntime.h#L216-L228>
/// [`CF_CONST_STRING`]: <https://github.com/apple-oss-distributions/CF/blob/CF-1153.18/CFInternal.h#L332-L336>
#[repr(C)]
pub struct CFConstString {
    isa: &'static Class,
    // Important that we don't just use `usize` here, since that would be
    // wrong on big-endian systems!
    cfinfo: u32,
    #[cfg(target_pointer_width = "64")]
    _rc: u32,
    data: *const c_void,
    len: usize,
}

// Required to place in a `static`.
unsafe impl Sync for CFConstString {}

impl CFConstString {
    // From `CFString.c`:
    // <https://github.com/apple-oss-distributions/CF/blob/CF-1153.18/CFString.c#L184-L212>
    // > !!! Note: Constant CFStrings use the bit patterns:
    // > C8 (11001000 = default allocator, not inline, not freed contents; 8-bit; has NULL byte; doesn't have length; is immutable)
    // > D0 (11010000 = default allocator, not inline, not freed contents; Unicode; is immutable)
    // > The bit usages should not be modified in a way that would effect these bit patterns.
    //
    // Hence CoreFoundation guarantees that these two are always valid.
    //
    // The `CFTypeID` of `CFStringRef` is guaranteed to always be 7:
    // <https://github.com/apple-oss-distributions/CF/blob/CF-1153.18/CFRuntime.c#L982>
    const FLAGS_ASCII: u32 = 0x07_C8;
    const FLAGS_UTF16: u32 = 0x07_D0;

    pub const unsafe fn new_ascii(isa: &'static Class, data: &'static [u8]) -> Self {
        Self {
            isa,
            cfinfo: Self::FLAGS_ASCII,
            #[cfg(target_pointer_width = "64")]
            _rc: 0,
            data: data.as_ptr().cast(),
            // The length does not include the trailing NUL.
            len: data.len() - 1,
        }
    }

    pub const unsafe fn new_utf16(isa: &'static Class, data: &'static [u16]) -> Self {
        Self {
            isa,
            cfinfo: Self::FLAGS_UTF16,
            #[cfg(target_pointer_width = "64")]
            _rc: 0,
            data: data.as_ptr().cast(),
            // The length does not include the trailing NUL.
            len: data.len() - 1,
        }
    }

    #[inline]
    pub const fn as_nsstring_const(&self) -> &NSString {
        let ptr: *const Self = self;
        unsafe { &*ptr.cast::<NSString>() }
    }

    // This is deliberately not `const` to prevent the result from being used
    // in other statics, since not all platforms support that (yet).
    #[inline]
    pub fn as_nsstring(&self) -> &NSString {
        self.as_nsstring_const()
    }
}

/// Returns `true` if `bytes` is entirely ASCII with no interior NULs.
pub const fn is_ascii_no_nul(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if !byte.is_ascii() || byte == b'\0' {
            return false;
        }
        i += 1;
    }
    true
}

pub struct Utf16Char {
    pub repr: [u16; 2],
    pub len: usize,
}

impl Utf16Char {
    const fn encode(ch: u32) -> Self {
        if ch <= 0xffff {
            Self {
                repr: [ch as u16, 0],
                len: 1,
            }
        } else {
            let payload = ch - 0x10000;
            let hi = (payload >> 10) | 0xd800;
            let lo = (payload & 0x3ff) | 0xdc00;
            Self {
                repr: [hi as u16, lo as u16],
                len: 2,
            }
        }
    }

    #[cfg(test)]
    fn as_slice(&self) -> &[u16] {
        &self.repr[..self.len]
    }
}

pub struct EncodeUtf16Iter {
    str: &'static [u8],
    index: usize,
}

impl EncodeUtf16Iter {
    pub const fn new(str: &'static [u8]) -> Self {
        Self { str, index: 0 }
    }

    pub const fn next(self) -> Option<(Self, Utf16Char)> {
        if self.index >= self.str.len() {
            None
        } else {
            let (index, ch) = decode_utf8(self.str, self.index);
            Some((Self { index, ..self }, Utf16Char::encode(ch)))
        }
    }
}

// (&str bytes, index) -> (new index, decoded char)
const fn decode_utf8(s: &[u8], i: usize) -> (usize, u32) {
    let b0 = s[i];
    match b0 {
        // one-byte seq
        0b0000_0000..=0b0111_1111 => {
            let decoded = b0 as u32;
            (i + 1, decoded)
        }
        // two-byte seq
        0b1100_0000..=0b1101_1111 => {
            let decoded = ((b0 as u32 & 0x1f) << 6) | (s[i + 1] as u32 & 0x3f);
            (i + 2, decoded)
        }
        // 3 byte seq
        0b1110_0000..=0b1110_1111 => {
            let decoded = ((b0 as u32 & 0x0f) << 12)
                | ((s[i + 1] as u32 & 0x3f) << 6)
                | (s[i + 2] as u32 & 0x3f);
            (i + 3, decoded)
        }
        // 4 byte seq
        0b1111_0000..=0b1111_0111 => {
            let decoded = ((b0 as u32 & 0x07) << 18)
                | ((s[i + 1] as u32 & 0x3f) << 12)
                | ((s[i + 2] as u32 & 0x3f) << 6)
                | (s[i + 3] as u32 & 0x3f);
            (i + 4, decoded)
        }
        // continuation bytes, or never-valid bytes.
        0b1000_0000..=0b1011_1111 | 0b1111_1000..=0b1111_1111 => {
            panic!("Encountered invalid bytes")
        }
    }
}

/// Allows storing a [`NSString`] in a static and lazily loading it.
#[doc(hidden)]
pub struct CachedNSString {
    ptr: AtomicPtr<NSString>,
}

impl CachedNSString {
    /// Constructs a new [`CachedNSString`].
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Returns the cached NSString. If no string is yet cached, creates one
    /// with the given name and stores it.
    #[inline]
    pub fn get(&self, s: &str) -> &'static NSString {
        // TODO: Investigate if we can use weaker orderings.
        let ptr = self.ptr.load(Ordering::SeqCst);
        // SAFETY: The pointer is either NULL, or has been created below.
        unsafe { ptr.as_ref() }.unwrap_or_else(|| {
            // "Forget" about releasing the string, effectively promoting it
            // to a static.
            let s = ManuallyDrop::new(NSString::from_str(s));
            let ptr = Id::as_ptr(&s);
            self.ptr.store(ptr as *mut NSString, Ordering::SeqCst);
            // SAFETY: The pointer is valid, and will always be valid, since
            // we haven't released it.
            unsafe { ptr.as_ref().unwrap_unchecked() }
        })
    }
}

/// Creates an [`NSString`][`crate::foundation::NSString`] from a static string.
///
///
/// # Examples
///
/// This macro takes a either a `"string"` literal or `const` string slice as
/// the argument, and produces a `&'static NSString`:
///
/// ```
/// use objc2::ns_string;
/// use objc2::foundation::NSString;
/// # #[cfg(feature = "gnustep-1-7")]
/// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
/// let hello: &'static NSString = ns_string!("hello");
/// assert_eq!(hello.to_string(), "hello");
///
/// const WORLD: &str = "world";
/// let world = ns_string!(WORLD);
/// assert_eq!(world.to_string(), WORLD);
/// ```
///
///
/// # Unicode Strings
///
/// An NSString can contain strings with many different encodings, including
/// ASCII, UTF-8, UTF-16, and so on. This macro automatically converts your
/// string to the most efficient encoding, you don't have to do anything!
///
/// ```
/// # use objc2::ns_string;
/// # #[cfg(feature = "gnustep-1-7")]
/// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
/// let hello_ru = ns_string!("Привет");
/// assert_eq!(hello_ru.to_string(), "Привет");
/// ```
///
/// Note that because this is implemented with `const` evaluation, massive
/// strings can increase compile time, and may even hit the `const` evaluation
/// limit.
///
///
/// # NUL handling
///
/// Strings containing ASCII NUL is allowed, the NUL is preserved as one would
/// expect:
///
/// ```
/// # use objc2::ns_string;
/// # #[cfg(feature = "gnustep-1-7")]
/// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
/// let example = ns_string!("example\0");
/// assert_eq!(example.to_string(), "example\0");
///
/// let example = ns_string!("exa\0mple");
/// assert_eq!(example.to_string(), "exa\0mple");
/// ```
///
///
/// # Runtime Cost
///
/// None.
///
/// The result is equivalent to `@"string"` syntax in Objective-C.
///
/// Because of that, this should be preferred over [`NSString::from_str`]
/// where possible.
///
/// [`NSString::from_str`]: crate::foundation::NSString::from_str
#[cfg(feature = "foundation")] // For auto_doc_cfg
#[macro_export]
macro_rules! ns_string {
    ($s:expr) => {{
        // Immediately place in constant for better UI
        const INPUT: &str = $s;
        $crate::__ns_string_inner!(INPUT)
    }};
}

#[doc(hidden)]
#[cfg(feature = "apple")]
#[macro_export]
macro_rules! __ns_string_inner {
    ($inp:ident) => {{
        const X: &[u8] = $inp.as_bytes();
        $crate::__ns_string_inner!(@inner X);
        // Return &'static NSString
        CFSTRING.as_nsstring()
    }};
    (@inner $inp:ident) => {
        // Note: We create both the ASCII + NUL and the UTF-16 + NUL versions
        // of the string, since we can't conditionally create a static.
        //
        // Since we don't add the `#[used]` attribute, Rust can fairly easily
        // figure out that one of the variants are never used, and simply
        // exclude it.

        // Convert the input slice to a C-style string with a NUL byte.
        //
        // The section is the same as what clang sets, see:
        // https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CodeGenModule.cpp#L5192
        #[link_section = "__TEXT,__cstring,cstring_literals"]
        static ASCII: [u8; $inp.len() + 1] = {
            // Zero-fill with $inp.len() + 1
            let mut res: [u8; $inp.len() + 1] = [0; $inp.len() + 1];
            let mut i = 0;
            // Fill with data from $inp
            while i < $inp.len() {
                res[i] = $inp[i];
                i += 1;
            }
            // Now contains $inp + '\0'
            res
        };

        // The full UTF-16 contents along with the written length.
        const UTF16_FULL: (&[u16; $inp.len()], usize) = {
            let mut out = [0u16; $inp.len()];
            let mut iter = $crate::foundation::__ns_string::EncodeUtf16Iter::new($inp);
            let mut written = 0;

            while let Some((state, chars)) = iter.next() {
                iter = state;
                out[written] = chars.repr[0];
                written += 1;

                if chars.len > 1 {
                    out[written] = chars.repr[1];
                    written += 1;
                }
            }

            (&{ out }, written)
        };

        // Convert the slice to an UTF-16 array + a final NUL byte.
        //
        // The section is the same as what clang sets, see:
        // https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CodeGenModule.cpp#L5193
        #[link_section = "__TEXT,__ustring"]
        static UTF16: [u16; UTF16_FULL.1 + 1] = {
            // Zero-fill with UTF16_FULL.1 + 1
            let mut res: [u16; UTF16_FULL.1 + 1] = [0; UTF16_FULL.1 + 1];
            let mut i = 0;
            // Fill with data from UTF16_FULL.0 up until UTF16_FULL.1
            while i < UTF16_FULL.1 {
                res[i] = UTF16_FULL.0[i];
                i += 1;
            }
            // Now contains UTF16_FULL.1 + NUL
            res
        };

        // Create the constant string structure, and store it in a static
        // within a special section.
        //
        // The section is the same as what clang sets, see:
        // https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CodeGenModule.cpp#L5243
        #[link_section = "__DATA,__cfstring"]
        static CFSTRING: $crate::foundation::__ns_string::CFConstString = unsafe {
            if $crate::foundation::__ns_string::is_ascii_no_nul($inp) {
                // This is technically an optimization (UTF-16 strings are
                // always valid), but it's a fairly important one!
                $crate::foundation::__ns_string::CFConstString::new_ascii(
                    &$crate::foundation::__ns_string::__CFConstantStringClassReference,
                    &ASCII,
                )
            } else {
                $crate::foundation::__ns_string::CFConstString::new_utf16(
                    &$crate::foundation::__ns_string::__CFConstantStringClassReference,
                    &UTF16,
                )
            }
        };
    };
}

#[doc(hidden)]
#[cfg(not(feature = "apple"))]
#[macro_export]
macro_rules! __ns_string_inner {
    ($inp:ident) => {{
        use $crate::foundation::__ns_string::CachedNSString;
        static CACHED_NSSTRING: CachedNSString = CachedNSString::new();
        CACHED_NSSTRING.get($inp)
    }};
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn test_is_ascii() {
        assert!(is_ascii_no_nul(b"a"));
        assert!(is_ascii_no_nul(b"abc"));

        assert!(!is_ascii_no_nul(b"\xff"));

        assert!(!is_ascii_no_nul(b"\0"));
        assert!(!is_ascii_no_nul(b"a\0b"));
        assert!(!is_ascii_no_nul(b"ab\0"));
        assert!(!is_ascii_no_nul(b"a\0b\0"));
    }

    #[test]
    fn test_decode_utf8() {
        for c in '\u{0}'..=core::char::MAX {
            let mut buf;
            for off in 0..4 {
                // Ensure we see garbage if we read outside bounds.
                buf = [0xff; 8];
                let len = c.encode_utf8(&mut buf[off..(off + 4)]).len();
                let (end_idx, decoded) = decode_utf8(&buf, off);
                assert_eq!(
                    (end_idx, decoded),
                    (off + len, c as u32),
                    "failed for U+{code:04X} ({ch:?}) encoded as {buf:#x?} over {range:?}",
                    code = c as u32,
                    ch = c,
                    buf = &buf[off..(off + len)],
                    range = off..(off + len),
                );
            }
        }
    }

    #[test]
    fn encode_utf16() {
        for c in '\u{0}'..=core::char::MAX {
            assert_eq!(
                c.encode_utf16(&mut [0u16; 2]),
                Utf16Char::encode(c as u32).as_slice(),
                "failed for U+{:04X} ({:?})",
                c as u32,
                c
            );
        }
    }

    #[test]
    fn ns_string() {
        macro_rules! test {
            ($($s:expr,)+) => {$({
                let s1 = ns_string!($s);
                let s2 = NSString::from_str($s);

                assert_eq!(s1, s1);
                assert_eq!(s1, &*s2);

                assert_eq!(s1.to_string(), $s);
                assert_eq!(s2.to_string(), $s);
            })+};
        }

        test! {
            "",
            "asdf",
            "🦀",
            "🏳️‍🌈",
            "𝄞music",
            "abcd【e】fg",
            "abcd⒠fg",
            "ääääh",
            "lööps, bröther?",
            "\u{fffd} \u{fffd} \u{fffd}",
            "讓每個人都能打造出。",
            "\0",
            "\0\x01\x02\x03\x04\x05\x06\x07\x08\x09",
            // "\u{feff}", // TODO
            include_str!("__ns_string.rs"),
        }
    }

    #[test]
    fn ns_string_in_unsafe() {
        // Test that the `unused_unsafe` lint doesn't trigger
        let s = unsafe {
            let s: *const NSString = ns_string!("abc");
            &*s
        };
        assert_eq!(s.to_string(), "abc");
    }
}
