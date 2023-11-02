//! Helper traits for Id.

use super::{Id, Owned, Ownership};
use crate::Message;

/// Helper trait for functionality on slices containing [`Id`]s.
pub trait SliceId {
    /// The type of the items in the slice.
    type Item: ?Sized;

    /// Convert a slice of [`Id`]s into a slice of references.
    fn as_slice_ref(&self) -> &[&Self::Item];

    /// Convert a mutable slice of [`Id`]s into a mutable slice of references.
    fn as_slice_mut(&mut self) -> &mut [&Self::Item];
}

/// Helper trait for functionality on slices containing owned [`Id`]s.
pub trait SliceIdMut: SliceId {
    /// Convert a mutable slice of mutable [`Id`]s into a mutable slice of
    /// mutable references.
    fn as_mut_slice_mut(&mut self) -> &mut [&mut Self::Item];
}

impl<T: Message + ?Sized, O: Ownership> SliceId for [Id<T, O>] {
    type Item = T;

    fn as_slice_ref(&self) -> &[&T] {
        let ptr = self as *const Self as *const [&T];
        // SAFETY: Id<T, O> and &T have the same memory layout. Further safety
        // follows from `Deref` impl.
        unsafe { ptr.as_ref().unwrap_unchecked() }
    }

    fn as_slice_mut(&mut self) -> &mut [&T] {
        let ptr = self as *mut Self as *mut [&T];
        // SAFETY: Id<T, O> and &T have the same memory layout. Further safety
        // follows from `Deref` impl.
        unsafe { ptr.as_mut().unwrap_unchecked() }
    }
}

impl<T: Message + ?Sized> SliceIdMut for [Id<T, Owned>] {
    fn as_mut_slice_mut(&mut self) -> &mut [&mut T] {
        let ptr = self as *mut Self as *mut [&mut T];
        // SAFETY: Id<T, O> and &mut T have the same memory layout, and the
        // `Id` is `Owned` so we're allowed to hand out mutable references.
        // Further safety follows from `DerefMut` impl.
        unsafe { ptr.as_mut().unwrap_unchecked() }
    }
}

/// Helper trait to implement [`Default`] on types whoose default value is an
/// [`Id`].
// TODO: Maybe make this `unsafe` and provide a default implementation?
pub trait DefaultId {
    /// Indicates whether the default value is mutable or immutable.
    type Ownership: Ownership;

    /// The default [`Id`] for a type.
    ///
    /// On most objects the implementation would just be sending a message to
    /// the `new` selector.
    fn default_id() -> Id<Self, Self::Ownership>;
}

impl<T: DefaultId + ?Sized> Default for Id<T, T::Ownership> {
    #[inline]
    fn default() -> Self {
        T::default_id()
    }
}
