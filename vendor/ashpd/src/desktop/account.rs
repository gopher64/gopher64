//! Access to the current logged user information such as the id, name
//! or their avatar uri.
//!
//! Wrapper of the DBus interface: [`org.freedesktop.portal.Account`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Account).
//!
//! ### Examples
//!
//! ```rust, no_run
//! use ashpd::desktop::account::UserInformation;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let response = UserInformation::request()
//!         .reason("App would like to access user information")
//!         .send()
//!         .await?
//!         .response()?;
//!
//!     println!("Name: {}", response.name());
//!     println!("ID: {}", response.id());
//!
//!     Ok(())
//! }
//! ```

use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

use super::HandleToken;
use crate::{desktop::request::Request, proxy::Proxy, Error, WindowIdentifier};

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct UserInformationOptions {
    handle_token: HandleToken,
    reason: Option<String>,
}

#[derive(Debug, DeserializeDict, SerializeDict, Type)]
/// The response of a [`UserInformationRequest`] request.
#[zvariant(signature = "dict")]
pub struct UserInformation {
    id: String,
    name: String,
    image: url::Url,
}

impl UserInformation {
    /// User identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// User name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// User image uri.
    pub fn image(&self) -> &url::Url {
        &self.image
    }

    /// Creates a new builder-pattern struct instance to construct
    /// [`UserInformation`].
    ///
    /// This method returns an instance of [`UserInformationRequest`].
    pub fn request() -> UserInformationRequest {
        UserInformationRequest::default()
    }
}

struct AccountProxy<'a>(Proxy<'a>);

impl<'a> AccountProxy<'a> {
    pub async fn new() -> Result<AccountProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Account").await?;
        Ok(Self(proxy))
    }

    pub async fn user_information(
        &self,
        identifier: &WindowIdentifier,
        options: UserInformationOptions,
    ) -> Result<Request<UserInformation>, Error> {
        self.0
            .request(
                &options.handle_token,
                "GetUserInformation",
                (&identifier, &options),
            )
            .await
    }
}

#[doc(alias = "xdp_portal_get_user_information")]
#[doc(alias = "org.freedesktop.portal.Account")]
#[derive(Debug, Default)]
/// A [builder-pattern] type to construct [`UserInformation`].
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
pub struct UserInformationRequest {
    options: UserInformationOptions,
    identifier: WindowIdentifier,
}

impl UserInformationRequest {
    #[must_use]
    /// Sets a user-visible reason for the request.
    pub fn reason<'a>(mut self, reason: impl Into<Option<&'a str>>) -> Self {
        self.options.reason = reason.into().map(ToOwned::to_owned);
        self
    }

    #[must_use]
    /// Sets a window identifier.
    pub fn identifier(mut self, identifier: impl Into<Option<WindowIdentifier>>) -> Self {
        self.identifier = identifier.into().unwrap_or_default();
        self
    }

    /// Build the [`UserInformation`].
    pub async fn send(self) -> Result<Request<UserInformation>, Error> {
        let proxy = AccountProxy::new().await?;
        proxy.user_information(&self.identifier, self.options).await
    }
}
