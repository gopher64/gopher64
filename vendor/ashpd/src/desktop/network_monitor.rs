//! **Note** This portal doesn't work for sandboxed applications.
//! # Examples
//!
//! ```rust,no_run
//! use ashpd::desktop::network_monitor::NetworkMonitor;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = NetworkMonitor::new().await?;
//!
//!     println!("{}", proxy.can_reach("www.google.com", 80).await?);
//!     println!("{}", proxy.is_available().await?);
//!     println!("{:#?}", proxy.connectivity().await?);
//!     println!("{}", proxy.is_metered().await?);
//!     println!("{:#?}", proxy.status().await?);
//!
//!     Ok(())
//! }
//! ```

use std::fmt;

use futures_util::Stream;
use serde_repr::Deserialize_repr;
use zbus::zvariant::{DeserializeDict, Type};

use crate::{proxy::Proxy, Error};

#[derive(DeserializeDict, Type, Debug)]
/// The network status, composed of the availability, metered & connectivity
#[zvariant(signature = "dict")]
pub struct NetworkStatus {
    /// Whether the network is considered available.
    available: bool,
    /// Whether the network is considered metered.
    metered: bool,
    /// More detailed information about the host's network connectivity
    connectivity: Connectivity,
}

impl NetworkStatus {
    /// Returns whether the network is considered available.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Returns whether the network is considered metered.
    pub fn is_metered(&self) -> bool {
        self.metered
    }

    /// Returns more detailed information about the host's network connectivity.
    pub fn connectivity(&self) -> Connectivity {
        self.connectivity
    }
}

#[derive(Deserialize_repr, PartialEq, Eq, Debug, Clone, Copy, Type)]
#[repr(u32)]
/// Host's network activity
pub enum Connectivity {
    /// The host is not configured with a route to the internet.
    Local = 1,
    /// The host is connected to a network, but can't reach the full internet.
    Limited = 2,
    /// The host is behind a captive portal and cannot reach the full internet.
    CaptivePortal = 3,
    /// The host connected to a network, and can reach the full internet.
    FullNetwork = 4,
}

impl fmt::Display for Connectivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let connectivity = match self {
            Self::Local => "local",
            Self::Limited => "limited",
            Self::CaptivePortal => "captive portal",
            Self::FullNetwork => "full network",
        };
        f.write_str(connectivity)
    }
}

/// The interface provides network status information to sandboxed applications.
///
/// It is not a portal in the strict sense, since it does not involve user
/// interaction. Applications are expected to use this interface indirectly,
/// via a library API such as the GLib [`gio::NetworkMonitor`](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/struct.NetworkMonitor.html) interface.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.NetworkMonitor`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.NetworkMonitor).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.NetworkMonitor")]
pub struct NetworkMonitor<'a>(Proxy<'a>);

impl<'a> NetworkMonitor<'a> {
    /// Create a new instance of [`NetworkMonitor`].
    pub async fn new() -> Result<NetworkMonitor<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.NetworkMonitor").await?;
        Ok(Self(proxy))
    }

    /// Returns whether the given hostname is believed to be reachable.
    ///
    /// # Arguments
    ///
    /// * `hostname` - The hostname to reach.
    /// * `port` - The port to reach.
    ///
    /// # Required version
    ///
    /// The method requires the 3nd version implementation of the portal and
    /// would fail with [`Error::RequiresVersion`] otherwise.
    ///
    /// # Specifications
    ///
    /// See also [`CanReach`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-NetworkMonitor.CanReach).
    #[doc(alias = "CanReach")]
    pub async fn can_reach(&self, hostname: &str, port: u32) -> Result<bool, Error> {
        self.0
            .call_versioned("CanReach", &(hostname, port), 3)
            .await
    }

    /// Returns whether the network is considered available.
    /// That is, whether the system as a default route for at least one of IPv4
    /// or IPv6.
    ///
    /// # Required version
    ///
    /// The method requires the 2nd version implementation of the portal and
    /// would fail with [`Error::RequiresVersion`] otherwise.
    ///
    /// # Specifications
    ///
    /// See also [`GetAvailable`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-NetworkMonitor.GetAvailable).
    #[doc(alias = "GetAvailable")]
    #[doc(alias = "get_available")]
    pub async fn is_available(&self) -> Result<bool, Error> {
        self.0.call_versioned("GetAvailable", &(), 2).await
    }

    /// Returns more detailed information about the host's network connectivity.
    ///
    /// # Required version
    ///
    /// The method requires the 2nd version implementation of the portal and
    /// would fail with [`Error::RequiresVersion`] otherwise.
    ///
    /// # Specifications
    ///
    /// See also [`GetConnectivity`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-NetworkMonitor.GetConnectivity).
    #[doc(alias = "GetConnectivity")]
    #[doc(alias = "get_connectivity")]
    pub async fn connectivity(&self) -> Result<Connectivity, Error> {
        self.0.call_versioned("GetConnectivity", &(), 2).await
    }

    /// Returns whether the network is considered metered.
    /// That is, whether the system as traffic flowing through the default
    /// connection that is subject to limitations by service providers.
    ///
    /// # Required version
    ///
    /// The method requires the 2nd version implementation of the portal and
    /// would fail with [`Error::RequiresVersion`] otherwise.
    ///
    /// # Specifications
    ///
    /// See also [`GetMetered`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-NetworkMonitor.GetMetered).
    #[doc(alias = "GetMetered")]
    #[doc(alias = "get_metered")]
    pub async fn is_metered(&self) -> Result<bool, Error> {
        self.0.call_versioned("GetMetered", &(), 2).await
    }

    /// Returns the three values all at once.
    ///
    /// # Required version
    ///
    /// The method requires the 3nd version implementation of the portal and
    /// would fail with [`Error::RequiresVersion`] otherwise.
    ///
    /// # Specifications
    ///
    /// See also [`GetStatus`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-NetworkMonitor.GetStatus).
    #[doc(alias = "GetStatus")]
    #[doc(alias = "get_status")]
    pub async fn status(&self) -> Result<NetworkStatus, Error> {
        self.0.call_versioned("GetStatus", &(), 3).await
    }

    /// Emitted when the network configuration changes.
    ///
    /// # Specifications
    ///
    /// See also [`changed`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-NetworkMonitor.changed).
    pub async fn receive_changed(&self) -> Result<impl Stream<Item = ()>, Error> {
        self.0.signal("changed").await
    }
}
