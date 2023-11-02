use zbus::dbus_proxy;

#[dbus_proxy(interface = "org.a11y.atspi.Event.Keyboard", assume_defaults = true)]
trait Keyboard {
	/// Modifiers signal
	#[dbus_proxy(signal)]
	fn modifiers(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;
}
