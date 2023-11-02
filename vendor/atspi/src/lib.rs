#![deny(clippy::all, clippy::pedantic, clippy::cargo, unsafe_code)]
// #![deny(clippy::missing_docs)]

pub mod accessible;
pub mod action;
pub mod application;
pub mod bus;
pub mod cache;
pub mod collection;
pub mod component;
pub mod convertable;
pub mod device_event_controller;
pub mod device_event_listener;
pub mod document;
pub mod editable_text;
pub mod events;
pub mod identify;
pub mod signify;
pub use events::{Event, EventBody};
pub mod hyperlink;
pub mod hypertext;
pub mod image;
pub mod processed;
pub mod registry;
pub mod selection;
pub mod socket;
pub mod table;
pub mod table_cell;
pub mod text;
pub mod value;

pub mod accessible_ext;
pub mod text_ext;

// Hand-written connection module
mod accessibility_bus;
pub use accessibility_bus::*;

mod interfaces;
pub use interfaces::*;

mod state;
pub use state::*;

pub mod error;
pub use error::AtspiError;

pub use zbus;
use zbus::zvariant::Type;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[repr(u32)]
/// The coordinate type encodes the frame of reference.
pub enum CoordType {
	/// In relation to the entire screen.
	Screen,
	/// In relation to only the window.
	Window,
	/// In relation to the parent of the element being checked.
	Parent,
}

pub trait AtspiProxy {
	const INTERFACE: Interface;
}
