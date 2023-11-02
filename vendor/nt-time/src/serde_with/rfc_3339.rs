// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use the well-known [RFC 3339 format][rfc-3339] when serializing and
//! deserializing a [`FileTime`].
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
//! struct DateTime(#[serde(with = "rfc_3339")] FileTime);
//!
//! let json = serde_json::to_string(&DateTime(FileTime::UNIX_EPOCH)).unwrap();
//! assert_eq!(json, r#""1970-01-01T00:00:00Z""#);
//!
//! assert_eq!(
//!     serde_json::from_str::<DateTime>(&json).unwrap(),
//!     DateTime(FileTime::UNIX_EPOCH)
//! );
//! ```
//!
//! [rfc-3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

pub mod option;

use serde::{de::Error as _, ser::Error as _, Deserializer, Serializer};
use time::serde::rfc3339;

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes a [`FileTime`] into the given Serde serializer.
///
/// This serializes using the well-known RFC 3339 format.
pub fn serialize<S: Serializer>(ft: &FileTime, serializer: S) -> Result<S::Ok, S::Error> {
    rfc3339::serialize(&(*ft).try_into().map_err(S::Error::custom)?, serializer)
}

#[allow(clippy::missing_errors_doc)]
/// Deserializes a [`FileTime`] from the given Serde deserializer.
///
/// This deserializes from its RFC 3339 representation.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<FileTime, D::Error> {
    FileTime::try_from(rfc3339::deserialize(deserializer)?).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_de_tokens_error, assert_ser_tokens_error, assert_tokens, Token};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::rfc_3339")] FileTime);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(FileTime::NT_TIME_EPOCH),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("1601-01-01T00:00:00Z"),
            ],
        );
        assert_tokens(
            &Test(FileTime::UNIX_EPOCH),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("1970-01-01T00:00:00Z"),
            ],
        );
    }

    #[cfg(not(feature = "large-dates"))]
    #[test]
    fn serialize_error_without_large_dates() {
        assert_ser_tokens_error::<Test>(
            &Test(FileTime::MAX),
            &[Token::NewtypeStruct { name: "Test" }],
            "date and time is out of range for `OffsetDateTime`",
        );
    }

    #[cfg(feature = "large-dates")]
    #[test]
    fn serialize_error_with_large_dates() {
        assert_ser_tokens_error::<Test>(
            &Test(FileTime::MAX),
            &[Token::NewtypeStruct { name: "Test" }],
            "The year component cannot be formatted into the requested format.",
        );
    }

    #[test]
    fn deserialize_error() {
        assert_de_tokens_error::<Test>(
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("1600-12-31T23:59:59.999999999Z"),
            ],
            "date and time is before `1601-01-01 00:00:00 UTC`",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(FileTime::NT_TIME_EPOCH)).unwrap(),
            r#""1601-01-01T00:00:00Z""#
        );
        assert_eq!(
            serde_json::to_string(&Test(FileTime::UNIX_EPOCH)).unwrap(),
            r#""1970-01-01T00:00:00Z""#
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
}
