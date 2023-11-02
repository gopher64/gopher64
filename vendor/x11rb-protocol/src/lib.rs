//! X11 rust bindings.
//!
//! This crate provides a representation of the X11 protocol in Rust. With this protocol, raw X11
//! bytes can be parsed into a structured representation or raw bytes can be produces.
//!
//! This protocol does not do any I/O. If you need an X11 client library, look at
//! <https://docs.rs/x11rb/latest/x11rb/>.

#![forbid(
    missing_copy_implementations,
    missing_debug_implementations,
    private_doc_tests,
    //single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused_import_braces,
    unused_results,
    clippy::cast_lossless,
    clippy::needless_pass_by_value,
)]
// A list of lints that are only #![deny] and not the stronger #![forbid]. Each one has a comment
// explaining why it gets the weaker treatment.
#![deny(
    // #[derive] generates an #[allow] for this
    unused_qualifications,
    // serde's Deserialize/Serialize impls add allows for this
    rust_2018_idioms,
    // Not everything in x11rb_protocol::protocol has doc comments
    missing_docs,
)]
#![no_std]

// std crate imports
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::borrow::Cow;
use alloc::vec::Vec;

pub mod connect;
pub mod connection;
#[macro_use]
pub mod x11_utils;
pub mod errors;
pub mod id_allocator;
pub mod packet_reader;
pub mod parse_display;
#[rustfmt::skip]
#[allow(missing_docs)]
pub mod protocol;
#[cfg(feature = "resource_manager")]
pub mod resource_manager;
mod utils;
pub mod wrapper;
#[cfg(feature = "std")]
pub mod xauth;

pub use utils::RawFdContainer;

// Used to avoid too-complex types.
/// A combination of a buffer and a list of file descriptors.
pub type BufWithFds<B> = (B, Vec<RawFdContainer>);
/// A buffer that is logically continuous, but presented in a number of pieces.
pub type PiecewiseBuf<'a> = Vec<Cow<'a, [u8]>>;

/// Number type used for referring to things that were sent to the server in responses from the
/// server.
///
/// Each request sent to the X11 server is implicitly assigned a monotonically increasing sequence
/// number. Replies, events, and errors contain the sequence number of the last request that the
/// server received. This allows to map replies to their requests and to figure out which request
/// caused an error.
pub type SequenceNumber = u64;

/// The raw bytes of an event and its sequence number.
pub type RawEventAndSeqNumber<B> = (B, SequenceNumber);

/// Variants describing which responses to a request should be discarded.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DiscardMode {
    /// Only discard the actual reply. Errors go to the main loop.
    DiscardReply,
    /// Ignore any kind of response that this request generates.
    DiscardReplyAndError,
}
