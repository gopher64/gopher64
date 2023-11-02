//! Request access to specific devices such as camera, speakers or microphone.
//!
//! **Note** This portal doesn't work for sandboxed applications.
//!
//! ### Examples
//!
//! Access a [`Device`]
//!
//! ```rust,no_run
//! use ashpd::desktop::device::{Device, DeviceProxy};
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = DeviceProxy::new().await?;
//!     proxy.access_device(6879, &[Device::Speakers]).await?;
//!     Ok(())
//! }
//! ```

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use zbus::zvariant::{SerializeDict, Type};

use super::{HandleToken, Request};
use crate::{proxy::Proxy, Error};

#[derive(SerializeDict, Type, Debug, Default)]
/// Specified options for a [`DeviceProxy::access_device`] request.
#[zvariant(signature = "dict")]
struct AccessDeviceOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[zvariant(signature = "s")]
#[serde(rename_all = "lowercase")]
/// The possible device to request access to.
pub enum Device {
    /// A microphone.
    Microphone,
    /// Speakers.
    Speakers,
    /// A Camera.
    Camera,
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Microphone => write!(f, "Microphone"),
            Self::Speakers => write!(f, "Speakers"),
            Self::Camera => write!(f, "Camera"),
        }
    }
}

impl AsRef<str> for Device {
    fn as_ref(&self) -> &str {
        match self {
            Self::Microphone => "Microphone",
            Self::Speakers => "Speakers",
            Self::Camera => "Camera",
        }
    }
}

impl From<Device> for &'static str {
    fn from(d: Device) -> Self {
        match d {
            Device::Microphone => "Microphone",
            Device::Speakers => "Speakers",
            Device::Camera => "Camera",
        }
    }
}

impl FromStr for Device {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Microphone" | "microphone" => Ok(Device::Microphone),
            "Speakers" | "speakers" => Ok(Device::Speakers),
            "Camera" | "camera" => Ok(Device::Camera),
            _ => Err(Error::ParseError("Failed to parse device, invalid value")),
        }
    }
}

/// The interface lets services ask if an application should get access to
/// devices such as microphones, speakers or cameras. Not a portal in the strict
/// sense, since the API is not directly accessible to applications inside the
/// sandbox.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Device`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Device).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Device")]
pub struct DeviceProxy<'a>(Proxy<'a>);

impl<'a> DeviceProxy<'a> {
    /// Create a new instance of [`DeviceProxy`].
    pub async fn new() -> Result<DeviceProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Device").await?;
        Ok(Self(proxy))
    }

    /// Asks for access to a device.
    ///
    /// # Arguments
    ///
    /// * `pid` - The pid of the application on whose behalf the request is
    ///   made.
    /// * `devices` - A list of devices to request access to.
    ///
    /// *Note* Asking for multiple devices at the same time may or may not be
    /// supported
    ///
    /// # Specifications
    ///
    /// See also [`AccessDevice`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Device.AccessDevice).
    #[doc(alias = "AccessDevice")]
    pub async fn access_device(&self, pid: u32, devices: &[Device]) -> Result<Request<()>, Error> {
        let options = AccessDeviceOptions::default();
        self.0
            .empty_request(
                &options.handle_token,
                "AccessDevice",
                &(pid, devices, &options),
            )
            .await
    }
}
