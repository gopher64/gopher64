//! # Custom Errors
//!
//! The most basic error type is [`ParserError`][crate::error::ParserError]
//!
//! Optional traits include:
//! - [`AddContext`][crate::error::AddContext]
//! - [`FromExternalError`][crate::error::FromExternalError]
//!
//! # Example
//!
//!```rust
#![doc = include_str!("../../examples/custom_error.rs")]
//!```
