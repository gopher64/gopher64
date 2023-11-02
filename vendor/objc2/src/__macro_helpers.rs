use crate::__sel_inner;
use crate::rc::{Allocated, Id, Ownership};
use crate::runtime::{Class, Object, Sel};
use crate::{Message, MessageArguments, MessageReceiver};

pub use crate::cache::CachedClass;
pub use crate::cache::CachedSel;

pub use core::borrow::{Borrow, BorrowMut};
pub use core::cell::UnsafeCell;
pub use core::convert::{AsMut, AsRef};
pub use core::mem::size_of;
pub use core::ops::{Deref, DerefMut};
pub use core::option::Option::{self, None, Some};
pub use core::primitive::{bool, str, u8};
pub use core::ptr::drop_in_place;
pub use core::{compile_error, concat, panic, stringify};
// TODO: Use `core::cell::LazyCell`
pub use std::sync::Once;

// Common selectors.
//
// These are put here to deduplicate the cached selector, and when using
// `unstable-static-sel`, the statics.
//
// Note that our assembly tests of `unstable-static-sel-inlined` output a GOT
// entry for such accesses, but that is just a limitation of our tests - the
// actual assembly is as one would expect.

#[inline]
pub fn alloc() -> Sel {
    // SAFETY: Must have NUL byte
    __sel_inner!("alloc\0", "alloc")
}
#[inline]
pub fn init() -> Sel {
    // SAFETY: Must have NUL byte
    __sel_inner!("init\0", "init")
}
#[inline]
pub fn new() -> Sel {
    // SAFETY: Must have NUL byte
    __sel_inner!("new\0", "new")
}

/// Helper for specifying the retain semantics for a given selector family.
///
/// Note that we can't actually check if a method is in a method family; only
/// whether the _selector_ is in a _selector_ family.
///
/// The slight difference here is:
/// - The method may be annotated with the `objc_method_family` attribute,
///   which would cause it to be in a different family. That this is not the
///   case is part of the `unsafe` contract of `msg_send_id!`.
/// - The method may not obey the added restrictions of the method family.
///   The added restrictions are:
///   - `new`, `alloc`, `copy` and `mutableCopy`: The method must return a
///     retainable object pointer type - we ensure this by making
///     `message_send_id` return `Id`.
///   - `init`: The method must be an instance method and must return an
///     Objective-C pointer type - We ensure this by taking `Id<T, O>`, which
///     means it can't be a class method!
///
/// While we're at it, we also limit a few other things to help the user out,
/// like only allowing `&Class` in `new` - this is not strictly required by
/// ARC though!
///
/// <https://clang.llvm.org/docs/AutomaticReferenceCounting.html#retainable-object-pointers-as-operands-and-arguments>
pub struct RetainSemantics<
    // `new` family
    const NEW: bool,
    // `alloc` family
    const ALLOC: bool,
    // `init` family
    const INIT: bool,
    // `copy` or `mutableCopy` family
    const COPY_OR_MUT_COPY: bool,
> {}

type New = RetainSemantics<true, false, false, false>;
type Alloc = RetainSemantics<false, true, false, false>;
type Init = RetainSemantics<false, false, true, false>;
type CopyOrMutCopy = RetainSemantics<false, false, false, true>;
type Other = RetainSemantics<false, false, false, false>;

pub trait MsgSendId<T, U: ?Sized, O: Ownership> {
    unsafe fn send_message_id<A: MessageArguments, R: MaybeUnwrap<U, O>>(
        obj: T,
        sel: Sel,
        args: A,
    ) -> R;
}

impl<T: ?Sized + Message, O: Ownership> MsgSendId<&'_ Class, T, O> for New {
    #[inline]
    #[track_caller]
    unsafe fn send_message_id<A: MessageArguments, R: MaybeUnwrap<T, O>>(
        cls: &Class,
        sel: Sel,
        args: A,
    ) -> R {
        // SAFETY: Checked by caller
        let obj = unsafe { MessageReceiver::send_message(cls, sel, args) };
        // SAFETY: The selector is `new`, so this has +1 retain count
        let obj = unsafe { Id::new(obj) };
        R::maybe_unwrap::<Self>(obj, (cls, sel))
    }
}

impl<T: ?Sized + Message, O: Ownership> MsgSendId<&'_ Class, Allocated<T>, O> for Alloc {
    #[inline]
    #[track_caller]
    unsafe fn send_message_id<A: MessageArguments, R: MaybeUnwrap<Allocated<T>, O>>(
        cls: &Class,
        sel: Sel,
        args: A,
    ) -> R {
        // SAFETY: Checked by caller
        let obj = unsafe { MessageReceiver::send_message(cls, sel, args) };
        // SAFETY: The selector is `alloc`, so this has +1 retain count
        let obj = unsafe { Id::new_allocated(obj) };
        R::maybe_unwrap::<Self>(obj, (cls, sel))
    }
}

impl<T: ?Sized + Message, O: Ownership> MsgSendId<Option<Id<Allocated<T>, O>>, T, O> for Init {
    #[inline]
    #[track_caller]
    unsafe fn send_message_id<A: MessageArguments, R: MaybeUnwrap<T, O>>(
        obj: Option<Id<Allocated<T>, O>>,
        sel: Sel,
        args: A,
    ) -> R {
        let ptr = Id::option_into_ptr(obj.map(|obj| unsafe { Id::assume_init(obj) }));
        // SAFETY: `ptr` may be null here, but that's fine since the return
        // is `*mut T`, which is one of the few types where messages to nil is
        // allowed.
        //
        // We do this for efficiency, to avoid having a branch after every
        // `alloc`, that the user did not intend.
        let obj = unsafe { MessageReceiver::send_message(ptr, sel, args) };
        // SAFETY: The selector is `init`, so this has +1 retain count
        let obj = unsafe { Id::new(obj) };
        R::maybe_unwrap::<Self>(obj, (ptr.cast(), sel))
    }
}

impl<T: MessageReceiver, U: ?Sized + Message, O: Ownership> MsgSendId<T, U, O> for CopyOrMutCopy {
    #[inline]
    #[track_caller]
    unsafe fn send_message_id<A: MessageArguments, R: MaybeUnwrap<U, O>>(
        obj: T,
        sel: Sel,
        args: A,
    ) -> R {
        // SAFETY: Checked by caller
        let obj = unsafe { MessageReceiver::send_message(obj, sel, args) };
        // SAFETY: The selector is `copy` or `mutableCopy`, so this has +1
        // retain count
        let obj = unsafe { Id::new(obj) };
        R::maybe_unwrap::<Self>(obj, ())
    }
}

impl<T: MessageReceiver, U: Message, O: Ownership> MsgSendId<T, U, O> for Other {
    #[inline]
    #[track_caller]
    unsafe fn send_message_id<A: MessageArguments, R: MaybeUnwrap<U, O>>(
        obj: T,
        sel: Sel,
        args: A,
    ) -> R {
        let ptr = obj.__as_raw_receiver();
        // SAFETY: Checked by caller
        let obj = unsafe { MessageReceiver::send_message(ptr, sel, args) };
        // All code between the message send and the `retain_autoreleased`
        // must be able to be optimized away for this to work.

        // SAFETY: The selector is not `new`, `alloc`, `init`, `copy` nor
        // `mutableCopy`, so the object must be manually retained.
        let obj = unsafe { Id::retain_autoreleased(obj) };

        // SAFETY: The object is still valid after a message send to a
        // normal method - it would not be if the method was `init`.
        R::maybe_unwrap::<Self>(obj, (unsafe { ptr.as_ref() }, sel))
    }
}

pub trait MaybeUnwrap<T: ?Sized, O: Ownership> {
    fn maybe_unwrap<'a, Failed: MsgSendIdFailed<'a>>(
        obj: Option<Id<T, O>>,
        args: Failed::Args,
    ) -> Self;
}

impl<T: ?Sized, O: Ownership> MaybeUnwrap<T, O> for Option<Id<T, O>> {
    #[inline]
    #[track_caller]
    fn maybe_unwrap<'a, Failed: MsgSendIdFailed<'a>>(
        obj: Option<Id<T, O>>,
        _args: Failed::Args,
    ) -> Self {
        obj
    }
}

impl<T: ?Sized, O: Ownership> MaybeUnwrap<T, O> for Id<T, O> {
    #[inline]
    #[track_caller]
    fn maybe_unwrap<'a, Failed: MsgSendIdFailed<'a>>(
        obj: Option<Id<T, O>>,
        args: Failed::Args,
    ) -> Self {
        match obj {
            Some(obj) => obj,
            None => Failed::failed(args),
        }
    }
}

// Note: It would have been much easier to do this kind of thing using
// closures, but then `track_caller` doesn't work properly!
pub trait MsgSendIdFailed<'a> {
    type Args;

    fn failed(args: Self::Args) -> !;
}

impl<'a> MsgSendIdFailed<'a> for New {
    type Args = (&'a Class, Sel);

    #[cold]
    #[track_caller]
    fn failed((cls, sel): Self::Args) -> ! {
        if sel == new() {
            panic!("failed creating new instance of {:?}", cls)
        } else {
            panic!("failed creating new instance using +[{:?} {:?}]", cls, sel)
        }
    }
}

impl<'a> MsgSendIdFailed<'a> for Alloc {
    type Args = (&'a Class, Sel);

    #[cold]
    #[track_caller]
    fn failed((cls, sel): Self::Args) -> ! {
        if sel == alloc() {
            panic!("failed allocating {:?}", cls)
        } else {
            panic!("failed allocating with +[{:?} {:?}]", cls, sel)
        }
    }
}

impl MsgSendIdFailed<'_> for Init {
    type Args = (*const Object, Sel);

    #[cold]
    #[track_caller]
    fn failed((ptr, sel): Self::Args) -> ! {
        if ptr.is_null() {
            panic!("failed allocating object")
        } else {
            // We can't really display a more descriptive message here since the
            // object is consumed by `init` and may not be valid any more.
            if sel == init() {
                panic!("failed initializing object")
            } else {
                panic!("failed initializing object with -{:?}", sel)
            }
        }
    }
}

impl MsgSendIdFailed<'_> for CopyOrMutCopy {
    type Args = ();

    #[cold]
    #[track_caller]
    fn failed(_: Self::Args) -> ! {
        panic!("failed copying object")
    }
}

impl<'a> MsgSendIdFailed<'a> for Other {
    type Args = (Option<&'a Object>, Sel);

    #[cold]
    #[track_caller]
    fn failed((obj, sel): Self::Args) -> ! {
        if let Some(obj) = obj {
            let cls = obj.class();
            panic!(
                "unexpected NULL returned from {}[{:?} {:?}]",
                if cls.is_metaclass() { "+" } else { "-" },
                cls,
                sel,
            )
        } else {
            panic!("unexpected NULL {:?}; receiver was NULL", sel);
        }
    }
}

/// Checks whether a given selector is said to be in a given selector family.
///
/// <https://clang.llvm.org/docs/AutomaticReferenceCounting.html#arc-method-families>
pub const fn in_selector_family(mut selector: &[u8], mut family: &[u8]) -> bool {
    // Skip leading underscores from selector
    loop {
        selector = match selector {
            [b'_', rest @ ..] => rest,
            _ => break,
        }
    }

    // Compare each character
    loop {
        (selector, family) = match (selector, family) {
            // Remaining items
            ([s, selector @ ..], [f, family @ ..]) => {
                if *s == *f {
                    // Next iteration
                    (selector, family)
                } else {
                    // Family does not begin with selector
                    return false;
                }
            }
            // Equal
            ([], []) => {
                return true;
            }
            // Selector can't be part of familiy if smaller than it
            ([], _) => {
                return false;
            }
            // Remaining items in selector
            // -> ensure next character is not lowercase
            ([s, ..], []) => {
                return !s.is_ascii_lowercase();
            }
        }
    }
}

/// Helper struct for emitting the module info that macOS 32-bit requires.
///
/// <https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/CodeGen/CGObjCMac.cpp#L5211-L5234>
#[repr(C)]
pub struct ModuleInfo {
    version: usize,
    size: usize,
    name: *const u8,
    symtab: *const (),
}

// SAFETY: ModuleInfo is immutable.
unsafe impl Sync for ModuleInfo {}

impl ModuleInfo {
    /// This is hardcoded in clang as 7.
    const VERSION: usize = 7;

    pub const fn new(name: *const u8) -> Self {
        Self {
            version: Self::VERSION,
            size: core::mem::size_of::<Self>(),
            name,
            // We don't expose any symbols
            symtab: core::ptr::null(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::vec;
    use core::ptr;

    #[cfg(feature = "objc2-proc-macros")]
    use crate::__hash_idents;
    use crate::foundation::{NSDictionary, NSObject, NSString, NSValue, NSZone};
    use crate::rc::{Owned, RcTestObject, Shared, ThreadTestData};
    use crate::runtime::Object;
    use crate::{class, msg_send_id, ns_string, ClassType};

    #[test]
    fn test_new() {
        let _obj: Id<Object, Shared> = unsafe { msg_send_id![NSObject::class(), new] };
        let _obj: Option<Id<Object, Shared>> = unsafe { msg_send_id![NSObject::class(), new] };
    }

    #[test]
    // newScriptingObjectOfClass only available on macOS
    #[cfg_attr(not(all(feature = "apple", target_os = "macos")), ignore)]
    fn test_new_with_args() {
        let mut expected = ThreadTestData::current();

        let object_class = RcTestObject::class();
        let key = ns_string!("");
        let contents_value: *const Object = ptr::null();
        let properties: Id<NSDictionary<NSString, Object>, _> =
            NSDictionary::from_keys_and_objects::<NSString>(&[], vec![]);

        let _obj: Option<Id<Object, Shared>> = unsafe {
            msg_send_id![
                NSObject::class(),
                newScriptingObjectOfClass: object_class,
                forValueForKey: key,
                withContentsValue: contents_value,
                properties: &*properties,
            ]
        };
        expected.alloc += 1;
        expected.init += 1;
        expected.assert_current();
    }

    #[test]
    fn test_macro_alloc() {
        let mut expected = ThreadTestData::current();
        let cls = RcTestObject::class();

        let obj: Id<Allocated<RcTestObject>, Shared> = unsafe { msg_send_id![cls, alloc] };
        expected.alloc += 1;
        expected.assert_current();

        drop(obj);
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_alloc_with_zone() {
        let mut expected = ThreadTestData::current();
        let cls = RcTestObject::class();

        let zone: *const NSZone = ptr::null();
        let _obj: Id<Allocated<RcTestObject>, Owned> =
            unsafe { msg_send_id![cls, allocWithZone: zone] };
        expected.alloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_macro_init() {
        let mut expected = ThreadTestData::current();
        let cls = RcTestObject::class();

        let obj: Option<Id<Allocated<RcTestObject>, Shared>> = unsafe { msg_send_id![cls, alloc] };
        expected.alloc += 1;
        // Don't check allocation error
        let _obj: Id<RcTestObject, Shared> = unsafe { msg_send_id![obj, init] };
        expected.init += 1;
        expected.assert_current();

        let obj: Option<Id<Allocated<RcTestObject>, Shared>> = unsafe { msg_send_id![cls, alloc] };
        expected.alloc += 1;
        // Check allocation error before init
        let obj = obj.unwrap();
        let _obj: Id<RcTestObject, Shared> = unsafe { msg_send_id![Some(obj), init] };
        expected.init += 1;
        expected.assert_current();
    }

    #[test]
    fn test_macro() {
        let mut expected = ThreadTestData::current();
        let cls = RcTestObject::class();
        crate::rc::autoreleasepool(|_| {
            let _obj: Id<RcTestObject, Owned> = unsafe { msg_send_id![cls, new] };
            expected.alloc += 1;
            expected.init += 1;
            expected.assert_current();

            let obj = unsafe { msg_send_id![cls, alloc] };
            expected.alloc += 1;
            expected.assert_current();

            let obj: Id<RcTestObject, Owned> = unsafe { msg_send_id![obj, init] };
            expected.init += 1;
            expected.assert_current();

            let _copy: Id<RcTestObject, Shared> = unsafe { msg_send_id![&obj, copy] };
            expected.copy += 1;
            expected.alloc += 1;
            expected.init += 1;
            expected.assert_current();

            let _mutable_copy: Id<RcTestObject, Shared> =
                unsafe { msg_send_id![&obj, mutableCopy] };
            expected.mutable_copy += 1;
            expected.alloc += 1;
            expected.init += 1;
            expected.assert_current();

            let _self: Id<RcTestObject, Shared> = unsafe { msg_send_id![&obj, self] };
            expected.retain += 1;
            expected.assert_current();

            let _desc: Option<Id<RcTestObject, Shared>> =
                unsafe { msg_send_id![&obj, description] };
            expected.assert_current();
        });
        expected.release += 5;
        expected.dealloc += 4;
        expected.assert_current();
    }

    #[test]
    #[should_panic = "failed creating new instance of NSValue"]
    // GNUStep instead returns an invalid instance that panics on accesses
    #[cfg_attr(feature = "gnustep-1-7", ignore)]
    fn new_nsvalue_fails() {
        let _val: Id<NSValue, Shared> = unsafe { msg_send_id![NSValue::class(), new] };
    }

    #[test]
    #[should_panic = "failed creating new instance using +[RcTestObject newReturningNull]"]
    fn test_new_with_null() {
        let _obj: Id<RcTestObject, Owned> =
            unsafe { msg_send_id![RcTestObject::class(), newReturningNull] };
    }

    #[test]
    #[should_panic = "failed allocating with +[RcTestObject allocReturningNull]"]
    fn test_alloc_with_null() {
        let _obj: Id<Allocated<RcTestObject>, Owned> =
            unsafe { msg_send_id![RcTestObject::class(), allocReturningNull] };
    }

    #[test]
    #[should_panic = "failed initializing object with -initReturningNull"]
    fn test_init_with_null() {
        let obj: Option<Id<Allocated<RcTestObject>, Owned>> =
            unsafe { msg_send_id![RcTestObject::class(), alloc] };
        let _obj: Id<RcTestObject, Owned> = unsafe { msg_send_id![obj, initReturningNull] };
    }

    #[test]
    #[should_panic = "failed allocating object"]
    #[cfg(not(feature = "verify_message"))] // Does NULL receiver checks
    fn test_init_with_null_receiver() {
        let obj: Option<Id<Allocated<RcTestObject>, Owned>> = None;
        let _obj: Id<RcTestObject, Owned> = unsafe { msg_send_id![obj, init] };
    }

    #[test]
    #[should_panic = "failed copying object"]
    fn test_copy_with_null() {
        let obj = Id::into_shared(RcTestObject::new());
        let _obj: Id<RcTestObject, Shared> = unsafe { msg_send_id![&obj, copyReturningNull] };
    }

    #[test]
    #[should_panic = "unexpected NULL returned from -[RcTestObject methodReturningNull]"]
    fn test_normal_with_null() {
        let obj = Id::into_shared(RcTestObject::new());
        let _obj: Id<RcTestObject, Shared> = unsafe { msg_send_id![&obj, methodReturningNull] };
    }

    #[test]
    #[should_panic = "unexpected NULL description; receiver was NULL"]
    #[cfg(not(feature = "verify_message"))] // Does NULL receiver checks
    fn test_normal_with_null_receiver() {
        let obj: *const NSObject = ptr::null();
        let _obj: Id<NSString, Shared> = unsafe { msg_send_id![obj, description] };
    }

    #[test]
    fn test_in_selector_family() {
        // Common cases

        assert!(in_selector_family(b"alloc", b"alloc"));
        assert!(in_selector_family(b"allocWithZone:", b"alloc"));
        assert!(!in_selector_family(b"dealloc", b"alloc"));
        assert!(!in_selector_family(b"initialize", b"init"));
        assert!(!in_selector_family(b"decimalNumberWithDecimal:", b"init"));
        assert!(in_selector_family(b"initWithCapacity:", b"init"));
        assert!(in_selector_family(b"_initButPrivate:withParam:", b"init"));
        assert!(!in_selector_family(b"description", b"init"));
        assert!(!in_selector_family(b"inIT", b"init"));

        assert!(!in_selector_family(b"init", b"copy"));
        assert!(!in_selector_family(b"copyingStuff:", b"copy"));
        assert!(in_selector_family(b"copyWithZone:", b"copy"));
        assert!(!in_selector_family(b"initWithArray:copyItems:", b"copy"));
        assert!(in_selector_family(b"copyItemAtURL:toURL:error:", b"copy"));

        assert!(!in_selector_family(b"mutableCopying", b"mutableCopy"));
        assert!(in_selector_family(b"mutableCopyWithZone:", b"mutableCopy"));
        assert!(in_selector_family(b"mutableCopyWithZone:", b"mutableCopy"));

        assert!(in_selector_family(
            b"newScriptingObjectOfClass:forValueForKey:withContentsValue:properties:",
            b"new"
        ));
        assert!(in_selector_family(
            b"newScriptingObjectOfClass:forValueForKey:withContentsValue:properties:",
            b"new"
        ));
        assert!(!in_selector_family(b"newsstandAssetDownload", b"new"));

        // Trying to weed out edge-cases:

        assert!(in_selector_family(b"__abcDef", b"abc"));
        assert!(in_selector_family(b"_abcDef", b"abc"));
        assert!(in_selector_family(b"abcDef", b"abc"));
        assert!(in_selector_family(b"___a", b"a"));
        assert!(in_selector_family(b"__a", b"a"));
        assert!(in_selector_family(b"_a", b"a"));
        assert!(in_selector_family(b"a", b"a"));

        assert!(!in_selector_family(b"_abcdef", b"abc"));
        assert!(!in_selector_family(b"_abcdef", b"def"));
        assert!(!in_selector_family(b"_bcdef", b"abc"));
        assert!(!in_selector_family(b"a_bc", b"abc"));
        assert!(!in_selector_family(b"abcdef", b"abc"));
        assert!(!in_selector_family(b"abcdef", b"def"));
        assert!(!in_selector_family(b"abcdef", b"abb"));
        assert!(!in_selector_family(b"___", b"a"));
        assert!(!in_selector_family(b"_", b"a"));
        assert!(!in_selector_family(b"", b"a"));

        assert!(in_selector_family(b"copy", b"copy"));
        assert!(in_selector_family(b"copy:", b"copy"));
        assert!(in_selector_family(b"copyMe", b"copy"));
        assert!(in_selector_family(b"_copy", b"copy"));
        assert!(in_selector_family(b"_copy:", b"copy"));
        assert!(in_selector_family(b"_copyMe", b"copy"));
        assert!(!in_selector_family(b"copying", b"copy"));
        assert!(!in_selector_family(b"copying:", b"copy"));
        assert!(!in_selector_family(b"_copying", b"copy"));
        assert!(!in_selector_family(b"Copy", b"copy"));
        assert!(!in_selector_family(b"COPY", b"copy"));

        // Empty family (not supported)
        assert!(in_selector_family(b"___", b""));
        assert!(in_selector_family(b"__", b""));
        assert!(in_selector_family(b"_", b""));
        assert!(in_selector_family(b"", b""));
        assert!(!in_selector_family(b"_a", b""));
        assert!(!in_selector_family(b"a", b""));
        assert!(in_selector_family(b"_A", b""));
        assert!(in_selector_family(b"A", b""));
    }

    mod test_trait_disambugated {
        use super::*;

        trait Abc {
            fn send_message_id() {}
        }

        impl<T> Abc for T {}

        #[test]
        fn test_macro_still_works() {
            let cls = class!(NSObject);
            let _obj: Id<Object, Owned> = unsafe { msg_send_id![cls, new] };
        }
    }

    #[test]
    #[cfg(feature = "objc2-proc-macros")]
    fn hash_idents_different() {
        assert_ne!(__hash_idents!(abc), __hash_idents!(def));
    }

    #[test]
    #[cfg(feature = "objc2-proc-macros")]
    fn hash_idents_same_no_equal() {
        assert_ne!(__hash_idents!(abc), __hash_idents!(abc));
        assert_ne!(__hash_idents!(abc def ghi), __hash_idents!(abc def ghi));
    }

    #[test]
    #[cfg(feature = "objc2-proc-macros")]
    fn hash_idents_exact_same_ident() {
        macro_rules! x {
            ($x:ident) => {
                (__hash_idents!($x), __hash_idents!($x))
            };
        }
        let (ident1, ident2) = x!(abc);
        // This is a limitation of `__hash_idents`, ideally we'd like these
        // to be different!
        assert_eq!(ident1, ident2);
    }

    #[test]
    #[cfg_attr(
        not(all(feature = "apple", target_os = "macos", target_arch = "x86")),
        ignore = "Only relevant on macOS 32-bit"
    )]
    fn ensure_size_of_module_info() {
        assert_eq!(core::mem::size_of::<ModuleInfo>(), 16);
    }
}
