//! ## Signified signal types
//!
//! The generic `AtspiEvent` has a specific meaning depending on its origin.
//! This module offers the signified types and their conversions from a generic `AtpiEvent`.
//!
//! The `TrySignify` macro implements a `TryFrom<Event>` on a per-name and member basis
//!

use crate::events::{AtspiEvent, GenericEvent};
use std::collections::HashMap;
use std::sync::Arc;
use zbus::{names::MemberName, zvariant, Message};
use zbus_names::{self, InterfaceName};
use zvariant::OwnedValue;

/// All Atspi / Qspi event types encapsulate `AtspiEvent`.
/// This trait allows access to the underlying item.
pub trait Signified {
	type Inner;

	fn inner(&self) -> &AtspiEvent;
	fn properties(&self) -> &HashMap<String, OwnedValue>;
	fn kind(&self) -> &str;
}

/// Shared functionality of Events, through its `Message` header
impl<T> GenericEvent for T
where
	T: Signified,
{
	/// Serialized bus message.
	#[must_use]
	fn message(&self) -> &Arc<Message> {
		&self.inner().message
	}

	// For now this returns the full interface name because the lifetimes in [`zbus_names`][zbus::names] are
	// wrong such that the `&str` you can get from a
	// [`zbus_names::InterfaceName`][zbus::names::InterfaceName] is tied to the lifetime of that
	// name, not to the lifetime of the message as it should be. In future, this will return only
	// the last component of the interface name (I.E. "Object" from
	// "org.a11y.atspi.Event.Object").

	/// The emitting interface.
	#[must_use]
	fn interface(&self) -> Option<InterfaceName<'_>> {
		self.inner().message.interface()
	}

	/// The interface member that dispatched this event / signal.
	///
	/// Members of the interface are either signals, methods or properties.
	/// eg. `PropertyChanged` or `TextChanged`
	#[must_use]
	fn member(&self) -> Option<MemberName<'_>> {
		self.inner().message.member()
	}

	/// The object path to the object where the signal is emitted from.
	#[must_use]
	fn path(&self) -> std::option::Option<zbus::zvariant::ObjectPath<'_>> {
		self.inner().message.path()
	}

	/// Identifies the `sender` of the `Event`.
	/// # Errors
	/// - when deserializeing the header failed, or
	/// * When `zbus::get_field!` finds that 'sender' is an invalid field.
	fn sender(&self) -> Result<Option<zbus::names::UniqueName>, crate::AtspiError> {
		Ok(self.inner().message.header()?.sender()?.cloned())
	}
}
