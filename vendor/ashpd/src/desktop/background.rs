//! Request to run in the background or started automatically when the user
//! logs in.
//!
//! **Note** This portal only works for sandboxed applications.
//!
//! Wrapper of the DBus interface: [`org.freedesktop.portal.Background`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Background).
//!
//! ### Examples
//!
//! ```rust,no_run
//! use ashpd::desktop::background::Background;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let response = Background::request()
//!         .reason("Automatically fetch your latest mails")
//!         .auto_start(true)
//!         .command(&["geary"])
//!         .dbus_activatable(false)
//!         .send()
//!         .await?
//!         .response()?;
//!
//!     println!("{}", response.auto_start());
//!     println!("{}", response.run_in_background());
//!
//!     Ok(())
//! }
//! ```
//!
//! If no `command` is provided, the [`Exec`](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#exec-variables) line from the [desktop
//! file](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#introduction) will be used.

use serde::Serialize;
use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

use super::{HandleToken, Request};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct BackgroundOptions {
    handle_token: HandleToken,
    reason: Option<String>,
    autostart: Option<bool>,
    #[zvariant(rename = "dbus-activatable")]
    dbus_activatable: Option<bool>,
    #[zvariant(rename = "commandline")]
    command: Option<Vec<String>>,
}

#[derive(DeserializeDict, Type, Debug)]
/// The response of a [`BackgroundRequest`] request.
#[zvariant(signature = "dict")]
pub struct Background {
    background: bool,
    autostart: bool,
}

impl Background {
    /// Creates a new builder-pattern struct instance to construct
    /// [`Background`].
    ///
    /// This method returns an instance of [`BackgroundRequest`].
    pub fn request() -> BackgroundRequest {
        BackgroundRequest::default()
    }

    /// If the application is allowed to run in the background.
    pub fn run_in_background(&self) -> bool {
        self.background
    }

    /// If the application will be auto-started.
    pub fn auto_start(&self) -> bool {
        self.autostart
    }
}

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct SetStatusOptions {
    message: String,
}

/// The interface lets sandboxed applications request that the application
/// is allowed to run in the background or started automatically when the user
/// logs in.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Background`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Background).
#[doc(alias = "org.freedesktop.portal.Background")]
pub struct BackgroundProxy<'a>(Proxy<'a>);

impl<'a> BackgroundProxy<'a> {
    /// Create a new instance of [`BackgroundProxy`].
    pub async fn new() -> Result<BackgroundProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Background").await?;
        Ok(Self(proxy))
    }

    ///  Sets the status of the application running in background.
    ///
    /// # Arguments
    ///
    /// * `message` - A string that will be used as the status message of the
    ///   application.
    ///
    /// # Required version
    ///
    /// The method requires the 2nd version implementation of the portal and
    /// would fail with [`Error::RequiresVersion`] otherwise.
    ///
    /// # Specifications
    ///
    /// See also [`SetStatus`](https://flatpak.github.io/xdg-desktop-portal/#gdbus-method-org-freedesktop-portal-Background.SetStatus).

    pub async fn set_status(&self, message: &str) -> Result<(), Error> {
        self.0
            .call_versioned(
                "SetStatus",
                &(SetStatusOptions {
                    message: message.to_owned(),
                }),
                2,
            )
            .await
    }

    async fn request_background(
        &self,
        identifier: &WindowIdentifier,
        options: BackgroundOptions,
    ) -> Result<Request<Background>, Error> {
        self.0
            .request(
                &options.handle_token,
                "RequestBackground",
                (&identifier, &options),
            )
            .await
    }
}

#[doc(alias = "xdp_portal_request_background")]
/// A [builder-pattern] type to construct [`Background`].
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[derive(Debug, Default)]
pub struct BackgroundRequest {
    identifier: WindowIdentifier,
    options: BackgroundOptions,
}

impl BackgroundRequest {
    #[must_use]
    /// Sets a window identifier.
    pub fn identifier(mut self, identifier: impl Into<Option<WindowIdentifier>>) -> Self {
        self.identifier = identifier.into().unwrap_or_default();
        self
    }

    #[must_use]
    /// Sets whether to auto start the application or not.
    pub fn auto_start(mut self, auto_start: impl Into<Option<bool>>) -> Self {
        self.options.autostart = auto_start.into();
        self
    }

    #[must_use]
    /// Sets whether the application is dbus activatable.
    pub fn dbus_activatable(mut self, dbus_activatable: impl Into<Option<bool>>) -> Self {
        self.options.dbus_activatable = dbus_activatable.into();
        self
    }

    #[must_use]
    /// Specifies the command line to execute.
    /// If this is not specified, the [`Exec`](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#exec-variables) line from the [desktop
    /// file](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#introduction)
    pub fn command<P: IntoIterator<Item = I>, I: AsRef<str> + Type + Serialize>(
        mut self,
        command: impl Into<Option<P>>,
    ) -> Self {
        self.options.command = command
            .into()
            .map(|a| a.into_iter().map(|s| s.as_ref().to_owned()).collect());
        self
    }

    #[must_use]
    /// Sets a user-visible reason for the request.
    pub fn reason<'a>(mut self, reason: impl Into<Option<&'a str>>) -> Self {
        self.options.reason = reason.into().map(ToOwned::to_owned);
        self
    }

    /// Build the [`Background`].
    pub async fn send(self) -> Result<Request<Background>, Error> {
        let proxy = BackgroundProxy::new().await?;
        proxy
            .request_background(&self.identifier, self.options)
            .await
    }
}
