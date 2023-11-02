//! Bindings to the `Foundation` framework.
//!
//! This is the [`std`] equivalent for Objective-C, containing essential data
//! types, collections, and operating-system services.
//!
//! See [Apple's documentation](https://developer.apple.com/documentation/foundation?language=objc).
//!
//!
//! ## Philosophy
//!
//! The `Foundation` framework is _huge_! If we aspired to map every API it
//! exposes (a lot of it is just helper methods to make Objective-C more
//! ergonomic), this library would never be finished. Instead, our focus lies
//! on conversion methods, to allow easily using them from Rust.
//!
//! If you find some API that an object doesn't expose (but should), we gladly
//! accept [pull requests]. If it is something that is out of scope, these
//! objects implement the [`Message`] trait, so you can always just manually
//! call a method on them using the [`msg_send!`] family of macros.
//!
//! [pull requests]: https://github.com/madsmtm/objc2/pulls
//! [`Message`]: crate::Message
//! [`msg_send!`]: crate::msg_send
//!
//!
//! # Use of `Deref`
//!
//! `objc2::foundation` uses the [`Deref`] trait in a bit special way: All
//! objects deref to their superclasses. For example, `NSMutableArray` derefs
//! to `NSArray`, which in turn derefs to `NSObject`.
//!
//! Note that this is explicitly recommended against in [the
//! documentation][`Deref`] and [the Rust Design patterns
//! book][anti-pattern-deref] (see those links for details).
//!
//! Due to Objective-C objects only ever being accessible behind pointers in
//! the first place, the problems stated there are less severe, and having the
//! implementation just means that everything is much nicer when you actually
//! want to use the objects!
//!
//! All objects also implement [`AsRef`] and [`AsMut`] to their superclass,
//! and can be used in [`Id::into_super`], so if you favour explicit
//! conversion, that is a possibility too.
//!
//! [`Deref`]: std::ops::Deref
//! [`ClassType`]: crate::ClassType
//! [anti-pattern-deref]: https://rust-unofficial.github.io/patterns/anti_patterns/deref.html
//! [`Id::into_super`]: crate::rc::Id::into_super

// TODO: Remove these
#![allow(missing_docs)]
#![allow(clippy::missing_safety_doc)]

use std::os::raw::c_double;

pub use self::array::NSArray;
pub use self::attributed_string::{NSAttributedString, NSAttributedStringKey};
pub use self::bundle::NSBundle;
pub use self::comparison_result::NSComparisonResult;
pub use self::copying::{NSCopying, NSMutableCopying};
pub use self::data::NSData;
pub use self::dictionary::NSDictionary;
pub use self::enumerator::{NSEnumerator, NSFastEnumeration, NSFastEnumerator};
pub use self::error::{NSError, NSErrorDomain, NSErrorUserInfoKey};
pub use self::exception::NSException;
pub use self::geometry::{CGFloat, CGPoint, CGRect, CGSize, NSPoint, NSRect, NSSize};
pub use self::mutable_array::NSMutableArray;
pub use self::mutable_attributed_string::NSMutableAttributedString;
pub use self::mutable_data::NSMutableData;
pub use self::mutable_dictionary::NSMutableDictionary;
pub use self::mutable_set::NSMutableSet;
pub use self::mutable_string::NSMutableString;
pub use self::number::NSNumber;
pub use self::object::NSObject;
pub use self::process_info::NSProcessInfo;
pub use self::range::NSRange;
pub use self::set::NSSet;
pub use self::string::NSString;
pub use self::thread::{is_main_thread, is_multi_threaded, MainThreadMarker, NSThread};
#[cfg(not(macos_10_7))] // Temporary
pub use self::uuid::NSUUID;
pub use self::value::NSValue;
pub use self::zone::NSZone;

// Available under Foundation, so makes sense here as well:
// https://developer.apple.com/documentation/foundation/numbers_data_and_basic_values?language=objc
#[doc(no_inline)]
pub use crate::ffi::{NSInteger, NSUInteger};

/// A value indicating that a requested item couldn’t be found or doesn’t exist.
///
/// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsnotfound?language=objc).
#[allow(non_upper_case_globals)]
pub const NSNotFound: NSInteger = crate::ffi::NSIntegerMax;

/// A number of seconds.
///
/// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nstimeinterval?language=objc).
pub type NSTimeInterval = c_double;

#[cfg(feature = "apple")]
#[link(name = "Foundation", kind = "framework")]
extern "C" {}

#[cfg(feature = "gnustep-1-7")]
#[link(name = "gnustep-base", kind = "dylib")]
extern "C" {}

#[doc(hidden)]
pub mod __ns_string;
mod array;
mod attributed_string;
mod bundle;
mod comparison_result;
mod copying;
mod data;
mod dictionary;
mod enumerator;
mod error;
mod exception;
mod geometry;
mod mutable_array;
mod mutable_attributed_string;
mod mutable_data;
mod mutable_dictionary;
mod mutable_set;
mod mutable_string;
mod number;
mod object;
mod process_info;
mod range;
mod set;
mod string;
mod thread;
// Temporarily disable testing UUID on macOS 10.7 until
#[cfg(not(macos_10_7))]
mod uuid;
mod value;
mod zone;

#[cfg(test)]
mod tests {
    use core::panic::{RefUnwindSafe, UnwindSafe};

    use super::*;
    use crate::rc::{Id, Owned, Shared};

    // We expect most Foundation types to be UnwindSafe and RefUnwindSafe,
    // since they follow Rust's usual mutability rules (&T = immutable).
    //
    // A _lot_ of Objective-C code out there would be subtly broken if e.g.
    // `NSString` wasn't exception safe!
    // As an example: -[NSArray objectAtIndex:] can throw, but it is still
    // perfectly valid to access the array after that!
    //
    // Note that e.g. `&mut NSMutableString` is still not exception safe, but
    // that is the entire idea of `UnwindSafe` (that if the object could have
    // been mutated, it is not exception safe).
    //
    // Also note that this is still just a speed bump, not actually part of
    // any unsafe contract; we really protect against it if something is not
    // exception safe, since `UnwindSafe` is a safe trait.
    fn assert_unwindsafe<T: UnwindSafe + RefUnwindSafe>() {}

    fn assert_auto_traits<T: Send + Sync + UnwindSafe + RefUnwindSafe>() {
        assert_unwindsafe::<T>();
    }

    #[test]
    fn send_sync_unwindsafe() {
        assert_auto_traits::<NSArray<NSString, Shared>>();
        assert_auto_traits::<NSArray<NSString, Owned>>();
        assert_auto_traits::<Id<NSArray<NSString, Shared>, Shared>>();
        assert_auto_traits::<Id<NSArray<NSString, Owned>, Shared>>();
        assert_auto_traits::<Id<NSArray<NSString, Shared>, Owned>>();
        assert_auto_traits::<Id<NSArray<NSString, Owned>, Owned>>();

        assert_auto_traits::<NSAttributedString>();
        assert_auto_traits::<NSComparisonResult>();
        assert_auto_traits::<NSData>();
        assert_auto_traits::<NSDictionary<NSString, NSString>>();
        assert_auto_traits::<NSSet<NSString, Shared>>();
        assert_auto_traits::<NSSet<NSString, Owned>>();
        assert_auto_traits::<Id<NSSet<NSString, Shared>, Shared>>();
        assert_auto_traits::<Id<NSSet<NSString, Owned>, Shared>>();
        assert_auto_traits::<Id<NSSet<NSString, Shared>, Owned>>();
        assert_auto_traits::<Id<NSSet<NSString, Owned>, Owned>>();
        // TODO: Figure out if Send + Sync is safe?
        // assert_auto_traits::<NSEnumerator<NSString>>();
        // assert_auto_traits::<NSFastEnumerator<NSArray<NSString, Shared>>>();
        assert_auto_traits::<NSError>();
        assert_auto_traits::<NSException>();
        assert_auto_traits::<CGFloat>();
        assert_auto_traits::<NSPoint>();
        assert_auto_traits::<NSRect>();
        assert_auto_traits::<NSSize>();
        assert_auto_traits::<NSMutableArray<NSString, Shared>>();
        assert_auto_traits::<NSMutableAttributedString>();
        assert_auto_traits::<NSMutableData>();
        assert_auto_traits::<NSMutableDictionary<NSString, NSString>>();
        assert_auto_traits::<NSMutableSet<NSString, Shared>>();
        assert_auto_traits::<NSMutableString>();
        assert_auto_traits::<NSNumber>();
        // assert_auto_traits::<NSObject>(); // Intentional
        assert_auto_traits::<NSProcessInfo>();
        assert_auto_traits::<NSRange>();
        assert_auto_traits::<NSString>();
        assert_unwindsafe::<MainThreadMarker>(); // Intentional
        assert_auto_traits::<NSThread>();
        #[cfg(not(macos_10_7))]
        assert_auto_traits::<NSUUID>();
        // assert_auto_traits::<NSValue>(); // Intentional
        assert_unwindsafe::<NSZone>(); // Intentional
    }
}
