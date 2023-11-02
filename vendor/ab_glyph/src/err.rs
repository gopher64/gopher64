use core::fmt;

/// Invalid font data error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvalidFont;

impl fmt::Display for InvalidFont {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InvalidFont")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidFont {}
