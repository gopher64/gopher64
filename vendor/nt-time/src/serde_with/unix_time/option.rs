// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use Unix time when serializing and deserializing an [`Option<FileTime>`].
//!
//! Use this module in combination with Serde's
//! [`#[with]`][serde-with-attribute] attribute.
//!
//! # Examples
//!
//! ```
//! use nt_time::{
//!     serde::{Deserialize, Serialize},
//!     serde_with::unix_time,
//!     FileTime,
//! };
//!
//! #[derive(Debug, Deserialize, PartialEq, Serialize)]
//! struct DateTime(#[serde(with = "unix_time::option")] Option<FileTime>);
//!
//! let json = serde_json::to_string(&DateTime(Some(FileTime::UNIX_EPOCH))).unwrap();
//! assert_eq!(json, "0");
//!
//! assert_eq!(
//!     serde_json::from_str::<DateTime>(&json).unwrap(),
//!     DateTime(Some(FileTime::UNIX_EPOCH))
//! );
//!
//! let json = serde_json::to_string(&DateTime(None)).unwrap();
//! assert_eq!(json, "null");
//!
//! assert_eq!(
//!     serde_json::from_str::<DateTime>(&json).unwrap(),
//!     DateTime(None)
//! );
//! ```
//!
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes an [`Option<FileTime>`] into the given Serde serializer.
///
/// This serializes using Unix time.
pub fn serialize<S: Serializer>(ft: &Option<FileTime>, serializer: S) -> Result<S::Ok, S::Error> {
    ft.map(FileTime::to_unix_time).serialize(serializer)
}

#[allow(clippy::missing_errors_doc)]
/// Deserializes an [`Option<FileTime>`] from the given Serde deserializer.
///
/// This deserializes from its Unix time.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<FileTime>, D::Error> {
    Option::deserialize(deserializer)?
        .map(FileTime::from_unix_time)
        .transpose()
        .map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use serde_test::{
        assert_de_tokens, assert_de_tokens_error, assert_ser_tokens, assert_tokens, Token,
    };

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::unix_time::option")] Option<FileTime>);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(Some(FileTime::NT_TIME_EPOCH)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::I64(-11_644_473_600),
            ],
        );
        assert_tokens(
            &Test(Some(FileTime::UNIX_EPOCH)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::I64(i64::default()),
            ],
        );
        assert_tokens(
            &Test(None),
            &[Token::NewtypeStruct { name: "Test" }, Token::None],
        );
    }

    #[test]
    fn serialize() {
        assert_ser_tokens(
            &Test(Some(FileTime::MAX)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::I64(1_833_029_933_770),
            ],
        );
    }

    #[test]
    fn deserialize() {
        assert_de_tokens(
            &Test(Some(FileTime::MAX - Duration::from_nanos(955_161_500))),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::I64(1_833_029_933_770),
            ],
        );
    }

    #[test]
    fn deserialize_error() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::I64(-11_644_473_601),
            ],
            "date and time is before `1601-01-01 00:00:00 UTC`",
        );
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::I64(1_833_029_933_771),
            ],
            "date and time is after `+60056-05-28 05:36:10.955161500 UTC`",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::NT_TIME_EPOCH))).unwrap(),
            "-11644473600"
        );
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::UNIX_EPOCH))).unwrap(),
            "0"
        );
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::MAX))).unwrap(),
            "1833029933770"
        );
        assert_eq!(serde_json::to_string(&Test(None)).unwrap(), "null");
    }

    #[test]
    fn deserialize_json() {
        assert_eq!(
            serde_json::from_str::<Test>("-11644473600").unwrap(),
            Test(Some(FileTime::NT_TIME_EPOCH))
        );
        assert_eq!(
            serde_json::from_str::<Test>("0").unwrap(),
            Test(Some(FileTime::UNIX_EPOCH))
        );
        assert_eq!(
            serde_json::from_str::<Test>("1833029933770").unwrap(),
            Test(Some(FileTime::MAX - Duration::from_nanos(955_161_500)))
        );
        assert_eq!(serde_json::from_str::<Test>("null").unwrap(), Test(None));
    }
}
