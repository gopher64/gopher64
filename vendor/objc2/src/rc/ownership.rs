use core::fmt::Debug;
use core::hash::Hash;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::AutoreleaseSafe;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Never {}

/// A type used to mark that a struct owns the object(s) it contains,
/// so it has the sole references to them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Owned {
    inner: Never,
}

/// A type used to mark that the object(s) a struct contains are shared,
/// so there may be other references to them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Shared {
    inner: Never,
}

mod private {
    pub trait Sealed {}

    impl Sealed for super::Owned {}
    impl Sealed for super::Shared {}
}

/// A type that marks what type of ownership a struct has over the object(s)
/// it contains; specifically, either [`Owned`] or [`Shared`].
///
/// This trait is sealed and not meant to be implemented outside of the this
/// crate.
pub trait Ownership:
    private::Sealed
    // Special
    + 'static
    + Sized
    // Auto-traits
    + Send
    + Sync
    + Unpin
    + UnwindSafe
    + RefUnwindSafe
    // Derived
    + Clone
    + Copy
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Hash
    + Debug
    // Custom
    + AutoreleaseSafe
{
}

impl Ownership for Owned {}
impl Ownership for Shared {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_ownership_traits() {
        fn assert_partialeq<T: PartialEq>() {}

        assert_partialeq::<Shared>();
        assert_partialeq::<Owned>();

        fn test_ownership_implies_partialeq<O: Ownership>() {
            assert_partialeq::<O>();
        }

        test_ownership_implies_partialeq::<Shared>();
        test_ownership_implies_partialeq::<Owned>();
    }
}
