//! Register global shortcuts

use std::{collections::HashMap, fmt::Debug, time::Duration};

use futures_util::{Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use zbus::zvariant::{
    DeserializeDict, ObjectPath, OwnedObjectPath, OwnedValue, SerializeDict, Type,
};

use super::{HandleToken, Request, Session};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[derive(Clone, SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct NewShortcutInfo {
    /// User-readable text describing what the shortcut does.
    description: String,
    /// The preferred shortcut trigger, defined as described by the "shortcuts"
    /// XDG specification. Optional.
    preferred_trigger: Option<String>,
}

/// Shortcut descriptor used to bind new shortcuts in
/// [`GlobalShortcuts::bind_shortcuts`]
#[derive(Clone, Serialize, Type, Debug)]
pub struct NewShortcut(String, NewShortcutInfo);

impl NewShortcut {
    /// Construct new shortcut
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self(
            id.into(),
            NewShortcutInfo {
                description: description.into(),
                preferred_trigger: None,
            },
        )
    }

    /// Sets the preferred shortcut trigger, defined as described by the
    /// "shortcuts" XDG specification.
    #[must_use]
    pub fn preferred_trigger<'a>(mut self, preferred_trigger: impl Into<Option<&'a str>>) -> Self {
        self.1.preferred_trigger = preferred_trigger.into().map(ToOwned::to_owned);
        self
    }
}

#[derive(Clone, DeserializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct ShortcutInfo {
    /// User-readable text describing what the shortcut does.
    description: String,
    /// User-readable text describing how to trigger the shortcut for the client
    /// to render.
    trigger_description: String,
}

/// Struct that contains information about existing binded shortcut.
///
/// If you need to create a new shortcuts, take a look at [`NewShortcut`]
/// instead.
#[derive(Clone, Deserialize, Type, Debug)]
pub struct Shortcut(String, ShortcutInfo);

impl Shortcut {
    /// Shortcut id
    pub fn id(&self) -> &str {
        &self.0
    }

    /// User-readable text describing what the shortcut does.
    pub fn description(&self) -> &str {
        &self.1.description
    }

    /// User-readable text describing how to trigger the shortcut for the client
    /// to render.
    pub fn trigger_description(&self) -> &str {
        &self.1.trigger_description
    }
}

/// Specified options for a [`GlobalShortcuts::create_session`] request.
#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct CreateSessionOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
    /// A string that will be used as the last element of the session handle.
    session_handle_token: HandleToken,
}

/// A response to a [`GlobalShortcuts::create_session`] request.
#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct CreateSession {
    session_handle: OwnedObjectPath,
}

/// Specified options for a [`GlobalShortcuts::bind_shortcuts`] request.
#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct BindShortcutsOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
}

/// A response to a [`GlobalShortcuts::bind_shortcuts`] request.
#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct BindShortcuts {
    shortcuts: Vec<Shortcut>,
}

impl BindShortcuts {
    /// A list of shortcuts.
    pub fn shortcuts(&self) -> &[Shortcut] {
        &self.shortcuts
    }
}

/// Specified options for a [`GlobalShortcuts::list_shortcuts`] request.
#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct ListShortcutsOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
}

/// A response to a [`GlobalShortcuts::list_shortcuts`] request.
#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct ListShortcuts {
    /// A list of shortcuts.
    shortcuts: Vec<Shortcut>,
}

impl ListShortcuts {
    /// A list of shortcuts.
    pub fn shortcuts(&self) -> &[Shortcut] {
        &self.shortcuts
    }
}

/// Notifies about a shortcut becoming active.
#[derive(Debug, Deserialize, Type)]
pub struct Activated(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>);

impl Activated {
    /// Session that requested the shortcut.
    pub fn session_handle(&self) -> ObjectPath<'_> {
        self.0.as_ref()
    }

    /// The application-provided ID for the shortcut.
    pub fn shortcut_id(&self) -> &str {
        &self.1
    }

    /// The timestamp, as seconds and microseconds since the Unix epoch.
    pub fn timestamp(&self) -> Duration {
        Duration::from_millis(self.2)
    }
}

/// Notifies that a shortcut is not active anymore.
#[derive(Debug, Deserialize, Type)]
pub struct Deactivated(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>);

impl Deactivated {
    /// Session that requested the shortcut.
    pub fn session_handle(&self) -> ObjectPath<'_> {
        self.0.as_ref()
    }

    /// The application-provided ID for the shortcut.
    pub fn shortcut_id(&self) -> &str {
        &self.1
    }

    /// The timestamp, as seconds and microseconds since the Unix epoch.
    pub fn timestamp(&self) -> Duration {
        Duration::from_millis(self.2)
    }
}

/// Indicates that the information associated with some of the shortcuts has
/// changed.
#[derive(Debug, Deserialize, Type)]
pub struct ShortcutsChanged(OwnedObjectPath, Vec<Shortcut>);

impl ShortcutsChanged {
    /// Session that requested the shortcut.
    pub fn session_handle(&self) -> ObjectPath<'_> {
        self.0.as_ref()
    }

    /// Shortcuts that have been registered.
    pub fn shortcuts(&self) -> &[Shortcut] {
        &self.1
    }
}

/// Wrapper of the DBus interface: [`org.freedesktop.portal.GlobalShortcuts`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.GlobalShortcuts).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.GlobalShortcuts")]
pub struct GlobalShortcuts<'a>(Proxy<'a>);

impl<'a> GlobalShortcuts<'a> {
    /// Create a new instance of [`GlobalShortcuts`].
    pub async fn new() -> Result<GlobalShortcuts<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.GlobalShortcuts").await?;
        Ok(Self(proxy))
    }

    /// Create a global shortcuts session.
    ///
    /// # Specifications
    ///
    /// See also [`CreateSession`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GlobalShortcuts.CreateSession).
    #[doc(alias = "CreateSession")]
    pub async fn create_session(&self) -> Result<Session<'a>, Error> {
        let options = CreateSessionOptions::default();
        let (request, proxy) = futures_util::try_join!(
            self.0
                .request::<CreateSession>(&options.handle_token, "CreateSession", &options)
                .into_future(),
            Session::from_unique_name(&options.session_handle_token).into_future(),
        )?;
        assert_eq!(proxy.path(), &request.response()?.session_handle.as_ref());
        Ok(proxy)
    }

    /// Bind the shortcuts.
    ///
    /// # Specifications
    ///
    /// See also [`BindShortcuts`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GlobalShortcuts.BindShortcuts).
    #[doc(alias = "BindShortcuts")]
    pub async fn bind_shortcuts(
        &self,
        session: &Session<'_>,
        shortcuts: &[NewShortcut],
        parent_window: &WindowIdentifier,
    ) -> Result<Request<BindShortcuts>, Error> {
        let options = BindShortcutsOptions::default();
        self.0
            .request(
                &options.handle_token,
                "BindShortcuts",
                &(session, shortcuts, parent_window, &options),
            )
            .await
    }

    /// Lists all shortcuts.
    ///
    /// # Specifications
    ///
    /// See also [`ListShortcuts`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GlobalShortcuts.ListShortcuts).
    #[doc(alias = "ListShortcuts")]
    pub async fn list_shortcuts(
        &self,
        session: &Session<'_>,
    ) -> Result<Request<ListShortcuts>, Error> {
        let options = ListShortcutsOptions::default();
        self.0
            .request(&options.handle_token, "ListShortcuts", &(session, &options))
            .await
    }

    /// Signal emitted when shortcut becomes active.
    ///
    /// # Specifications
    ///
    /// See also [`Activated`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-GlobalShortcuts.Activated).
    #[doc(alias = "Activated")]
    pub async fn receive_activated(&self) -> Result<impl Stream<Item = Activated>, Error> {
        self.0.signal("Activated").await
    }

    /// Signal emitted when shortcut is not active anymore.
    ///
    /// # Specifications
    ///
    /// See also [`Deactivated`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-GlobalShortcuts.Deactivated).
    #[doc(alias = "Deactivated")]
    pub async fn receive_deactivated(&self) -> Result<impl Stream<Item = Deactivated>, Error> {
        self.0.signal("Deactivated").await
    }

    /// Signal emitted when information associated with some of the shortcuts
    /// has changed.
    ///
    /// # Specifications
    ///
    /// See also [`ShortcutsChanged`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-GlobalShortcuts.ShortcutsChanged).
    #[doc(alias = "ShortcutsChanged")]
    pub async fn receive_shortcuts_changed(
        &self,
    ) -> Result<impl Stream<Item = ShortcutsChanged>, Error> {
        self.0.signal("ShortcutsChanged").await
    }
}
