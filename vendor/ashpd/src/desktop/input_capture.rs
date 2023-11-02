use std::{
    collections::HashMap,
    os::fd::{IntoRawFd, RawFd},
};

use enumflags2::{bitflags, BitFlags};
use futures_util::{Stream, TryFutureExt};
use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};
use zbus::zvariant::{
    DeserializeDict, ObjectPath, OwnedFd, OwnedObjectPath, OwnedValue, SerializeDict, Type, Value,
};

use super::{HandleToken, Request, Session};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Debug, Copy, Clone, Type)]
#[bitflags]
#[repr(u32)]
/// Supported capabilities
pub enum Capabilities {
    /// Keyboard
    Keyboard,
    /// Pointer
    Pointer,
    /// Touchscreen
    Touchscreen,
}

#[derive(Debug, SerializeDict, Type)]
#[zvariant(signature = "dict")]
struct CreateSessionOptions {
    handle_token: HandleToken,
    session_handle_token: HandleToken,
    capabilities: BitFlags<Capabilities>,
}

#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "dict")]
struct CreateSessionResponse {
    session_handle: OwnedObjectPath,
    capabilities: BitFlags<Capabilities>,
}

#[derive(Default, Debug, SerializeDict, Type)]
#[zvariant(signature = "dict")]
struct GetZonesOptions {
    handle_token: HandleToken,
}

#[derive(Default, Debug, SerializeDict, Type)]
#[zvariant(signature = "dict")]
struct SetPointerBarriersOptions {
    handle_token: HandleToken,
}

#[derive(Default, Debug, SerializeDict, Type)]
#[zvariant(signature = "dict")]
struct EnableOptions {}

#[derive(Default, Debug, SerializeDict, Type)]
#[zvariant(signature = "dict")]
struct DisableOptions {}

#[derive(Default, Debug, SerializeDict, Type)]
#[zvariant(signature = "dict")]
struct ReleaseOptions {
    activation_id: u32,
    cursor_position: (f64, f64),
}

/// Indicates that an input capturing session was disabled.
#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "oa{sv}")]
pub struct Disabled(OwnedObjectPath, HashMap<String, OwnedValue>);

impl Disabled {
    /// Session that was disabled.
    pub fn session_handle(&self) -> ObjectPath<'_> {
        self.0.as_ref()
    }
}

#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "dict")]
struct DeactivatedOptions {
    activation_id: u32,
}

/// Indicates that an input capturing session was disabled.
#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "oa{sv}")]
pub struct Deactivated(OwnedObjectPath, DeactivatedOptions);

impl Deactivated {
    /// Session that was deactivated.
    pub fn session_handle(&self) -> ObjectPath<'_> {
        self.0.as_ref()
    }

    /// The same activation_id number as in the corresponding "Activated"
    /// signal.
    pub fn activation_id(&self) -> u32 {
        self.1.activation_id
    }
}

#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "dict")]

struct ActivatedOptions {
    activation_id: u32,
    cursor_position: (f32, f32),
    barrier_id: BarrierID,
}

/// Indicates that an input capturing session was disabled.
#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "oa{sv}")]
pub struct Activated(OwnedObjectPath, ActivatedOptions);

impl Activated {
    /// Session that was activated.
    pub fn session_handle(&self) -> ObjectPath<'_> {
        self.0.as_ref()
    }

    /// A number that can be used to synchronize with the transport-layer.
    pub fn activation_id(&self) -> u32 {
        self.1.activation_id
    }

    /// The current cursor position in the same coordinate space as the zones.
    pub fn cursor_position(&self) -> (f32, f32) {
        self.1.cursor_position
    }

    /// The barrier id of the barrier that triggered
    pub fn barrier_id(&self) -> BarrierID {
        self.1.barrier_id
    }
}

#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "dict")]
struct ZonesChangedOptions {
    zone_set: u32,
}

/// Indicates that an input capturing session was disabled.
#[derive(Debug, Deserialize, Type)]
#[zvariant(signature = "oa{sv}")]
pub struct ZonesChanged(OwnedObjectPath, ZonesChangedOptions);

impl ZonesChanged {
    /// Session that was deactivated.
    pub fn session_handle(&self) -> ObjectPath<'_> {
        self.0.as_ref()
    }

    ///  The zone_set ID of the invalidated zone.
    pub fn zone_set(&self) -> u32 {
        self.1.zone_set
    }
}

/// A region of a [`Zones`].
#[derive(Debug, Clone, Copy, Deserialize, Type)]
#[zvariant(signature = "uuii")]
pub struct Region(u32, u32, i32, i32);

impl Region {
    /// The width.
    pub fn width(self) -> u32 {
        self.0
    }

    /// The height
    pub fn height(self) -> u32 {
        self.1
    }

    /// The x offset.
    pub fn x_offset(self) -> i32 {
        self.2
    }

    /// The y offset.
    pub fn y_offset(self) -> i32 {
        self.3
    }
}

/// A response of [`InputCapture::zones`].
#[derive(Debug, Type, Deserialize)]
#[zvariant(signature = "a(uuii)u")]
pub struct Zones(Vec<Region>, u32);

impl Zones {
    /// A list of regions.
    pub fn regions(&self) -> &[Region] {
        &self.0
    }

    /// A unique ID to be used in [`InputCapture::set_pointer_barriers`].
    pub fn zone_set(&self) -> u32 {
        self.1
    }
}

/// A barrier ID.
pub type BarrierID = u32;

#[derive(Debug, SerializeDict, Type)]
#[zvariant(signature = "dict")]
/// Input Barrier.
pub struct Barrier {
    barrier_id: BarrierID,
    position: (i32, i32, i32, i32),
}

impl Barrier {
    /// Create a new barrier.
    pub fn new(barrier_id: BarrierID, position: (i32, i32, i32, i32)) -> Self {
        Self {
            barrier_id,
            position,
        }
    }
}

/// A response to [`InputCapture::set_pointer_barriers`]
#[derive(Debug, DeserializeDict, Type)]
#[zvariant(signature = "dict")]
pub struct SetPointerBarriersResponse {
    failed_barriers: Vec<BarrierID>,
}

impl SetPointerBarriersResponse {
    /// List of pointer barriers that have been denied
    pub fn failed_barriers(&self) -> &[BarrierID] {
        &self.failed_barriers
    }
}

/// Wrapper of the DBus interface: [`org.freedesktop.portal.InputCapture`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.InputCapture).
#[doc(alias = "org.freedesktop.portal.InputCapture")]
pub struct InputCapture<'a>(Proxy<'a>);

impl<'a> InputCapture<'a> {
    /// Create a new instance of [`InputCapture`].
    pub async fn new() -> Result<InputCapture<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.InputCapture").await?;
        Ok(Self(proxy))
    }

    /// Create an input capture session.
    ///
    /// # Specifications
    ///
    /// See also [`CreateSession`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-InputCapture.CreateSession).
    pub async fn create_session(
        &self,
        parent_window: &WindowIdentifier,
        capabilities: BitFlags<Capabilities>,
    ) -> Result<(Session<'_>, BitFlags<Capabilities>), Error> {
        let options = CreateSessionOptions {
            handle_token: Default::default(),
            session_handle_token: Default::default(),
            capabilities,
        };

        let (request, proxy) = futures_util::try_join!(
            self.0
                .request::<CreateSessionResponse>(
                    &options.handle_token,
                    "CreateSession",
                    (parent_window, &options)
                )
                .into_future(),
            Session::from_unique_name(&options.session_handle_token).into_future(),
        )?;
        let response = request.response()?;
        assert_eq!(proxy.path(), &response.session_handle.as_ref());
        Ok((proxy, response.capabilities))
    }

    /// A set of currently available input zones for this session.
    ///
    /// # Specifications
    ///
    /// See also [`CreateSession`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-InputCapture.CreateSession).
    #[doc(alias = "GetZones")]
    pub async fn zones(&self, session: &Session<'_>) -> Result<Request<Zones>, Error> {
        let options = GetZonesOptions::default();
        self.0
            .request(&options.handle_token, "GetZones", (session, &options))
            .await
    }

    /// Set up zero or more pointer barriers.
    ///
    /// # Specifications
    ///
    /// See also [`SetPointerBarriers`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-InputCapture.SetPointerBarriers).
    #[doc(alias = "SetPointerBarriers")]
    pub async fn set_pointer_barriers(
        &self,
        session: &Session<'_>,
        barriers: &[Barrier],
        zone_set: u32,
    ) -> Result<Request<SetPointerBarriersResponse>, Error> {
        let options = SetPointerBarriersOptions::default();
        self.0
            .request(
                &options.handle_token,
                "SetPointerBarriers",
                &(session, &options, barriers, zone_set),
            )
            .await
    }

    /// Enable input capturing.
    ///
    /// # Specifications
    ///
    /// See also [`Enable`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-InputCapture.Enable).
    pub async fn enable(&self, session: &Session<'_>) -> Result<(), Error> {
        let options = EnableOptions::default();
        self.0.call("Enable", &(session, &options)).await
    }

    /// Disable input capturing.
    ///
    /// # Specifications
    ///
    /// See also [`Disable`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-InputCapture.Disable).
    pub async fn disable(&self, session: &Session<'_>) -> Result<(), Error> {
        let options = DisableOptions::default();
        self.0.call("Disable", &(session, &options)).await
    }

    /// Release any ongoing input capture.
    ///
    /// # Specifications
    ///
    /// See also [`Release`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-InputCapture.Release).
    pub async fn release(
        &self,
        session: &Session<'_>,
        activation_id: u32,
        cursor_position: (f64, f64),
    ) -> Result<(), Error> {
        let options = ReleaseOptions {
            activation_id,
            cursor_position,
        };
        self.0.call("Release", &(session, &options)).await
    }

    /// Connect to EIS.
    ///
    /// # Specifications
    ///
    /// See also [`ConnectToEIS`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-InputCapture.ConnectToEIS).
    #[doc(alias = "ConnectToEIS")]
    pub async fn connect_to_eis(&self, session: &Session<'_>) -> Result<RawFd, Error> {
        // `ConnectToEIS` doesn't take any options for now
        let options: HashMap<&str, Value<'_>> = HashMap::new();
        let fd = self
            .0
            .call::<OwnedFd>("ConnectToEIS", &(session, options))
            .await?;
        Ok(fd.into_raw_fd())
    }

    /// Signal emitted when the application will no longer receive captured
    /// events.
    ///
    /// # Specifications
    ///
    /// See also [`Disabled`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-InputCapture.Disabled).
    #[doc(alias = "Disabled")]
    pub async fn receive_disabled(&self) -> Result<impl Stream<Item = Disabled>, Error> {
        self.0.signal("Disabled").await
    }

    /// Signal emitted when the application will no longer receive captured
    /// events.
    ///
    /// # Specifications
    ///
    /// See also [`Activated`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-InputCapture.Activated).
    #[doc(alias = "Activated")]
    pub async fn receive_activated(&self) -> Result<impl Stream<Item = Activated>, Error> {
        self.0.signal("Activated").await
    }

    /// Signal emitted when the application will no longer receive captured
    /// events.
    ///
    /// # Specifications
    ///
    /// See also [`Deactivated`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-InputCapture.Deactivated).
    #[doc(alias = "Deactivated")]
    pub async fn receive_deactivated(&self) -> Result<impl Stream<Item = Deactivated>, Error> {
        self.0.signal("Deactivated").await
    }

    /// Signal emitted when the application will no longer receive captured
    /// events.
    ///
    /// # Specifications
    ///
    /// See also [`ZonesChanged`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-InputCapture.ZonesChanged).
    #[doc(alias = "ZonesChanged")]
    pub async fn receive_zones_changed(&self) -> Result<impl Stream<Item = ZonesChanged>, Error> {
        self.0.signal("ZonesChanged").await
    }

    /// Supported capabilities.
    ///
    /// # Specifications
    ///
    /// See also [`SupportedCapabilities`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-property-org-freedesktop-portal-InputCapture.SupportedCapabilities).
    #[doc(alias = "SupportedCapabilities")]
    pub async fn supported_capabilities(&self) -> Result<BitFlags<Capabilities>, Error> {
        self.0.property("SupportedCapabilities").await
    }
}
