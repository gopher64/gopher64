use core::fmt;
use core::hash::{Hash, Hasher};
use malloc_buf::Malloc;
use std::error::Error;

use crate::encode::{Encode, EncodeArguments, EncodeConvert, Encoding};
use crate::runtime::{Class, Object, Sel};

/// Workaround for `Malloc<str>` not implementing common traits
#[derive(Debug)]
struct MallocEncoding(Malloc<str>);

// SAFETY: `char*` strings can safely be free'd on other threads.
unsafe impl Send for MallocEncoding {}
unsafe impl Sync for MallocEncoding {}

impl PartialEq for MallocEncoding {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for MallocEncoding {}

impl Hash for MallocEncoding {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&*self.0, state)
    }
}

impl fmt::Display for MallocEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Inner {
    MethodNotFound,
    MismatchedReturn(MallocEncoding, Encoding),
    MismatchedArgumentsCount(usize, usize),
    MismatchedArgument(usize, MallocEncoding, Encoding),
}

impl fmt::Display for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MethodNotFound => {
                write!(f, "method not found")
            }
            Self::MismatchedReturn(expected, actual) => {
                write!(
                    f,
                    "expected return to have type code {}, but found {}",
                    expected, actual
                )
            }
            Self::MismatchedArgumentsCount(expected, actual) => {
                write!(
                    f,
                    "expected {} arguments, but {} were given",
                    expected, actual
                )
            }
            Self::MismatchedArgument(i, expected, actual) => {
                write!(
                    f,
                    "expected argument at index {} to have type code {}, but found {}",
                    i, expected, actual
                )
            }
        }
    }
}

/// Failed verifying selector on a class.
///
/// This is returned in the error case of [`Class::verify_sel`], see that for
/// details.
///
/// This implements [`Error`], and a description of the error can be retrieved
/// using [`fmt::Display`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct VerificationError(Inner);

impl From<Inner> for VerificationError {
    fn from(inner: Inner) -> Self {
        Self(inner)
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to inner
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for VerificationError {}

pub(crate) fn verify_message_signature<A, R>(cls: &Class, sel: Sel) -> Result<(), VerificationError>
where
    A: EncodeArguments,
    R: EncodeConvert,
{
    let method = match cls.instance_method(sel) {
        Some(method) => method,
        None => return Err(Inner::MethodNotFound.into()),
    };

    let actual = R::__Inner::ENCODING;
    let expected = method.return_type();
    if !actual.equivalent_to_str(&*expected) {
        return Err(Inner::MismatchedReturn(MallocEncoding(expected), actual).into());
    }

    let self_and_cmd = [<*mut Object>::ENCODING, Sel::ENCODING];
    let args = A::ENCODINGS;

    let actual = self_and_cmd.len() + args.len();
    let expected = method.arguments_count();
    if actual != expected {
        return Err(Inner::MismatchedArgumentsCount(expected, actual).into());
    }

    for (i, actual) in self_and_cmd.iter().chain(args).copied().enumerate() {
        let expected = method.argument_type(i).unwrap();
        if !actual.equivalent_to_str(&*expected) {
            return Err(Inner::MismatchedArgument(i, MallocEncoding(expected), actual).into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sel;
    use crate::test_utils;
    use alloc::string::ToString;
    use core::panic::{RefUnwindSafe, UnwindSafe};

    #[test]
    fn test_verify_message() {
        let cls = test_utils::custom_class();

        assert!(cls.verify_sel::<(), u32>(sel!(foo)).is_ok());
        assert!(cls.verify_sel::<(u32,), ()>(sel!(setFoo:)).is_ok());

        let metaclass = cls.metaclass();
        metaclass
            .verify_sel::<(i32, i32), i32>(sel!(addNumber:toNumber:))
            .unwrap();
    }

    #[test]
    fn test_verify_message_errors() {
        let cls = test_utils::custom_class();

        // Unimplemented selector (missing colon)
        let err = cls.verify_sel::<(), ()>(sel!(setFoo)).unwrap_err();
        assert_eq!(err.to_string(), "method not found");

        // Incorrect return type
        let err = cls.verify_sel::<(u32,), u64>(sel!(setFoo:)).unwrap_err();
        assert_eq!(
            err.to_string(),
            "expected return to have type code v, but found Q"
        );

        // Too many arguments
        let err = cls.verify_sel::<(u32, i8), ()>(sel!(setFoo:)).unwrap_err();
        assert_eq!(err.to_string(), "expected 3 arguments, but 4 were given");

        // Too few arguments
        let err = cls.verify_sel::<(), ()>(sel!(setFoo:)).unwrap_err();
        assert_eq!(err.to_string(), "expected 3 arguments, but 2 were given");

        // Incorrect argument type
        let err = cls.verify_sel::<(Sel,), ()>(sel!(setFoo:)).unwrap_err();
        assert_eq!(
            err.to_string(),
            "expected argument at index 2 to have type code I, but found :"
        );

        // Metaclass
        let metaclass = cls.metaclass();
        let err = metaclass
            .verify_sel::<(i32, i32, i32), i32>(sel!(addNumber:toNumber:))
            .unwrap_err();
        assert_eq!(err.to_string(), "expected 4 arguments, but 5 were given");
    }

    #[test]
    #[cfg(feature = "verify_message")]
    #[should_panic = "invalid message send to -[CustomObject foo]: expected return to have type code I, but found i"]
    fn test_send_message_verified() {
        let obj = test_utils::custom_object();
        let _: i32 = unsafe { crate::msg_send![&obj, foo] };
    }

    #[test]
    #[cfg(feature = "verify_message")]
    #[should_panic = "invalid message send to +[CustomObject abcDef]: method not found"]
    fn test_send_message_verified_to_class() {
        let cls = test_utils::custom_class();
        let _: i32 = unsafe { crate::msg_send![cls, abcDef] };
    }

    #[test]
    fn test_marker_traits() {
        fn assert_marker_traits<T: Send + Sync + UnwindSafe + RefUnwindSafe + Unpin>() {}
        assert_marker_traits::<VerificationError>();
    }
}
