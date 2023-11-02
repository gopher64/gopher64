use std::{fmt, str::FromStr};

#[cfg(all(feature = "raw_handle", feature = "gtk4"))]
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use serde::{ser::Serializer, Deserialize, Serialize};
use zbus::zvariant::Type;
/// Most portals interact with the user by showing dialogs.
/// These dialogs should generally be placed on top of the application window
/// that triggered them. To arrange this, the compositor needs to know about the
/// application window. Many portal requests expect a [`WindowIdentifier`] for
/// this reason.
///
/// Under X11, the [`WindowIdentifier`] should have the form `x11:XID`, where
/// XID is the XID of the application window in hexadecimal. Under Wayland, it
/// should have the form `wayland:HANDLE`, where HANDLE is a surface handle
/// obtained with the [xdg-foreign](https://github.com/wayland-project/wayland-protocols/blob/master/unstable/xdg-foreign/xdg-foreign-unstable-v2.xml) protocol.
///
/// See also [Parent window identifiers](https://flatpak.github.io/xdg-desktop-portal/index.html#parent_window).
///
/// # Usage
///
/// ## From an X11 XID
///
/// ```rust,ignore
/// let identifier = WindowIdentifier::from_xid(212321);
///
/// /// Open some portals
/// ```
///
/// ## From a Wayland Surface
///
/// The `wayland` feature must be enabled. The exported surface handle will be
/// unexported on `Drop`.
///
/// ```text
/// // let wl_surface = some_surface;
/// // let identifier = WindowIdentifier::from_wayland(wl_surface).await;
///
/// /// Open some portals
/// ```
///
/// Or using a raw `wl_surface` pointer
///
/// ```text
/// // let wl_surface_ptr = some_surface;
/// // let wl_display_ptr = corresponding_display;
/// // let identifier = WindowIdentifier::from_wayland_raw(wl_surface_ptr, wl_display_ptr).await;
///
/// /// Open some portals
/// ```
///
/// ## With GTK 4
///
/// The feature `gtk4` must be enabled. You can get a
/// [`WindowIdentifier`] from a [`IsA<gtk4::Native>`](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.Native.html) using `WindowIdentifier::from_native`
///
/// ```rust, ignore
/// let widget = gtk4::Button::new();
///
/// let ctx = glib::MainContext::default();
/// ctx.spawn_async(async move {
///     let identifier = WindowIdentifier::from_native(&widget.native().unwrap()).await;
///
///     /// Open some portals
/// });
/// ```
/// The constructor should return a valid identifier under both X11 and Wayland
/// and fallback to the [`Default`] implementation otherwise.
///
/// ## Other Toolkits
///
/// If you have access to `RawWindowHandle` you can convert it to a
/// [`WindowIdentifier`] with
///
/// ```rust, ignore
/// let handle = RawWindowHandle::Xlib(XlibHandle::empty());
/// let identifier = WindowIdentifier::from_raw_handle(handle, None);
///
/// /// Open some portals
/// ```
///
/// In case you don't have access to a WindowIdentifier:
/// ```rust
/// use ashpd::WindowIdentifier;
///
/// let identifier = WindowIdentifier::default();
/// ```
#[derive(Default, Type)]
#[zvariant(signature = "s")]
#[doc(alias = "XdpParent")]
pub enum WindowIdentifier {
    /// Gtk 4 Window Identifier
    #[cfg(feature = "gtk4")]
    #[doc(hidden)]
    Gtk4(Gtk4WindowIdentifier),
    #[cfg(feature = "wayland")]
    #[doc(hidden)]
    Wayland(WaylandWindowIdentifier),
    #[doc(hidden)]
    X11(WindowIdentifierType),
    #[doc(hidden)]
    #[default]
    None,
}

unsafe impl Send for WindowIdentifier {}
unsafe impl Sync for WindowIdentifier {}

impl Serialize for WindowIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for WindowIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "gtk4")]
            Self::Gtk4(identifier) => f.write_str(&format!("{identifier}")),
            #[cfg(feature = "wayland")]
            Self::Wayland(identifier) => f.write_str(&format!("{identifier}")),
            Self::X11(identifier) => f.write_str(&format!("{identifier}")),
            Self::None => f.write_str(""),
        }
    }
}

impl std::fmt::Debug for WindowIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WindowIdentifier")
            .field(&format!("{self}"))
            .finish()
    }
}

impl WindowIdentifier {
    #[cfg(feature = "gtk4")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gtk4")))]
    /// Creates a [`WindowIdentifier`] from a [`gtk4::Native`](https://docs.gtk.org/gtk4/class.Native.html).
    ///
    /// The constructor returns a valid handle under both Wayland & x11.
    ///
    /// **Note** the function has to be async as the Wayland handle retrieval
    /// API is async as well.
    #[doc(alias = "xdp_parent_new_gtk")]
    pub async fn from_native(native: &impl ::gtk4::glib::IsA<::gtk4::Native>) -> Self {
        match Gtk4WindowIdentifier::new(native).await {
            Some(identifier) => Self::Gtk4(identifier),
            None => Self::default(),
        }
    }

    #[cfg(feature = "raw_handle")]
    #[cfg_attr(docsrs, doc(cfg(feature = "raw_handle")))]
    /// Create an instance of [`WindowIdentifier`] from a
    /// [`RawWindowHandle`](raw_window_handle::RawWindowHandle).
    ///
    /// The constructor returns a valid handle under both Wayland & X11.
    ///
    /// This method is only async and requires a `RawDisplayHandle` only for
    /// Wayland handles.
    pub async fn from_raw_handle(
        window_handle: &raw_window_handle::RawWindowHandle,
        display_handle: Option<&raw_window_handle::RawDisplayHandle>,
    ) -> Self {
        use raw_window_handle::{
            RawDisplayHandle::Wayland as DisplayHandle,
            RawWindowHandle::{Wayland, Xcb, Xlib},
        };
        match (window_handle, display_handle) {
            (Wayland(wl_handle), Some(DisplayHandle(wl_display))) => unsafe {
                Self::from_wayland_raw(wl_handle.surface, wl_display.display).await
            },
            (Xlib(x_handle), _) => Self::from_xid(x_handle.window),
            (Xcb(x_handle), _) => Self::from_xid(x_handle.window.into()),
            _ => Self::default(), // Fallback to default
        }
    }

    /// Create an instance of [`WindowIdentifier`] from an X11 window's XID.
    pub fn from_xid(xid: std::os::raw::c_ulong) -> Self {
        Self::X11(WindowIdentifierType::X11(xid))
    }

    #[cfg(feature = "wayland")]
    #[cfg_attr(docsrs, doc(cfg(feature = "wayland")))]
    /// Create an instance of [`WindowIdentifier`] from a Wayland surface.
    ///
    /// # Safety
    ///
    /// Both pointers have to be valid surface and display pointers. You must
    /// ensure the `display_ptr` lives longer than the returned
    /// `WindowIdentifier`.
    pub async unsafe fn from_wayland_raw(
        surface_ptr: *mut std::ffi::c_void,
        display_ptr: *mut std::ffi::c_void,
    ) -> Self {
        match WaylandWindowIdentifier::from_raw(surface_ptr, display_ptr).await {
            Some(identifier) => Self::Wayland(identifier),
            None => Self::default(),
        }
    }

    #[cfg(feature = "wayland")]
    #[cfg_attr(docsrs, doc(cfg(feature = "wayland")))]
    /// Create an instance of [`WindowIdentifier`] from a Wayland surface.
    pub async fn from_wayland(surface: &wayland_client::protocol::wl_surface::WlSurface) -> Self {
        match WaylandWindowIdentifier::new(surface).await {
            Some(identifier) => Self::Wayland(identifier),
            None => Self::default(),
        }
    }
}

#[cfg(all(feature = "raw_handle", feature = "gtk4"))]
unsafe impl HasRawDisplayHandle for WindowIdentifier {
    /// Convert a [`WindowIdentifier`] to
    /// [`RawDisplayHandle`](raw_window_handle::RawDisplayHandle`).
    ///
    /// # Panics
    ///
    /// If you attempt to convert a [`WindowIdentifier`] created from a
    /// [`RawDisplayHandle`](raw_window_handle::RawDisplayHandle`) instead of
    /// the gtk4 constructors.
    fn raw_display_handle(&self) -> RawDisplayHandle {
        match self {
            #[cfg(feature = "gtk4")]
            Self::Gtk4(identifier) => identifier.as_raw_display_handle(),
            _ => unreachable!(),
        }
    }
}

#[cfg(all(feature = "raw_handle", feature = "gtk4"))]
unsafe impl HasRawWindowHandle for WindowIdentifier {
    /// Convert a [`WindowIdentifier`] to
    /// [`RawWindowHandle`](raw_window_handle::RawWindowHandle`).
    ///
    /// # Panics
    ///
    /// If you attempt to convert a [`WindowIdentifier`] created from a
    /// [`RawWindowHandle`](raw_window_handle::RawWindowHandle`) instead of
    /// the gtk4 constructors.
    fn raw_window_handle(&self) -> RawWindowHandle {
        match self {
            #[cfg(feature = "gtk4")]
            Self::Gtk4(identifier) => identifier.as_raw_window_handle(),
            _ => unreachable!(),
        }
    }
}

/// Supported WindowIdentifier kinds
#[derive(Debug, Clone, PartialEq, Eq, Type)]
#[zvariant(signature = "s")]
pub enum WindowIdentifierType {
    X11(std::os::raw::c_ulong),
    #[allow(dead_code)]
    Wayland(String),
}

impl fmt::Display for WindowIdentifierType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X11(xid) => f.write_str(&format!("x11:0x{xid:x}")),
            Self::Wayland(handle) => f.write_str(&format!("wayland:{handle}")),
        }
    }
}

impl FromStr for WindowIdentifierType {
    type Err = PortalError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (kind, handle) = s
            .split_once(':')
            .ok_or_else(|| PortalError::InvalidArgument("Invalid Window Identifier".to_owned()))?;
        match kind {
            "x11" => {
                let handle = handle.trim_start_matches("0x");
                Ok(Self::X11(
                    std::os::raw::c_ulong::from_str_radix(handle, 16)
                        .map_err(|_| PortalError::InvalidArgument(format!("Wrong XID {handle}")))?,
                ))
            }
            "wayland" => Ok(Self::Wayland(handle.to_owned())),
            t => Err(PortalError::InvalidArgument(format!(
                "Invalid Window Identifier type {t}",
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for WindowIdentifierType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let handle = String::deserialize(deserializer)?;
        Self::from_str(&handle)
            .map_err(|e| serde::de::Error::custom(format!("Invalid Window identifier {e}")))
    }
}

#[cfg(feature = "gtk4")]
mod gtk4;

#[cfg(feature = "gtk4")]
pub use self::gtk4::Gtk4WindowIdentifier;
use crate::PortalError;

#[cfg(feature = "wayland")]
mod wayland;

#[cfg(feature = "wayland")]
pub use self::wayland::WaylandWindowIdentifier;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::WindowIdentifier;
    use crate::window_identifier::WindowIdentifierType;

    #[test]
    fn test_serialize() {
        let x11 = WindowIdentifier::from_xid(1024);
        assert_eq!(x11.to_string(), "x11:0x400");

        assert_eq!(WindowIdentifier::default().to_string(), "");

        assert_eq!(
            WindowIdentifierType::from_str("x11:0x11432").unwrap(),
            WindowIdentifierType::X11(70706)
        );

        assert_eq!(
            WindowIdentifierType::from_str("wayland:Somerandomchars").unwrap(),
            WindowIdentifierType::Wayland("Somerandomchars".to_owned())
        );
        assert!(WindowIdentifierType::from_str("some_handle").is_err());
        assert!(WindowIdentifierType::from_str("some_type:some_handle").is_err());
    }
}
