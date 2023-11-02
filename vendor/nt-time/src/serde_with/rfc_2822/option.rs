// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use the well-known [RFC 2822 format][rfc-2822] when serializing and
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
//!     serde_with::rfc_2822,
//!     FileTime,
//! };
//!
//! #[derive(Debug, Deserialize, PartialEq, Serialize)]
//! struct DateTime(#[serde(with = "rfc_2822::option")] Option<FileTime>);
//!
//! let json = serde_json::to_string(&DateTime(Some(FileTime::UNIX_EPOCH))).unwrap();
//! assert_eq!(json, r#""Thu, 01 Jan 1970 00:00:00 +0000""#);
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
//! [rfc-2822]: https://datatracker.ietf.org/doc/html/rfc2822#section-3.3
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

use serde::{de::Error as _, ser::Error as _, Deserializer, Serializer};
use time::{serde::rfc2822, OffsetDateTime};

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes an [`Option<FileTime>`] into the given Serde serializer.
///
/// This serializes using the well-known RFC 2822 format.
pub fn serialize<S: Serializer>(ft: &Option<FileTime>, serializer: S) -> Result<S::Ok, S::Error> {
    rfc2822::option::serialize(
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
/// This deserializes from its RFC 2822 representation.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<FileTime>, D::Error> {
    rfc2822::option::deserialize(deserializer)?
        .map(FileTime::try_from)
        .transpose()
        .map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_ser_tokens_error, assert_tokens, Token};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::rfc_2822::option")] Option<FileTime>);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(Some(FileTime::UNIX_EPOCH)),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::Some,
                Token::BorrowedStr("Thu, 01 Jan 1970 00:00:00 +0000"),
            ],
        );
        assert_tokens(
            &Test(None),
            &[Token::NewtypeStruct { name: "Test" }, Token::None],
        );
    }

    #[test]
    fn serialize_error() {
        assert_ser_tokens_error::<Test>(
            &Test(Some(FileTime::NT_TIME_EPOCH)),
            &[Token::NewtypeStruct { name: "Test" }],
            "The year component cannot be formatted into the requested format.",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(Some(FileTime::UNIX_EPOCH))).unwrap(),
            r#""Thu, 01 Jan 1970 00:00:00 +0000""#
        );
        assert_eq!(serde_json::to_string(&Test(None)).unwrap(), "null");
    }

    #[test]
    fn deserialize_json() {
        assert_eq!(
            serde_json::from_str::<Test>(r#""Thu, 01 Jan 1970 00:00:00 +0000""#).unwrap(),
            Test(Some(FileTime::UNIX_EPOCH))
        );
        assert_eq!(serde_json::from_str::<Test>("null").unwrap(), Test(None));
    }
}
