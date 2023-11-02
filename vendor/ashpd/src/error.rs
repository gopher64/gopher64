use zbus::DBusError;

use crate::desktop::{dynamic_launcher::UnexpectedIconError, request::ResponseError};

/// An error type that describes the various DBus errors.
///
/// See <https://github.com/flatpak/xdg-desktop-portal/blob/master/src/xdp-utils.h#L119-L127>.
#[allow(missing_docs)]
#[derive(DBusError, Debug)]
#[dbus_error(prefix = "org.freedesktop.portal.Error")]
pub enum PortalError {
    #[dbus_error(zbus_error)]
    /// ZBus specific error.
    ZBus(zbus::Error),
    /// Request failed.
    Failed,
    /// Invalid arguments passed.
    InvalidArgument(String),
    /// Not found.
    NotFound(String),
    /// Exists already.
    Exist(String),
    /// Method not allowed to be called.
    NotAllowed(String),
    /// Request cancelled.
    Cancelled(String),
    /// Window destroyed.
    WindowDestroyed(String),
}

#[derive(Debug)]
#[non_exhaustive]
/// The error type for ashpd.
pub enum Error {
    /// The portal request didn't succeed.
    Response(ResponseError),
    /// Something Failed on the portal request.
    Portal(PortalError),
    /// A zbus::fdo specific error.
    Zbus(zbus::Error),
    /// A signal returned no response.
    NoResponse,
    /// Failed to parse a string into an enum variant
    ParseError(&'static str),
    /// Input/Output
    IO(std::io::Error),
    /// A pipewire error
    #[cfg(feature = "pipewire")]
    Pipewire(pw::Error),
    /// Invalid AppId
    ///
    /// See <https://developer.gnome.org/documentation/tutorials/application-id.html#rules-for-application-ids>
    InvalidAppID,
    /// An error indicating that an interior nul byte was found
    NulTerminated(usize),
    /// Requires a newer interface version.
    ///
    /// The inner fields are the required version and the version advertised by
    /// the interface.
    RequiresVersion(u32, u32),
    /// An error indicating that a Icon::Bytes was expected but wrong type was
    /// passed
    UnexpectedIcon,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Response(e) => f.write_str(&format!("Portal request didn't succeed: {e}")),
            Self::Zbus(e) => f.write_str(&format!("ZBus Error: {e}")),
            Self::Portal(e) => f.write_str(&format!("Portal request failed: {e}")),
            Self::NoResponse => f.write_str("Portal error: no response"),
            Self::IO(e) => f.write_str(&format!("IO: {e}")),
            #[cfg(feature = "pipewire")]
            Self::Pipewire(e) => f.write_str(&format!("Pipewire: {e}")),
            Self::ParseError(e) => f.write_str(e),
            Self::InvalidAppID => f.write_str("Invalid app id"),
            Self::NulTerminated(u) => write!(f, "Nul byte found in provided data at position {u}"),
            Self::RequiresVersion(required, current) => write!(
                f,
                "This interface requires version {required}, but {current} is available"
            ),
            Self::UnexpectedIcon => write!(
                f,
                "Expected icon of type Icon::Bytes but a different type was used."
            ),
        }
    }
}

impl From<ResponseError> for Error {
    fn from(e: ResponseError) -> Self {
        Self::Response(e)
    }
}

impl From<PortalError> for Error {
    fn from(e: PortalError) -> Self {
        Self::Portal(e)
    }
}

#[cfg(feature = "pipewire")]
impl From<pw::Error> for Error {
    fn from(e: pw::Error) -> Self {
        Self::Pipewire(e)
    }
}

impl From<zbus::fdo::Error> for Error {
    fn from(e: zbus::fdo::Error) -> Self {
        Self::Zbus(zbus::Error::FDO(Box::new(e)))
    }
}

impl From<zbus::Error> for Error {
    fn from(e: zbus::Error) -> Self {
        Self::Zbus(e)
    }
}

impl From<zbus::zvariant::Error> for Error {
    fn from(e: zbus::zvariant::Error) -> Self {
        Self::Zbus(zbus::Error::Variant(e))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<UnexpectedIconError> for Error {
    fn from(_: UnexpectedIconError) -> Self {
        Self::UnexpectedIcon
    }
}
