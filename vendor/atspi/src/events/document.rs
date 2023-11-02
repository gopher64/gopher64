use zbus::dbus_proxy;

#[dbus_proxy(interface = "org.a11y.atspi.Event.Document", assume_defaults = true)]
trait Document {
	/// AttributesChanged signal
	#[dbus_proxy(signal)]
	fn attributes_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// ContentChanged signal
	#[dbus_proxy(signal)]
	fn content_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// LoadComplete signal
	#[dbus_proxy(signal)]
	fn load_complete(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// LoadStopped signal
	#[dbus_proxy(signal)]
	fn load_stopped(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// PageChanged signal
	#[dbus_proxy(signal)]
	fn page_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Reload signal
	#[dbus_proxy(signal)]
	fn reload(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;
}
