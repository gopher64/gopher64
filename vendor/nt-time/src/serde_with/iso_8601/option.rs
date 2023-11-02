// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use the well-known [ISO 8601 format][iso-8601-description-url] when
//! serializing and deserializing an [`Option<FileTime>`].
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
//! struct DateTime(#[serde(with = "iso_8601::option")] Option<FileTime>);
//!
//! let json = serde_json::to_string(&DateTime(Some(FileTime::UNIX_EPOCH))).unwrap();
//! assert_eq!(json, r#""+001970-01-01T00:00:00.000000000Z""#);
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
//! [iso-8601-description-url]: https://www.iso.org/iso-8601-date-and-time-format.html
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

use serde::{de::Error as _, ser::Error as _, Deserializer, Serializer};
use time::{serde::iso8601, OffsetDateTime};

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes an [`Option<FileTime>`] into the given Serde serializer.
///
/// This serializes using the well-known ISO 8601 format.
pub fn serialize<S: Serializer>(ft: &Option<FileTime>, serializer: S) -> Result<S::Ok, S::Error> {
    iso8601::option::serialize(
        &(*ft)
            .map(OffsetDateTime::try_from)
            .transpose()
            .map_err(S::Error::custom)?,
        serializer,
    )
}

#[allow(clippy::missing_errors_doc)]
/// Deserializes an [`Option<FileTime>`] from the given Serde deserializer.
///
/// This deserializes from its ISO 8601 representation.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<FileTime>, D::Error> {
    iso8601::option::deserialize(deserializer)?
        .map(FileTime::try_from)
        .transpose()
        .map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::iso_8601::option")] Option<FileTime>);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(Some(FileTime::NT_TIME_EPOCH)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::BorrowedStr("+001601-01-01T00:00:00.000000000Z"),
            ],
        );
        assert_tokens(
            &Test(Some(FileTime::UNIX_EPOCH)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::BorrowedStr("+001970-01-01T00:00:00.000000000Z"),
            ],
        );
        assert_tokens(
            &Test(None),
            &[Token::NewtypeStruct { name: "Test" }, Token::None],
        );
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn serde_with_large_dates() {
        assert_tokens(
            &Test(Some(FileTime::MAX)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::BorrowedStr("+060056-05-28T05:36:10.955161500Z"),
            ],
        );
    }

    #[test]
    fn deserialize_error() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
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
                Token::Some,
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
                Token::Some,
                Token::BorrowedStr("+060056-05-28T05:36:10.955161600Z"),
            ],
            "date and time is after `+60056-05-28 05:36:10.955161500 UTC`",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::NT_TIME_EPOCH))).unwrap(),
            r#""+001601-01-01T00:00:00.000000000Z""#
        );
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::UNIX_EPOCH))).unwrap(),
            r#""+001970-01-01T00:00:00.000000000Z""#
        );
        assert_eq!(serde_json::to_string(&Test(None)).unwrap(), "null");
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn serialize_json_with_large_dates() {
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::MAX))).unwrap(),
            r#""+060056-05-28T05:36:10.955161500Z""#
        );
    }

    #[test]
    fn deserialize_json() {
        assert_eq!(
            serde_json::from_str::<Test>(r#""1601-01-01T00:00:00Z""#).unwrap(),
            Test(Some(FileTime::NT_TIME_EPOCH))
        );
        assert_eq!(
            serde_json::from_str::<Test>(r#""1970-01-01T00:00:00Z""#).unwrap(),
            Test(Some(FileTime::UNIX_EPOCH))
        );
        assert_eq!(serde_json::from_str::<Test>("null").unwrap(), Test(None));
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn deserialize_json_with_large_dates() {
        assert_eq!(
            serde_json::from_str::<Test>(r#""+060056-05-28T05:36:10.955161500Z""#).unwrap(),
            Test(Some(FileTime::MAX))
        );
    }
}
