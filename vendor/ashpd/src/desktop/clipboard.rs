//! Interact with the clipboard.
//!
//! The portal is mostly meant to be used along with
//! [`RemoteDesktop`](crate::desktop::remote_desktop::RemoteDesktop)

use std::collections::HashMap;

use futures_util::{Stream, StreamExt};
use zbus::zvariant::{DeserializeDict, OwnedFd, OwnedObjectPath, SerializeDict, Type, Value};

use super::Session;
use crate::{proxy::Proxy, Result};

#[derive(Debug, Type, SerializeDict)]
#[zvariant(signature = "dict")]
struct SetSelectionOptions<'a> {
    mime_types: &'a [&'a str],
}

#[derive(Debug, Type, DeserializeDict)]
#[zvariant(signature = "dict")]
/// The details of a new clipboard selection.
pub struct SelectionOwnerChanged {
    mime_types: Option<Vec<String>>,
    session_is_owner: Option<bool>,
}

impl SelectionOwnerChanged {
    /// Whether the session is the owner of the clipboard selection or not.
    pub fn session_is_owner(&self) -> Option<bool> {
        self.session_is_owner
    }

    /// A list of mime types the new clipboard has content for.
    pub fn mime_types(&self) -> Vec<String> {
        self.mime_types.clone().unwrap_or_default()
    }
}

#[doc(alias = "org.freedesktop.portal.Clipboard")]
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Clipboard`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Clipboard).
pub struct Clipboard<'a>(Proxy<'a>);

impl<'a> Clipboard<'a> {
    /// Create a new instance of [`Clipboard`].
    pub async fn new() -> Result<Clipboard<'a>> {
        Ok(Self(
            Proxy::new_desktop("org.freedesktop.portal.Clipboard").await?,
        ))
    }

    /// # Specifications
    ///
    /// See also [`RequestClipboard`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Clipboard.RequestClipboard).
    #[doc(alias = "RequestClipboard")]
    pub async fn request(&self, session: &Session<'_>) -> Result<()> {
        let options: HashMap<&str, Value<'_>> = HashMap::default();
        self.0
            .call_method("RequestClipboard", &(session, options))
            .await?;
        Ok(())
    }

    /// # Specifications
    ///
    /// See also [`RequestClipboard`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Clipboard.RequestClipboard).
    #[doc(alias = "SetSelection")]
    pub async fn set_selection(&self, session: &Session<'_>, mime_types: &[&str]) -> Result<()> {
        let options = SetSelectionOptions { mime_types };
        self.0.call("SetSelection", &(session, options)).await?;

        Ok(())
    }

    /// # Specifications
    ///
    /// See also [`SelectionWrite`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Clipboard.SelectionWrite).
    #[doc(alias = "SelectionWrite")]
    pub async fn selection_write(&self, session: &Session<'_>, serial: u32) -> Result<OwnedFd> {
        let fd = self
            .0
            .call::<OwnedFd>("SelectionWrite", &(session, serial))
            .await?;
        Ok(fd)
    }

    /// # Specifications
    ///
    /// See also [`SelectionWriteDone`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Clipboard.SelectionWriteDone).
    #[doc(alias = "SelectionWriteDone")]
    pub async fn selection_write_done(
        &self,
        session: &Session<'_>,
        serial: u32,
        success: bool,
    ) -> Result<()> {
        self.0
            .call("SelectionWriteDone", &(session, serial, success))
            .await
    }

    /// # Specifications
    ///
    /// See also [`SelectionRead`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Clipboard.SelectionRead).
    #[doc(alias = "SelectionRead")]
    pub async fn selection_read(&self, session: &Session<'_>, mime_type: &str) -> Result<OwnedFd> {
        let fd = self
            .0
            .call::<OwnedFd>("SelectionRead", &(session, mime_type))
            .await?;
        Ok(fd)
    }

    /// Notifies the session that the clipboard selection has changed.
    /// # Specifications
    ///
    /// See also [`SelectionOwnerChanged`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-Clipboard.SelectionOwnerChanged).
    #[doc(alias = "SelectionOwnerChanged")]
    pub async fn receive_selection_owner_changed(
        &self,
    ) -> Result<impl Stream<Item = (Session, SelectionOwnerChanged)>> {
        Ok(self
            .0
            .signal::<(OwnedObjectPath, SelectionOwnerChanged)>("SelectionOwnerChanged")
            .await?
            .filter_map(|(p, o)| async move { Session::new(p).await.map(|s| (s, o)).ok() }))
    }

    /// # Specifications
    ///
    /// See also [`SelectionTransfer`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-Clipboard.SelectionTransfer).
    #[doc(alias = "SelectionTransfer")]
    pub async fn receive_selection_transfer(
        &self,
    ) -> Result<impl Stream<Item = (Session, String, u32)>> {
        Ok(self
            .0
            .signal::<(OwnedObjectPath, String, u32)>("SelectionTransfer")
            .await?
            .filter_map(|(p, mime_type, serial)| async move {
                Session::new(p)
                    .await
                    .map(|session| (session, mime_type, serial))
                    .ok()
            }))
    }
}
