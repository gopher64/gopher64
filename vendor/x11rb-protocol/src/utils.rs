//! Utility functions that are not specific to X11.
//!
//! # RawFdContainer
//!
//! [`RawFdContainer`] is a variant of [`std::os::unix::io::RawFd`] with ownership semantics. This
//! means that the `RawFd` will be closed when the `RawFdContainer` is dropped.
//!
//! On non-`cfg(unix)`-systems, this is an empty type without methods. It still exists as a type so
//! that it can appear in interfaces, but it is not actually possible to construct an instance of
//! `RawFdContainer`.

#[cfg(all(feature = "std", unix))]
mod raw_fd_container {
    use std::mem::forget;
    use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

    /// A simple wrapper around RawFd that closes the fd on drop.
    ///
    /// On non-unix systems, this type is empty and does not provide
    /// any method.
    #[derive(Debug, Hash, PartialEq, Eq)]
    pub struct RawFdContainer(RawFd);

    impl Drop for RawFdContainer {
        fn drop(&mut self) {
            let _ = nix::unistd::close(self.0);
        }
    }

    impl RawFdContainer {
        /// Create a new `RawFdContainer` for the given `RawFd`.
        ///
        /// The `RawFdContainer` takes ownership of the `RawFd` and closes it on drop.
        pub fn new(fd: RawFd) -> Self {
            RawFdContainer(fd)
        }

        /// Tries to clone the `RawFdContainer` creating a new FD
        /// with `dup`. The new `RawFdContainer` will take ownership
        /// of the `dup`ed version, whereas the original `RawFdContainer`
        /// will keep the ownership of its FD.
        pub fn try_clone(&self) -> Result<Self, std::io::Error> {
            Ok(Self::new(nix::unistd::dup(self.0)?))
        }

        /// Get the `RawFd` out of this `RawFdContainer`.
        ///
        /// This function would be an implementation of `IntoRawFd` if that were possible. However, it
        /// causes a conflict with an `impl` from libcore...
        pub fn into_raw_fd(self) -> RawFd {
            let fd = self.0;
            forget(self);
            fd
        }

        /// Consumes the `RawFdContainer` and closes the wrapped FD with
        /// the `close` system call.
        ///
        /// This is similar to dropping the `RawFdContainer`, but it allows
        /// the caller to handle errors.
        pub fn close(self) -> Result<(), std::io::Error> {
            let fd = self.into_raw_fd();
            nix::unistd::close(fd).map_err(|e| e.into())
        }
    }

    impl<T: IntoRawFd> From<T> for RawFdContainer {
        fn from(fd: T) -> Self {
            Self::new(fd.into_raw_fd())
        }
    }

    impl AsRawFd for RawFdContainer {
        fn as_raw_fd(&self) -> RawFd {
            self.0
        }
    }
}

#[cfg(not(all(feature = "std", unix)))]
mod raw_fd_container {
    use core::convert::Infallible;

    /// A simple wrapper around RawFd that closes the fd on drop.
    ///
    /// On non-unix systems, this type is empty and does not provide
    /// any method.
    #[derive(Debug, Hash, PartialEq, Eq)]
    pub struct RawFdContainer(Infallible);

    impl Drop for RawFdContainer {
        fn drop(&mut self) {
            // This function exists for symmetry with cfg(unix)
            match self.0 {}
        }
    }
}

pub use raw_fd_container::RawFdContainer;

mod pretty_printer {
    use core::fmt::{Debug, Formatter, Result};

    /// A helper to pretty-print an enumeration value.
    ///
    /// This function prints the given number. If it matches one of the provided variants, that
    /// match is used. Otherwise, the number is printed as a decimal.
    ///
    /// In alternate mode, the second string in the given array is used, else the first.
    pub(crate) fn pretty_print_enum(
        fmt: &mut Formatter<'_>,
        value: u32,
        cases: &[(u32, &str, &str)],
    ) -> Result {
        for (variant, name1, name2) in cases {
            if &value == variant {
                if fmt.alternate() {
                    return fmt.write_str(name2);
                } else {
                    return fmt.write_str(name1);
                }
            }
        }
        Debug::fmt(&value, fmt)
    }

    /// A helper to pretty-print a bitmask.
    ///
    /// This function prints the given number. All bit-matches with the given variants are printed.
    /// Any left-over number is printed as a decimal.
    ///
    /// In alternate mode, the second string in the given array is used, else the first.
    pub(crate) fn pretty_print_bitmask(
        fmt: &mut Formatter<'_>,
        value: u32,
        cases: &[(u32, &str, &str)],
    ) -> Result {
        // First, figure out if there are any bits not covered by any case
        let known_bits = cases.iter().fold(0, |acc, (value, _, _)| acc | value);
        let remaining = value & !known_bits;
        let mut already_printed = if value == 0 || remaining != 0 {
            Debug::fmt(&remaining, fmt)?;
            true
        } else {
            false
        };
        for (variant, name1, name2) in cases {
            if variant & value != 0 {
                if already_printed {
                    fmt.write_str(" | ")?;
                }
                already_printed = true;
                if fmt.alternate() {
                    fmt.write_str(name2)?;
                } else {
                    fmt.write_str(name1)?;
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod test {
        use super::{pretty_print_bitmask, pretty_print_enum};
        use alloc::format;
        use core::fmt::{Display, Formatter, Result};

        type CallbackType = fn(&mut Formatter<'_>, u32, &[(u32, &str, &str)]) -> Result;

        struct CallbackFormating<'a, 'b> {
            callback: CallbackType,
            value: u32,
            cases: &'a [(u32, &'b str, &'b str)],
        }

        fn new_enum<'a, 'b>(
            value: u32,
            cases: &'a [(u32, &'b str, &'b str)],
        ) -> CallbackFormating<'a, 'b> {
            CallbackFormating {
                callback: pretty_print_enum,
                value,
                cases,
            }
        }

        fn new_bitmask<'a, 'b>(
            value: u32,
            cases: &'a [(u32, &'b str, &'b str)],
        ) -> CallbackFormating<'a, 'b> {
            CallbackFormating {
                callback: pretty_print_bitmask,
                value,
                cases,
            }
        }

        impl Display for CallbackFormating<'_, '_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                (self.callback)(f, self.value, self.cases)
            }
        }

        #[test]
        fn test_enum() {
            let cases = [(0, "zero", "ZERO"), (42, "the answer", "ANSWER")];
            let printer = new_enum(0, &cases);
            assert_eq!(&format!("{}", printer), "zero");
            assert_eq!(&format!("{:#}", printer), "ZERO");
            let printer = new_enum(1, &cases);
            assert_eq!(&format!("{}", printer), "1");
            assert_eq!(&format!("{:#}", printer), "1");
            let printer = new_enum(42, &cases);
            assert_eq!(&format!("{}", printer), "the answer");
            assert_eq!(&format!("{:#}", printer), "ANSWER");
        }

        #[test]
        fn test_bitmask() {
            let bits = [
                (1 << 5, "b5", "B5"),
                (1 << 1, "b1", "B1"),
                (1 << 0, "unused", "UNUSED"),
            ];
            let printer = new_bitmask(8, &bits);
            assert_eq!(&format!("{}", printer), "8");
            assert_eq!(&format!("{:#}", printer), "8");
            let printer = new_bitmask(32, &bits);
            assert_eq!(&format!("{}", printer), "b5");
            assert_eq!(&format!("{:#}", printer), "B5");
            let printer = new_bitmask(34, &bits);
            assert_eq!(&format!("{}", printer), "b5 | b1");
            assert_eq!(&format!("{:#}", printer), "B5 | B1");
            let printer = new_bitmask(42, &bits);
            assert_eq!(&format!("{}", printer), "8 | b5 | b1");
            assert_eq!(&format!("{:#}", printer), "8 | B5 | B1");
        }
    }
}

pub(crate) use pretty_printer::{pretty_print_bitmask, pretty_print_enum};
