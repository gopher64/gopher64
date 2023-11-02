//! Open a URI or a directory.
//!
//! Wrapper of the DBus interface: [`org.freedesktop.portal.OpenURI`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.OpenURI).
//!
//! # Examples
//!
//! ## Open a file
//!
//! ```rust,no_run
//! use std::fs::File;
//!
//! use ashpd::desktop::open_uri::OpenFileRequest;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let file = File::open("/home/bilelmoussaoui/adwaita-day.jpg").unwrap();
//!     OpenFileRequest::default()
//!         .ask(true)
//!         .send_file(&file)
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Open a file from a URI
//!
//!
//! ```rust,no_run
//! use ashpd::desktop::open_uri::OpenFileRequest;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let uri =
//!         url::Url::parse("file:///home/bilelmoussaoui/Downloads/adwaita-night.jpg").unwrap();
//!     OpenFileRequest::default().ask(true).send_uri(&uri).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Open a directory
//!
//! ```rust,no_run
//! use std::fs::File;
//!
//! use ashpd::desktop::open_uri::OpenDirectoryRequest;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let directory = File::open("/home/bilelmoussaoui/Downloads").unwrap();
//!     OpenDirectoryRequest::default().send(&directory).await?;
//!     Ok(())
//! }
//! ```

use std::os::unix::prelude::AsRawFd;

use url::Url;
use zbus::zvariant::{Fd, SerializeDict, Type};

use super::{HandleToken, Request};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct OpenDirOptions {
    handle_token: HandleToken,
    activation_token: Option<String>,
}

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct OpenFileOptions {
    handle_token: HandleToken,
    writeable: Option<bool>,
    ask: Option<bool>,
    activation_token: Option<String>,
}

#[derive(Debug)]
struct OpenURIProxy<'a>(Proxy<'a>);

impl<'a> OpenURIProxy<'a> {
    pub async fn new() -> Result<OpenURIProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.OpenURI").await?;
        Ok(Self(proxy))
    }

    pub async fn open_directory(
        &self,
        identifier: &WindowIdentifier,
        directory: &impl AsRawFd,
        options: OpenDirOptions,
    ) -> Result<Request<()>, Error> {
        self.0
            .empty_request(
                &options.handle_token,
                "OpenDirectory",
                &(&identifier, Fd::from(directory.as_raw_fd()), &options),
            )
            .await
    }

    pub async fn open_file(
        &self,
        identifier: &WindowIdentifier,
        file: &impl AsRawFd,
        options: OpenFileOptions,
    ) -> Result<Request<()>, Error> {
        self.0
            .empty_request(
                &options.handle_token,
                "OpenFile",
                &(&identifier, Fd::from(file.as_raw_fd()), &options),
            )
            .await
    }

    pub async fn open_uri(
        &self,
        identifier: &WindowIdentifier,
        uri: &url::Url,
        options: OpenFileOptions,
    ) -> Result<Request<()>, Error> {
        self.0
            .empty_request(
                &options.handle_token,
                "OpenURI",
                &(&identifier, uri, &options),
            )
            .await
    }
}

#[derive(Debug, Default)]
#[doc(alias = "org.freedesktop.portal.OpenURI")]
#[doc(alias = "xdp_portal_open_uri")]
/// A [builder-pattern] type to open a file.
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
pub struct OpenFileRequest {
    identifier: WindowIdentifier,
    options: OpenFileOptions,
}

impl OpenFileRequest {
    #[must_use]
    /// Sets a window identifier.
    pub fn identifier(mut self, identifier: impl Into<Option<WindowIdentifier>>) -> Self {
        self.identifier = identifier.into().unwrap_or_default();
        self
    }

    #[must_use]
    /// Whether the file should be writeable or not.
    pub fn writeable(mut self, writeable: impl Into<Option<bool>>) -> Self {
        self.options.writeable = writeable.into();
        self
    }

    #[must_use]
    /// Whether to always ask the user which application to use or not.
    pub fn ask(mut self, ask: impl Into<Option<bool>>) -> Self {
        self.options.ask = ask.into();
        self
    }

    /// Send the request for a file.
    pub async fn send_file(self, file: &impl AsRawFd) -> Result<Request<()>, Error> {
        let proxy = OpenURIProxy::new().await?;
        proxy.open_file(&self.identifier, file, self.options).await
    }

    /// Send the request for a URI.
    pub async fn send_uri(self, uri: &Url) -> Result<Request<()>, Error> {
        let proxy = OpenURIProxy::new().await?;
        proxy.open_uri(&self.identifier, uri, self.options).await
    }
}

#[derive(Debug, Default)]
#[doc(alias = "xdp_portal_open_directory")]
#[doc(alias = "org.freedesktop.portal.OpenURI")]
/// A [builder-pattern] type to open a directory.
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
pub struct OpenDirectoryRequest {
    identifier: WindowIdentifier,
    options: OpenDirOptions,
}

impl OpenDirectoryRequest {
    #[must_use]
    /// Sets a window identifier.
    pub fn identifier(mut self, identifier: impl Into<Option<WindowIdentifier>>) -> Self {
        self.identifier = identifier.into().unwrap_or_default();
        self
    }

    /// Send the request.
    pub async fn send(self, directory: &impl AsRawFd) -> Result<Request<()>, Error> {
        let proxy = OpenURIProxy::new().await?;
        proxy
            .open_directory(&self.identifier, directory, self.options)
            .await
    }
}
