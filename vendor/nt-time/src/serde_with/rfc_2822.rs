// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Use the well-known [RFC 2822 format][rfc-2822] when serializing and
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
//!     serde_with::rfc_2822,
//!     FileTime,
//! };
//!
//! #[derive(Debug, Deserialize, PartialEq, Serialize)]
//! struct DateTime(#[serde(with = "rfc_2822")] FileTime);
//!
//! let json = serde_json::to_string(&DateTime(FileTime::UNIX_EPOCH)).unwrap();
//! assert_eq!(json, r#""Thu, 01 Jan 1970 00:00:00 +0000""#);
//!
//! assert_eq!(
//!     serde_json::from_str::<DateTime>(&json).unwrap(),
//!     DateTime(FileTime::UNIX_EPOCH)
//! );
//! ```
//!
//! [rfc-2822]: https://datatracker.ietf.org/doc/html/rfc2822#section-3.3
//! [serde-with-attribute]: https://serde.rs/field-attrs.html#with

pub mod option;

use serde::{de::Error as _, ser::Error as _, Deserializer, Serializer};
use time::serde::rfc2822;

use crate::FileTime;

#[allow(clippy::missing_errors_doc)]
/// Serializes a [`FileTime`] into the given Serde serializer.
///
/// This serializes using the well-known RFC 2822 format.
pub fn serialize<S: Serializer>(ft: &FileTime, serializer: S) -> Result<S::Ok, S::Error> {
    rfc2822::serialize(&(*ft).try_into().map_err(S::Error::custom)?, serializer)
}

#[allow(clippy::missing_errors_doc)]
/// Deserializes a [`FileTime`] from the given Serde deserializer.
///
/// This deserializes from its RFC 2822 representation.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<FileTime, D::Error> {
    FileTime::try_from(rfc2822::deserialize(deserializer)?).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_ser_tokens_error, assert_tokens, Token};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test(#[serde(with = "crate::serde_with::rfc_2822")] FileTime);

    #[test]
    fn serde() {
        assert_tokens(
            &Test(FileTime::UNIX_EPOCH),
            &[
                Token::NewtypeStruct { name: "Test" },
                Token::BorrowedStr("Thu, 01 Jan 1970 00:00:00 +0000"),
            ],
        );
    }

    #[test]
    fn serialize_error() {
        assert_ser_tokens_error::<Test>(
            &Test(FileTime::NT_TIME_EPOCH),
            &[Token::NewtypeStruct { name: "Test" }],
            "The year component cannot be formatted into the requested format.",
        );
    }

    #[test]
    fn serialize_json() {
        assert_eq!(
            serde_json::to_string(&Test(FileTime::UNIX_EPOCH)).unwrap(),
            r#""Thu, 01 Jan 1970 00:00:00 +0000""#
        );
    }

    #[test]
    fn deserialize_json() {
        assert_eq!(
            serde_json::from_str::<Test>(r#""Thu, 01 Jan 1970 00:00:00 +0000""#).unwrap(),
            Test(FileTime::UNIX_EPOCH)
        );
    }
}
