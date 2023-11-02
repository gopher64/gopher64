//! Read from the global pasteboard, and write a new string into it.
//!
//! Works on macOS 10.7+
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(all(feature = "apple", target_os = "macos")), allow(unused))]

use std::mem::ManuallyDrop;

use objc2::foundation::{NSArray, NSDictionary, NSInteger, NSObject, NSString};
use objc2::rc::{Id, Shared};
use objc2::runtime::{Class, Object};
use objc2::{extern_class, msg_send, msg_send_id, ClassType};

type NSPasteboardType = NSString;
type NSPasteboardReadingOptionKey = NSString;

#[cfg(all(feature = "apple", target_os = "macos"))]
#[link(name = "AppKit", kind = "framework")]
extern "C" {
    /// <https://developer.apple.com/documentation/appkit/nspasteboardtypestring?language=objc>
    static NSPasteboardTypeString: Option<&'static NSPasteboardType>;
}

#[cfg(all(feature = "apple", target_os = "macos"))]
extern_class!(
    /// <https://developer.apple.com/documentation/appkit/nspasteboard?language=objc>
    pub struct NSPasteboard;

    // SAFETY: NSPasteboard actually inherits from NSObject.
    unsafe impl ClassType for NSPasteboard {
        type Super = NSObject;
    }
);

#[cfg(all(feature = "apple", target_os = "macos"))]
impl NSPasteboard {
    /// We return a `Shared` `Id` because `general` can easily be called
    /// again, and it would return the same object, resulting in two aliasing
    /// mutable references if we returned an `Owned` Id.
    ///
    /// Besides, even if we could prevent this, there might be Objective-C
    /// code somewhere else that accesses this instance.
    ///
    /// TODO: Is this safe to call outside the main thread?
    ///
    /// <https://developer.apple.com/documentation/appkit/nspasteboard/1530091-generalpasteboard?language=objc>
    pub fn general() -> Id<Self, Shared> {
        unsafe { msg_send_id![Self::class(), generalPasteboard] }
    }

    /// Simple, straightforward implementation
    ///
    /// <https://developer.apple.com/documentation/appkit/nspasteboard/1533566-stringfortype?language=objc>
    pub fn text_impl_1(&self) -> Id<NSString, Shared> {
        let s = unsafe { NSPasteboardTypeString }.unwrap();
        unsafe { msg_send_id![self, stringForType: s] }
    }

    /// More complex implementation using `readObjectsForClasses:options:`,
    /// intended to show some how some patterns might require more knowledge
    /// of nitty-gritty details.
    ///
    /// <https://developer.apple.com/documentation/appkit/nspasteboard/1524454-readobjectsforclasses?language=objc>
    pub fn text_impl_2(&self) -> Id<NSString, Shared> {
        // The NSPasteboard API is a bit weird, it requires you to pass
        // classes as objects, which `objc2::foundation::NSArray` was not
        // really made for - so we convert the class to an `Object` type
        // instead. Also, we wrap it in `ManuallyDrop` because I'm not sure
        // how classes handle `release` calls?
        //
        // TODO: Investigate and find a better way to express this in objc2.
        let string_classes: ManuallyDrop<[Id<Object, Shared>; 1]> = {
            let cls: *const Class = NSString::class();
            let cls = cls as *mut Object;
            unsafe { ManuallyDrop::new([Id::new(cls).unwrap()]) }
        };
        // Temporary, see https://github.com/rust-lang/rust-clippy/issues/9101
        #[allow(unknown_lints, clippy::explicit_auto_deref)]
        let class_array = NSArray::from_slice(&*string_classes);
        let options = NSDictionary::new();
        let objects = unsafe { self.read_objects_for_classes(&class_array, &options) };

        // TODO: Should perhaps return Id<Object, Shared>?
        let ptr: *const Object = objects.first().unwrap();

        // And this part is weird as well, since we now have to convert the
        // object into an NSString, which we know it to be since that's what
        // we told `readObjectsForClasses:options:`.
        let ptr = ptr as *mut NSString;
        unsafe { Id::retain(ptr) }.unwrap()
    }

    /// Defined here to make it easier to declare which types are expected.
    /// This is a common pattern that I can wholeheartedly recommend!
    ///
    /// SAFETY: `class_array` must contain classes!
    unsafe fn read_objects_for_classes(
        &self,
        class_array: &NSArray<Object, Shared>,
        options: &NSDictionary<NSPasteboardReadingOptionKey, Object>,
    ) -> Id<NSArray<Object, Shared>, Shared> {
        unsafe { msg_send_id![self, readObjectsForClasses: class_array, options: options] }
    }

    /// This takes `&self` even though `writeObjects:` would seem to mutate
    /// the pasteboard. "What is going on?", you might rightfully ask!
    ///
    /// We do this because we can't soundly get a mutable reference to the
    /// global `NSPasteboard` instance, see [`NSPasteboard::general`].
    ///
    /// This is sound because `NSPasteboard` contains `NSObject`, which in
    /// turn contains `UnsafeCell`, allowing interior mutability.
    ///
    /// <https://developer.apple.com/documentation/appkit/nspasteboard/1533599-clearcontents?language=objc>
    /// <https://developer.apple.com/documentation/appkit/nspasteboard/1525945-writeobjects?language=objc>
    pub fn set_text(&self, text: Id<NSString, Shared>) {
        let _: NSInteger = unsafe { msg_send![self, clearContents] };
        let string_array = NSArray::from_slice(&[text]);
        let res: bool = unsafe { msg_send![self, writeObjects: &*string_array] };
        if !res {
            panic!("Failed writing to pasteboard");
        }
    }
}

#[cfg(all(feature = "apple", target_os = "macos"))]
fn main() {
    let pasteboard = NSPasteboard::general();
    let impl_1 = pasteboard.text_impl_1();
    let impl_2 = pasteboard.text_impl_2();
    println!("Pasteboard text from implementation 1 was: {:?}", impl_1);
    println!("Pasteboard text from implementation 2 was: {:?}", impl_2);
    assert_eq!(impl_1, impl_2);

    let s = NSString::from_str("Hello, world!");
    pasteboard.set_text(s.clone());
    println!("Now the pasteboard text should be: {:?}", s);
    assert_eq!(s, pasteboard.text_impl_1());
}

#[cfg(not(all(feature = "apple", target_os = "macos")))]
fn main() {
    panic!("this example only works on macOS");
}
