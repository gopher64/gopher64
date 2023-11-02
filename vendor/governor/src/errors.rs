use std::fmt;

/// Error indicating that the number of cells tested (the first
/// argument) is larger than the bucket's capacity.
///
/// This means the decision can never have a conforming result. The
/// argument gives the maximum number of cells that could ever have a
/// conforming result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsufficientCapacity(pub u32);

impl fmt::Display for InsufficientCapacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "required number of cells {} exceeds bucket's capacity",
            self.0
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InsufficientCapacity {}

#[cfg(all(feature = "std", test))]
mod test {
    use super::*;

    #[test]
    fn coverage() {
        let display_output = format!("{}", InsufficientCapacity(3));
        assert!(display_output.contains("3"));
        let debug_output = format!("{:?}", InsufficientCapacity(3));
        assert!(debug_output.contains("3"));
        assert_eq!(InsufficientCapacity(3), InsufficientCapacity(3));
    }
}
