// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use Unix time when serializing and deserializing a [`FileTime`].
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
//! struct DateTime(#[serde(with = "unix_time")] FileTime);
//!
//! let json = serde_json::to_string(&DateTime(FileTime::UNIX_EPOCH)).unwrap();
//! assert_eq!(json, "0");
//!
//! assert_eq!(
//!     serde_json::from_str::<DateTime>(&json).unwrap(),
//!     DateTime(FileTime::UNIX_EPOCH)
//! );
//! ```
//!
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

pub mod option;

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes a [`FileTime`] into the given Serde serializer.
///
/// This serializes using Unix time.
pub fn serialize<S: Serializer>(ft: &FileTime, serializer: S) -> Result<S::Ok, S::Error> {
    ft.to_unix_time().serialize(serializer)
}

#[allow(clippy::missing_errors_doc)]
/// Deserializes a [`FileTime`] from the given Serde deserializer.
///
/// This deserializes from its Unix time.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<FileTime, D::Error> {
    FileTime::from_unix_time(i64::deserialize(deserializer)?).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use serde_test::{
        assert_de_tokens, assert_de_tokens_error, assert_ser_tokens, assert_tokens, Token,
    };

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::unix_time")] FileTime);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(FileTime::NT_TIME_EPOCH),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::I64(-11_644_473_600),
            ],
        );
        assert_tokens(
            &Test(FileTime::UNIX_EPOCH),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::I64(i64::default()),
            ],
        );
    }

    #[test]
    fn serialize() {
        assert_ser_tokens(
            &Test(FileTime::MAX),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::I64(1_833_029_933_770),
            ],
        );
    }

    #[test]
    fn deserialize() {
        assert_de_tokens(
            &Test(FileTime::MAX - Duration::from_nanos(955_161_500)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::I64(1_833_029_933_770),
            ],
        );
    }

    #[test]
    fn deserialize_error() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::I64(-11_644_473_601),
            ],
            "date and time is before `1601-01-01 00:00:00 UTC`",
        );
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::I64(1_833_029_933_771),
            ],
            "date and time is after `+60056-05-28 05:36:10.955161500 UTC`",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(FileTime::NT_TIME_EPOCH)).unwrap(),
            "-11644473600"
        );
        assert_eq!(
            serde_json::to_string(&Test(FileTime::UNIX_EPOCH)).unwrap(),
            "0"
        );
        assert_eq!(
            serde_json::to_string(&Test(FileTime::MAX)).unwrap(),
            "1833029933770"
        );
    }

    #[test]
    fn deserialize_json() {
        assert_eq!(
            serde_json::from_str::<Test>("-11644473600").unwrap(),
            Test(FileTime::NT_TIME_EPOCH)
        );
        assert_eq!(
            serde_json::from_str::<Test>("0").unwrap(),
            Test(FileTime::UNIX_EPOCH)
        );
        assert_eq!(
            serde_json::from_str::<Test>("1833029933770").unwrap(),
            Test(FileTime::MAX - Duration::from_nanos(955_161_500))
        );
    }
}
