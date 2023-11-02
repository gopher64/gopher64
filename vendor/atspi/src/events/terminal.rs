use zbus::dbus_proxy;

#[dbus_proxy(interface = "org.a11y.atspi.Event.Terminal", assume_defaults = true)]
trait Terminal {
	/// ApplicationChanged signal
	#[dbus_proxy(signal)]
	fn application_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// CharwidthChanged signal
	#[dbus_proxy(signal)]
	fn charwidth_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// ColumncountChanged signal
	#[dbus_proxy(signal)]
	fn columncount_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// LineChanged signal
	#[dbus_proxy(signal)]
	fn line_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// LinecountChanged signal
	#[dbus_proxy(signal)]
	fn linecount_changed(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;
}
