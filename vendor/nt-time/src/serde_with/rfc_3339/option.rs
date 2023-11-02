// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use the well-known [RFC 3339 format][rfc-3339] when serializing and
//! deserializing an [`Option<FileTime>`].
//!
//! Use this module in combination with Serde's
//! [`#[with]`][serde-with-attribute] attribute.
//!
//! # Examples
//!
//! ```
//! use nt_time::{
//!     serde::{Deserialize, Serialize},
//!     serde_with::rfc_3339,
//!     FileTime,
//! };
//!
//! #[derive(Debug, Deserialize, PartialEq, Serialize)]
//! struct DateTime(#[serde(with = "rfc_3339::option")] Option<FileTime>);
//!
//! let json = serde_json::to_string(&DateTime(Some(FileTime::UNIX_EPOCH))).unwrap();
//! assert_eq!(json, r#""1970-01-01T00:00:00Z""#);
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
//! [rfc-3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

use serde::{de::Error as _, ser::Error as _, Deserializer, Serializer};
use time::{serde::rfc3339, OffsetDateTime};

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes an [`Option<FileTime>`] into the given Serde serializer.
///
/// This serializes using the well-known RFC 3339 format.
pub fn serialize<S: Serializer>(ft: &Option<FileTime>, serializer: S) -> Result<S::Ok, S::Error> {
    rfc3339::option::serialize(
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
/// This deserializes from its RFC 3339 representation.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<FileTime>, D::Error> {
    rfc3339::option::deserialize(deserializer)?
        .map(FileTime::try_from)
        .transpose()
        .map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_de_tokens_error, assert_ser_tokens_error, assert_tokens, Token};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::rfc_3339::option")] Option<FileTime>);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(Some(FileTime::NT_TIME_EPOCH)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::BorrowedStr("1601-01-01T00:00:00Z"),
            ],
        );
        assert_tokens(
            &Test(Some(FileTime::UNIX_EPOCH)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::BorrowedStr("1970-01-01T00:00:00Z"),
            ],
        );
        assert_tokens(
            &Test(None),
            &[Token::NewtypeStruct { name: "Test" }, Token::None],
        );
    }

    #[cfg(not(feature = "large-dates"))]
    #[test]
    fn serialize_error_without_large_dates() {
        assert_ser_tokens_error::<Test>(
            &Test(Some(FileTime::MAX)),
            &[Token::NewtypeStruct { name: "Test" }],
            "date and time is out of range for `OffsetDateTime`",
        );
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn serialize_error_with_large_dates() {
        assert_ser_tokens_error::<Test>(
            &Test(Some(FileTime::MAX)),
            &[Token::NewtypeStruct { name: "Test" }],
            "The year component cannot be formatted into the requested format.",
        );
    }

    #[test]
    fn deserialize_error() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::BorrowedStr("1600-12-31T23:59:59.999999999Z"),
            ],
            "date and time is before `1601-01-01 00:00:00 UTC`",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::NT_TIME_EPOCH))).unwrap(),
            r#""1601-01-01T00:00:00Z""#
        );
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::UNIX_EPOCH))).unwrap(),
            r#""1970-01-01T00:00:00Z""#
        );
        assert_eq!(serde_json::to_string(&Test(None)).unwrap(), "null");
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
}
