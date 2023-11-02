use zbus::dbus_proxy;

#[dbus_proxy(interface = "org.a11y.atspi.Event.Focus", assume_defaults = true)]
trait Focus {
	/// Focus signal
	#[dbus_proxy(signal)]
	fn focus(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;
}
