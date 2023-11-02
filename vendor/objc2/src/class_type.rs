use crate::runtime::Class;
use crate::Message;

/// Marks types that represent specific classes.
///
/// Usually it is enough to generically know that a type is messageable, e.g.
/// [`rc::Id`][crate::rc::Id] works with any type that implements the
/// [`Message`] trait. But often, you have an object that you know represents
/// a specific Objective-C class - this trait allows you to communicate that
/// to the rest of the type-system.
///
/// This is implemented automatically by the
/// [`declare_class!`][crate::declare_class] and
/// [`extern_class!`][crate::extern_class] macros.
///
///
/// # Safety
///
/// The class returned by [`Self::class`] must be a subclass of the class that
/// [`Self::Super`] represents, and `as_super`/`as_super_mut` must be
/// implemented correctly. Finally [`Self::NAME`] must be correct.
///
/// In pseudocode:
/// ```ignore
/// Self::class().superclass() == <Self::Super as ClassType>::class()
/// Self::class().name() == Self::NAME
/// ```
///
///
/// # Examples
///
/// Use the trait to access the [`Class`] of different objects.
///
/// ```
/// # #[cfg(feature = "gnustep-1-7")]
/// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
/// use objc2::ClassType;
/// use objc2::foundation::NSObject;
/// // Get a class object representing `NSObject`
/// let cls = <NSObject as ClassType>::class(); // Or just `NSObject::class()`
/// ```
///
/// Use the [`extern_class!`][crate::extern_class] macro to implement this
/// trait for a type.
///
/// ```ignore
/// use objc2::{extern_class, ClassType};
///
/// extern_class!(
///     struct MyClass;
///
///     unsafe impl ClassType for MyClass {
///         type Super = NSObject;
///     }
/// );
///
/// let cls = MyClass::class();
/// ```
pub unsafe trait ClassType: Message {
    /// The superclass of this class.
    ///
    /// If you have implemented [`Deref`] for your type, it is highly
    /// recommended that this is equal to [`Deref::Target`].
    ///
    /// This may be [`runtime::Object`] if the class is a root class.
    ///
    /// [`Deref`]: std::ops::Deref
    /// [`Deref::Target`]: std::ops::Deref::Target
    /// [`runtime::Object`]: crate::runtime::Object
    type Super: Message;

    /// The name of the Objective-C class that this type represents.
    const NAME: &'static str;

    /// Get a reference to the Objective-C class that this type represents.
    ///
    /// May register the class with the runtime if it wasn't already.
    ///
    ///
    /// # Panics
    ///
    /// This may panic if something went wrong with getting or declaring the
    /// class, e.g. if the program is not properly linked to the framework
    /// that defines the class.
    fn class() -> &'static Class;

    /// Get an immutable reference to the superclass.
    // Note: It'd be safe to provide a default impl using transmute here if
    // we wanted to!
    fn as_super(&self) -> &Self::Super;

    /// Get a mutable reference to the superclass.
    fn as_super_mut(&mut self) -> &mut Self::Super;
}
