use crate::{
	bus::BusProxy,
	events::{Event, HasMatchRule, HasRegistryEventString},
	registry::RegistryProxy,
	AtspiError,
};
use futures_lite::stream::{Stream, StreamExt};
use std::ops::Deref;
use zbus::{fdo::DBusProxy, Address, MatchRule, MessageStream, MessageType};

/// A connection to the at-spi bus
pub struct AccessibilityBus {
	registry: RegistryProxy<'static>,
	dbus_proxy: DBusProxy<'static>,
}

impl AccessibilityBus {
	/// Open a new connection to the bus
	#[tracing::instrument]
	pub async fn open() -> zbus::Result<Self> {
		// Grab the a11y bus address from the session bus
		let a11y_bus_addr = {
			tracing::debug!("Connecting to session bus");
			let session_bus = zbus::Connection::session().await?;
			tracing::debug!(
				name = session_bus.unique_name().map(|n| n.as_str()),
				"Connected to session bus"
			);
			let proxy = BusProxy::new(&session_bus).await?;
			tracing::debug!("Getting a11y bus address from session bus");
			proxy.get_address().await?
		};
		tracing::debug!(address = %a11y_bus_addr, "Got a11y bus address");
		let addr: Address = a11y_bus_addr.parse()?;
		Self::connect(addr).await
	}

	/// Returns an [`AccessibilityBus`], a wrapper for the [`RegistryProxy`]; a handle for the registry provider
	/// on the accessibility bus.
	///
	/// You may want to call this if you have the accessibility bus address and want a connection with
	/// a convenient async event stream provisioning.
	///
	/// Without address, you will want to call  `open`, which tries to obtain the accessibility bus' address
	/// on your behalf.
	///
	/// # Errors
	///
	/// `RegistryProxy` is configured with invalid path, interface or destination

	pub async fn connect(bus_addr: Address) -> zbus::Result<Self> {
		tracing::debug!("Connecting to a11y bus");
		let bus = zbus::ConnectionBuilder::address(bus_addr)?.build().await?;
		tracing::debug!(name = bus.unique_name().map(|n| n.as_str()), "Connected to a11y bus");

		// The Proxy holds a strong reference to a Connection, so we only need to store the proxy
		let registry = RegistryProxy::new(&bus).await?;
		let dbus_proxy = DBusProxy::new(registry.connection()).await?;

		Ok(Self { registry, dbus_proxy })
	}

	/// Stream yielding all `Event` types.
	///
	/// Monitor this stream to be notified and receive events on the a11y bus.
	///
	/// # Example
	/// Basic use:
	///
	/// ```rust
	/// use atspi::events::EventInterfaces;
	/// use enumflags2::BitFlag;
	/// use atspi::identify::object::ObjectEvents;
	/// use atspi::signify::Signified;
	/// use atspi::zbus::{fdo::DBusProxy, MatchRule, MessageType};
	/// use atspi::Event;
	/// # use futures_lite::StreamExt;
	/// # use std::error::Error;
	///
	/// # fn main() {
	/// #   assert!(futures_lite::future::block_on(example()).is_ok());
	/// # }
	///
	/// # async fn example() -> Result<(), Box<dyn Error>> {
	///     let atspi = atspi::AccessibilityBus::open().await?;
	///     atspi.register_event::<ObjectEvents>().await?;
	///
	///     let events = atspi.event_stream();
	///     futures_lite::pin!(events);
	/// #   let output = std::process::Command::new("busctl")
	/// #       .arg("--user")
	/// #       .arg("call")
	/// #       .arg("org.a11y.Bus")
	/// #       .arg("/org/a11y/bus")
	/// #       .arg("org.a11y.Bus")
	/// #       .arg("GetAddress")
	/// #       .output()
	/// #       .unwrap();
	/// #    let addr_string = String::from_utf8(output.stdout).unwrap();
	/// #    let addr_str = addr_string
	/// #        .strip_prefix("s \"")
	/// #        .unwrap()
	/// #        .trim()
	/// #        .strip_suffix('"')
	/// #        .unwrap();
	/// #   let mut base_cmd = std::process::Command::new("busctl");
	/// #   let thing = base_cmd
	/// #       .arg("--address")
	/// #       .arg(addr_str)
	/// #       .arg("emit")
	/// #       .arg("/org/a11y/atspi/accessible/null")
	/// #       .arg("org.a11y.atspi.Event.Object")
	/// #       .arg("StateChanged")
	/// #       .arg("siiva{sv}")
	/// #       .arg("")
	/// #       .arg("0")
	/// #       .arg("0")
	/// #       .arg("i")
	/// #       .arg("0")
	/// #       .arg("0")
	/// #       .output()
	/// #       .unwrap();
	///
	///     while let Some(Ok(ev)) = events.next().await {
	///         // Handle Objject events
	///        break;
	///     }
	/// #    Ok(())
	/// # }
	/// ```
	pub fn event_stream(&self) -> impl Stream<Item = Result<Event, AtspiError>> {
		MessageStream::from(self.registry.connection()).filter_map(|res| {
			let msg = match res {
				Ok(m) => m,
				Err(e) => return Some(Err(e.into())),
			};
			match msg.message_type() {
				MessageType::Signal => Some(Event::try_from(msg)),
				_ => None,
			}
		})
	}

	/// Registers an events as defined in [`crate::identify`]. This function registers a single event, like so:
	/// ```rust
	/// use atspi::identify::object::StateChangedEvent;
	/// # tokio_test::block_on(async {
	/// let connection = atspi::AccessibilityBus::open().await.unwrap();
	/// connection.register_event::<StateChangedEvent>().await.unwrap();
	/// # })
	/// ```
	///
	/// # Errors
	///
	/// This function may return an error if a [`zbus::Error`] is caused by all the various calls to [`zbus::fdo::DBusProxy`] and [`zbus::MatchRule::try_from`].
	pub async fn add_match_rule<T: HasMatchRule>(&self) -> Result<(), AtspiError> {
		let match_rule = MatchRule::try_from(<T as HasMatchRule>::MATCH_RULE_STRING)?;
		self.dbus_proxy.add_match_rule(match_rule).await?;
		Ok(())
	}

	/// Deregisters an events as defined in [`crate::identify`]. This function registers a single event, like so:
	/// ```rust
	/// use atspi::identify::object::StateChangedEvent;
	/// # tokio_test::block_on(async {
	/// let connection = atspi::AccessibilityBus::open().await.unwrap();
	/// connection.add_match_rule::<StateChangedEvent>().await.unwrap();
	/// connection.remove_match_rule::<StateChangedEvent>().await.unwrap();
	/// # })
	/// ```
	///
	/// # Errors
	///
	/// This function may return an error if a [`zbus::Error`] is caused by all the various calls to [`zbus::fdo::DBusProxy`] and [`zbus::MatchRule::try_from`].
	pub async fn remove_match_rule<T: HasMatchRule>(&self) -> Result<(), AtspiError> {
		let match_rule = MatchRule::try_from(<T as HasMatchRule>::MATCH_RULE_STRING)?;
		self.dbus_proxy.add_match_rule(match_rule).await?;
		Ok(())
	}

	/// Add a registry event.
	/// This tells accessible applications which events should be forwarded to the accessbility bus.
	/// This is called by [`Self::register_event`].
	///
	/// ```rust
	/// use atspi::identify::object::StateChangedEvent;
	/// # tokio_test::block_on(async {
	/// let connection = atspi::AccessibilityBus::open().await.unwrap();
	/// connection.add_registry_event::<StateChangedEvent>().await.unwrap();
	/// connection.remove_registry_event::<StateChangedEvent>().await.unwrap();
	/// # })
	/// ```
	///
	/// # Errors
	///
	/// May cause an error if the `DBus` method [`crate::registry::RegistryProxy::register_event`] fails.
	pub async fn add_registry_event<T: HasRegistryEventString>(&self) -> Result<(), AtspiError> {
		self.registry
			.register_event(<T as HasRegistryEventString>::REGISTRY_EVENT_STRING)
			.await?;
		Ok(())
	}

	/// Remove a registry event.
	/// This tells accessible applications which events should be forwarded to the accessbility bus.
	/// This is called by [`Self::deregister_event`].
	/// It may be called like so:
	///
	/// ```rust
	/// use atspi::identify::object::StateChangedEvent;
	/// # tokio_test::block_on(async {
	/// let connection = atspi::AccessibilityBus::open().await.unwrap();
	/// connection.add_registry_event::<StateChangedEvent>().await.unwrap();
	/// connection.remove_registry_event::<StateChangedEvent>().await.unwrap();
	/// # })
	/// ```
	///
	/// # Errors
	///
	/// May cause an error if the `DBus` method [`RegistryProxy::deregister_event`] fails.
	pub async fn remove_registry_event<T: HasRegistryEventString>(&self) -> Result<(), AtspiError> {
		self.registry
			.deregister_event(<T as HasRegistryEventString>::REGISTRY_EVENT_STRING)
			.await?;
		Ok(())
	}

	/// This calls [`Self::add_registry_event`] and [`Self::add_match_rule`], two components necessary to receive accessiblity events.
	/// # Errors
	/// This will only fail if [`Self::add_registry_event`[ or [`Self::add_match_rule`] fails.
	pub async fn register_event<T: HasRegistryEventString + HasMatchRule>(
		&self,
	) -> Result<(), AtspiError> {
		self.add_registry_event::<T>().await?;
		self.add_match_rule::<T>().await?;
		Ok(())
	}

	/// This calls [`Self::remove_registry_event`] and [`Self::remove_match_rule`], two components necessary to receive accessiblity events.
	/// # Errors
	/// This will only fail if [`Self::remove_registry_event`] or [`Self::remove_match_rule`] fails.
	pub async fn deregister_event<T: HasRegistryEventString + HasMatchRule>(
		&self,
	) -> Result<(), AtspiError> {
		self.remove_registry_event::<T>().await?;
		self.remove_match_rule::<T>().await?;
		Ok(())
	}

	/// Shorthand for a reference to the underlying [`zbus::Connection`]
	#[must_use]
	pub fn connection(&self) -> &zbus::Connection {
		self.registry.connection()
	}
}

impl Deref for AccessibilityBus {
	type Target = RegistryProxy<'static>;

	fn deref(&self) -> &Self::Target {
		&self.registry
	}
}

/// Set the `IsEnabled` property in the session bus.
///
/// Assistive Technology provider applications (ATs) should set the accessibility
/// `IsEnabled` status on the users session bus on startup as applications may monitor this property
/// to  enable their accessibility support dynamically.
///
/// See: The [freedesktop - AT-SPI2 wiki](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
///
///  ## Example
/// ```rust
///     use futures_lite::future::block_on;
///
///     let result =  block_on( atspi::set_session_accessibility(true) );
///     assert!(result.is_ok());
/// ```
/// # Errors
///
/// 1. when no connection with the session bus can be established,
/// 2. if creation of a [`crate::bus::StatusProxy`] fails
/// 3. if the `IsEnabled` property cannot be read
/// 4. the `IsEnabled` property cannot be set.
pub async fn set_session_accessibility(status: bool) -> std::result::Result<(), AtspiError> {
	// Get a connection to the session bus.
	let session = zbus::Connection::session().await?;

	// Aqcuire a `StatusProxy` for the session bus.
	let status_proxy = crate::bus::StatusProxy::new(&session).await?;

	if status_proxy.is_enabled().await? != status {
		status_proxy.set_is_enabled(status).await?;
	}
	Ok(())
}
