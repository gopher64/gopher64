use zbus::dbus_proxy;

#[dbus_proxy(interface = "org.a11y.atspi.Event.Window", assume_defaults = true)]
trait Window {
	/// Activate signal
	#[dbus_proxy(signal)]
	fn activate(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Close signal
	#[dbus_proxy(signal)]
	fn close(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Create signal
	#[dbus_proxy(signal)]
	fn create(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Deactivate signal
	#[dbus_proxy(signal)]
	fn deactivate(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// DesktopCreate signal
	#[dbus_proxy(signal)]
	fn desktop_create(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// DesktopDestroy signal
	#[dbus_proxy(signal)]
	fn desktop_destroy(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Destroy signal
	#[dbus_proxy(signal)]
	fn destroy(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Lower signal
	#[dbus_proxy(signal)]
	fn lower(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Maximize signal
	#[dbus_proxy(signal)]
	fn maximize(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Minimize signal
	#[dbus_proxy(signal)]
	fn minimize(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Move signal
	#[dbus_proxy(signal)]
	fn move_(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// PropertyChange signal
	#[dbus_proxy(signal)]
	fn property_change(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Raise signal
	#[dbus_proxy(signal)]
	fn raise(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Reparent signal
	#[dbus_proxy(signal)]
	fn reparent(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Resize signal
	#[dbus_proxy(signal)]
	fn resize(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Restore signal
	#[dbus_proxy(signal)]
	fn restore(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Restyle signal
	#[dbus_proxy(signal)]
	fn restyle(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// Shade signal
	#[dbus_proxy(signal)]
	fn shade(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;

	/// uUshade signal
	#[dbus_proxy(signal)]
	fn u_ushade(&self, event: super::EventBody<'_, &str>) -> zbus::Result<()>;
}
