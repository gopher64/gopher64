//! Compose an email.
//!
//! Wrapper of the DBus interface: [`org.freedesktop.portal.Email`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Email).
//!
//! # Examples
//!
//! Compose an email
//!
//! ```rust,no_run
//! use std::fs::File;
//!
//! use ashpd::desktop::email::EmailRequest;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let file = File::open("/home/bilelmoussaoui/Downloads/adwaita-night.jpg").unwrap();
//!     EmailRequest::default()
//!         .address("test@gmail.com")
//!         .subject("email subject")
//!         .body("the pre-filled email body")
//!         .attach(&file)
//!         .send()
//!         .await;
//!     Ok(())
//! }
//! ```

use std::os::unix::prelude::AsRawFd;

use serde::Serialize;
use zbus::zvariant::{Fd, SerializeDict, Type};

use super::{HandleToken, Request};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct EmailOptions {
    handle_token: HandleToken,
    address: Option<String>,
    addresses: Option<Vec<String>>,
    cc: Option<Vec<String>>,
    bcc: Option<Vec<String>>,
    subject: Option<String>,
    body: Option<String>,
    attachment_fds: Option<Vec<Fd>>,
}

#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Email")]
struct EmailProxy<'a>(Proxy<'a>);

impl<'a> EmailProxy<'a> {
    /// Create a new instance of [`EmailProxy`].
    pub async fn new() -> Result<EmailProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Email").await?;
        Ok(Self(proxy))
    }

    /// Presents a window that lets the user compose an email.
    ///
    /// **Note** the default email client for the host will need to support
    /// `mailto:` URIs following RFC 2368.
    ///
    /// # Arguments
    ///
    /// * `identifier` - Identifier for the application window.
    /// * `options` - An [`EmailOptions`].
    ///
    /// # Specifications
    ///
    /// See also [`ComposeEmail`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Email.ComposeEmail).
    #[doc(alias = "ComposeEmail")]
    pub async fn compose(
        &self,
        identifier: &WindowIdentifier,
        options: EmailOptions,
    ) -> Result<Request<()>, Error> {
        self.0
            .empty_request(
                &options.handle_token,
                "ComposeEmail",
                &(&identifier, &options),
            )
            .await
    }
}

#[derive(Debug, Default)]
#[doc(alias = "xdp_portal_compose_email")]
/// A [builder-pattern] type to compose an email.
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
pub struct EmailRequest {
    identifier: WindowIdentifier,
    options: EmailOptions,
}

impl EmailRequest {
    /// Sets a window identifier.
    #[must_use]
    pub fn identifier(mut self, identifier: impl Into<Option<WindowIdentifier>>) -> Self {
        self.identifier = identifier.into().unwrap_or_default();
        self
    }

    /// Sets the email address to send the email to.
    #[must_use]
    pub fn address<'a>(mut self, address: impl Into<Option<&'a str>>) -> Self {
        self.options.address = address.into().map(ToOwned::to_owned);
        self
    }

    /// Sets a list of email addresses to send the email to.
    #[must_use]
    pub fn addresses<P: IntoIterator<Item = I>, I: AsRef<str> + Type + Serialize>(
        mut self,
        addresses: impl Into<Option<P>>,
    ) -> Self {
        self.options.addresses = addresses
            .into()
            .map(|a| a.into_iter().map(|s| s.as_ref().to_owned()).collect());
        self
    }

    /// Sets a list of email addresses to BCC.
    #[must_use]
    pub fn bcc<P: IntoIterator<Item = I>, I: AsRef<str> + Type + Serialize>(
        mut self,
        bcc: impl Into<Option<P>>,
    ) -> Self {
        self.options.bcc = bcc
            .into()
            .map(|a| a.into_iter().map(|s| s.as_ref().to_owned()).collect());
        self
    }

    /// Sets a list of email addresses to CC.
    #[must_use]
    pub fn cc<P: IntoIterator<Item = I>, I: AsRef<str> + Type + Serialize>(
        mut self,
        cc: impl Into<Option<P>>,
    ) -> Self {
        self.options.cc = cc
            .into()
            .map(|a| a.into_iter().map(|s| s.as_ref().to_owned()).collect());
        self
    }

    /// Sets the email subject.
    #[must_use]
    pub fn subject<'a>(mut self, subject: impl Into<Option<&'a str>>) -> Self {
        self.options.subject = subject.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the email body.
    #[must_use]
    pub fn body<'a>(mut self, body: impl Into<Option<&'a str>>) -> Self {
        self.options.body = body.into().map(ToOwned::to_owned);
        self
    }

    /// Attaches a file to the email.
    #[must_use]
    pub fn attach(mut self, attachment: &impl AsRawFd) -> Self {
        self.add_attachment(attachment);
        self
    }

    /// A different variant of [`Self::attach`].
    pub fn add_attachment(&mut self, attachment: &impl AsRawFd) {
        let attachment = Fd::from(attachment.as_raw_fd());
        match self.options.attachment_fds {
            Some(ref mut attachments) => attachments.push(attachment),
            None => {
                self.options.attachment_fds.replace(vec![attachment]);
            }
        };
    }

    /// Send the request.
    pub async fn send(self) -> Result<Request<()>, Error> {
        let proxy = EmailProxy::new().await?;
        proxy.compose(&self.identifier, self.options).await
    }
}
