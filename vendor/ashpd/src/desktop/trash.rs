//! Move a file to the trash.
//!
//! # Examples
//!
//!
//! ```rust,no_run
//! use std::fs::File;
//!
//! use ashpd::desktop::trash;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let file = File::open("/home/bilelmoussaoui/adwaita-night.jpg").unwrap();
//!     trash::trash_file(&file).await?;
//!     Ok(())
//! }
//! ```
//!
//! Or by using the Proxy directly
//!
//! ```rust,no_run
//! use std::fs::File;
//!
//! use ashpd::desktop::trash::TrashProxy;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let file = File::open("/home/bilelmoussaoui/Downloads/adwaita-night.jpg").unwrap();
//!     let proxy = TrashProxy::new().await?;
//!     proxy.trash_file(&file).await?;
//!     Ok(())
//! }
//! ```

use std::os::unix::io::AsRawFd;

use serde_repr::{Deserialize_repr, Serialize_repr};
use zbus::zvariant::{Fd, Type};

use crate::{error::PortalError, proxy::Proxy, Error};

#[derive(Debug, Deserialize_repr, Serialize_repr, PartialEq, Type)]
#[repr(u32)]
enum TrashStatus {
    Failed = 0,
    Succeeded = 1,
}

/// The interface lets sandboxed applications send files to the trashcan.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Trash`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Trash).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Trash")]
pub struct TrashProxy<'a>(Proxy<'a>);

impl<'a> TrashProxy<'a> {
    /// Create a new instance of [`TrashProxy`].
    pub async fn new() -> Result<TrashProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Trash").await?;
        Ok(Self(proxy))
    }

    /// Sends a file to the trashcan.
    /// Applications are allowed to trash a file if they can open it in
    /// read/write mode.
    ///
    /// # Arguments
    ///
    /// * `fd` - The file descriptor.
    ///
    /// # Specifications
    ///
    /// See also [`TrashFile`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Trash.TrashFile).
    #[doc(alias = "TrashFile")]
    #[doc(alias = "xdp_portal_trash_file")]
    pub async fn trash_file(&self, fd: &impl AsRawFd) -> Result<(), Error> {
        let status = self
            .0
            .call("TrashFile", &(Fd::from(fd.as_raw_fd())))
            .await?;
        match status {
            TrashStatus::Failed => Err(Error::Portal(PortalError::Failed)),
            TrashStatus::Succeeded => Ok(()),
        }
    }
}

#[doc(alias = "xdp_portal_trash_file")]
/// A handy wrapper around [`TrashProxy::trash_file`].
pub async fn trash_file(fd: &impl AsRawFd) -> Result<(), Error> {
    let proxy = TrashProxy::new().await?;
    proxy.trash_file(fd).await
}

#[cfg(test)]
mod test {
    use super::TrashStatus;

    #[test]
    fn status_serde() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Test {
            status: TrashStatus,
        }

        let status = Test {
            status: TrashStatus::Failed,
        };

        let x = serde_json::to_string(&status).unwrap();
        let y: Test = serde_json::from_str(&x).unwrap();
        assert_eq!(y.status, TrashStatus::Failed);
    }
}
