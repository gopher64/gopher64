/// A marker type that can be used within [`Id`] to indicate that the object
/// has been allocated but not initialized.
///
/// The reason we use `Option<Id<Allocated<T>, O>>` instead of just `*mut T`
/// is:
/// - To allow releasing allocated objects, e.g. in the face of panics.
/// - To safely know the object is valid (albeit uninitialized).
/// - To allow specifying ownership.
///
/// [`Id`]: crate::rc::Id
#[repr(transparent)]
#[derive(Debug)]
pub struct Allocated<T: ?Sized>(T);

// Explicitly don't implement `Deref`, `Message` nor `RefEncode`!
