// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use the well-known [ISO 8601 format][iso-8601-description-url] when
//! serializing and deserializing a [`FileTime`].
//!
//! Use this module in combination with Serde's
//! [`#[with]`][serde-with-attribute] attribute.
//!
//! If the `large-dates` feature is not enabled, the maximum date and time is
//! "9999-12-31 23:59:59.999999999 UTC".
//!
//! # Examples
//!
//! ```
//! use nt_time::{
//!     serde::{Deserialize, Serialize},
//!     serde_with::iso_8601,
//!     FileTime,
//! };
//!
//! #[derive(Debug, Deserialize, PartialEq, Serialize)]
//! struct DateTime(#[serde(with = "iso_8601")] FileTime);
//!
//! let json = serde_json::to_string(&DateTime(FileTime::UNIX_EPOCH)).unwrap();
//! assert_eq!(json, r#""+001970-01-01T00:00:00.000000000Z""#);
//!
//! assert_eq!(
//!     serde_json::from_str::<DateTime>(&json).unwrap(),
//!     DateTime(FileTime::UNIX_EPOCH)
//! );
//! ```
//!
//! [iso-8601-description-url]: https://www.iso.org/iso-8601-date-and-time-format.html
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

pub mod option;

use serde::{de::Error as _, ser::Error as _, Deserializer, Serializer};
use time::serde::iso8601;

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes a [`FileTime`] into the given Serde serializer.
///
/// This serializes using the well-known ISO 8601 format.
pub fn serialize<S: Serializer>(ft: &FileTime, serializer: S) -> Result<S::Ok, S::Error> {
    iso8601::serialize(&(*ft).try_into().map_err(S::Error::custom)?, serializer)
}

#[allow(clippy::missing_errors_doc)]
/// Deserializes a [`FileTime`] from the given Serde deserializer.
///
/// This deserializes from its ISO 8601 representation.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<FileTime, D::Error> {
    FileTime::try_from(iso8601::deserialize(deserializer)?).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::iso_8601")] FileTime);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(FileTime::NT_TIME_EPOCH),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("+001601-01-01T00:00:00.000000000Z"),
            ],
        );
        assert_tokens(
            &Test(FileTime::UNIX_EPOCH),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("+001970-01-01T00:00:00.000000000Z"),
            ],
        );
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn serde_with_large_dates() {
        assert_tokens(
            &Test(FileTime::MAX),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("+060056-05-28T05:36:10.955161500Z"),
            ],
        );
    }

    #[test]
    fn deserialize_error() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("+001600-12-31T23:59:59.999999999Z"),
            ],
            "date and time is before `1601-01-01 00:00:00 UTC`",
        );
    }

    #[cfg(not(feature = "large-dates"))]
    #[test]
    fn deserialize_error_without_large_dates() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("+010000-01-01T00:00:00.000000000Z"),
            ],
            "year must be in the range -9999..=9999",
        );
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn deserialize_error_with_large_dates() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("+060056-05-28T05:36:10.955161600Z"),
            ],
            "date and time is after `+60056-05-28 05:36:10.955161500 UTC`",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(FileTime::NT_TIME_EPOCH)).unwrap(),
            r#""+001601-01-01T00:00:00.000000000Z""#
        );
        assert_eq!(
            serde_json::to_string(&Test(FileTime::UNIX_EPOCH)).unwrap(),
            r#""+001970-01-01T00:00:00.000000000Z""#
        );
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn serialize_json_with_large_dates() {
        assert_eq!(
            serde_json::to_string(&Test(FileTime::MAX)).unwrap(),
            r#""+060056-05-28T05:36:10.955161500Z""#
        );
    }

    #[test]
    fn deserialize_json() {
        assert_eq!(
            serde_json::from_str::<Test>(r#""1601-01-01T00:00:00Z""#).unwrap(),
            Test(FileTime::NT_TIME_EPOCH)
        );
        assert_eq!(
            serde_json::from_str::<Test>(r#""1970-01-01T00:00:00Z""#).unwrap(),
            Test(FileTime::UNIX_EPOCH)
        );
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn deserialize_json_with_large_dates() {
        assert_eq!(
            serde_json::from_str::<Test>(r#""+060056-05-28T05:36:10.955161500Z""#).unwrap(),
            Test(FileTime::MAX)
        );
    }
}
