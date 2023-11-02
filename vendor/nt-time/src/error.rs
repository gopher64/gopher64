// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Error types for this crate.

use core::fmt;

/// The error type indicating that a [`OffsetDateTime`](time::OffsetDateTime)
/// was out of range.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OffsetDateTimeRangeError;

impl fmt::Display for OffsetDateTimeRangeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "date and time is out of range for `OffsetDateTime`")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OffsetDateTimeRangeError {}

/// The error type indicating that a [`FileTime`](crate::FileTime) was out of
/// range.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileTimeRangeError(FileTimeRangeErrorKind);

impl FileTimeRangeError {
    #[inline]
    pub(crate) const fn new(kind: FileTimeRangeErrorKind) -> Self {
        Self(kind)
    }

    /// Returns the corresponding [`FileTimeRangeErrorKind`] for this error.
    #[must_use]
    #[inline]
    pub const fn kind(self) -> FileTimeRangeErrorKind {
        self.0
    }
}

impl fmt::Display for FileTimeRangeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind().fmt(f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FileTimeRangeError {}

/// Details of the error that caused a [`FileTimeRangeError`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileTimeRangeErrorKind {
    /// Value was negative.
    ///
    /// This means the date and time was before "1601-01-01 00:00:00 UTC".
    Negative,

    /// Value was too big to be represented as [`FileTime`](crate::FileTime).
    ///
    /// This means the date and time was after "+60056-05-28 05:36:10.955161500
    /// UTC".
    Overflow,
}

impl fmt::Display for FileTimeRangeErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Negative => {
                write!(f, "date and time is before `1601-01-01 00:00:00 UTC`")
            }
            Self::Overflow => {
                write!(
                    f,
                    "date and time is after `+60056-05-28 05:36:10.955161500 UTC`"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_offset_date_time_range_error() {
        assert_eq!(OffsetDateTimeRangeError.clone(), OffsetDateTimeRangeError);
    }

    #[test]
    fn copy_offset_date_time_range_error() {
        let a = OffsetDateTimeRangeError;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn debug_offset_date_time_range_error() {
        assert_eq!(
            format!("{OffsetDateTimeRangeError:?}"),
            "OffsetDateTimeRangeError"
        );
    }

    #[test]
    fn offset_date_time_range_error_equality() {
        assert_eq!(OffsetDateTimeRangeError, OffsetDateTimeRangeError);
    }

    #[test]
    fn display_offset_date_time_range_error() {
        assert_eq!(
            format!("{OffsetDateTimeRangeError}"),
            "date and time is out of range for `OffsetDateTime`"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn source_offset_date_time_range_error() {
        use std::error::Error;

        assert!(OffsetDateTimeRangeError.source().is_none());
    }

    #[test]
    fn clone_file_time_range_error() {
        assert_eq!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Negative).clone(),
            FileTimeRangeError::new(FileTimeRangeErrorKind::Negative)
        );
        assert_eq!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow).clone(),
            FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow)
        );
    }

    #[test]
    fn copy_file_time_range_error() {
        {
            let a = FileTimeRangeError::new(FileTimeRangeErrorKind::Negative);
            let b = a;
            assert_eq!(a, b);
        }

        {
            let a = FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow);
            let b = a;
            assert_eq!(a, b);
        }
    }

    #[test]
    fn debug_file_time_range_error() {
        assert_eq!(
            format!(
                "{:?}",
                FileTimeRangeError::new(FileTimeRangeErrorKind::Negative)
            ),
            "FileTimeRangeError(Negative)"
        );
        assert_eq!(
            format!(
                "{:?}",
                FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow)
            ),
            "FileTimeRangeError(Overflow)"
        );
    }

    #[test]
    fn file_time_range_error_equality() {
        assert_eq!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Negative),
            FileTimeRangeError::new(FileTimeRangeErrorKind::Negative)
        );
        assert_ne!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Negative),
            FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow)
        );
        assert_ne!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow),
            FileTimeRangeError::new(FileTimeRangeErrorKind::Negative)
        );
        assert_eq!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow),
            FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow)
        );
    }

    #[test]
    fn kind_file_time_range_error() {
        assert_eq!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Negative).kind(),
            FileTimeRangeErrorKind::Negative
        );
        assert_eq!(
            FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow).kind(),
            FileTimeRangeErrorKind::Overflow
        );
    }

    #[test]
    fn display_file_time_range_error() {
        assert_eq!(
            format!(
                "{}",
                FileTimeRangeError::new(FileTimeRangeErrorKind::Negative)
            ),
            "date and time is before `1601-01-01 00:00:00 UTC`"
        );
        assert_eq!(
            format!(
                "{}",
                FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow)
            ),
            "date and time is after `+60056-05-28 05:36:10.955161500 UTC`"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn source_file_time_range_error() {
        use std::error::Error;

        assert!(FileTimeRangeError::new(FileTimeRangeErrorKind::Negative)
            .source()
            .is_none());
        assert!(FileTimeRangeError::new(FileTimeRangeErrorKind::Overflow)
            .source()
            .is_none());
    }

    #[test]
    fn clone_file_time_range_error_kind() {
        assert_eq!(
            FileTimeRangeErrorKind::Negative.clone(),
            FileTimeRangeErrorKind::Negative
        );
        assert_eq!(
            FileTimeRangeErrorKind::Overflow.clone(),
            FileTimeRangeErrorKind::Overflow
        );
    }

    #[test]
    fn copy_file_time_range_error_kind() {
        {
            let a = FileTimeRangeErrorKind::Negative;
            let b = a;
            assert_eq!(a, b);
        }

        {
            let a = FileTimeRangeErrorKind::Overflow;
            let b = a;
            assert_eq!(a, b);
        }
    }

    #[test]
    fn debug_file_time_range_error_kind() {
        assert_eq!(
            format!("{:?}", FileTimeRangeErrorKind::Negative),
            "Negative"
        );
        assert_eq!(
            format!("{:?}", FileTimeRangeErrorKind::Overflow),
            "Overflow"
        );
    }

    #[test]
    fn file_time_range_error_kind_equality() {
        assert_eq!(
            FileTimeRangeErrorKind::Negative,
            FileTimeRangeErrorKind::Negative
        );
        assert_ne!(
            FileTimeRangeErrorKind::Negative,
            FileTimeRangeErrorKind::Overflow
        );
        assert_ne!(
            FileTimeRangeErrorKind::Overflow,
            FileTimeRangeErrorKind::Negative
        );
        assert_eq!(
            FileTimeRangeErrorKind::Overflow,
            FileTimeRangeErrorKind::Overflow
        );
    }

    #[test]
    fn display_file_time_range_error_kind() {
        assert_eq!(
            format!("{}", FileTimeRangeErrorKind::Negative),
            "date and time is before `1601-01-01 00:00:00 UTC`"
        );
        assert_eq!(
            format!("{}", FileTimeRangeErrorKind::Overflow),
            "date and time is after `+60056-05-28 05:36:10.955161500 UTC`"
        );
    }
}
