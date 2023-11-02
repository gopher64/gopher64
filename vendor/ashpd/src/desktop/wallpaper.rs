//! Set a wallpaper on lockscreen, background or both.
//!
//! Wrapper of the DBus interface: [`org.freedesktop.portal.Wallpaper`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Wallpaper).
//!
//! # Examples
//!
//! ## Sets a wallpaper from a file:
//!
//! ```rust,no_run
//! use std::fs::File;
//!
//! use ashpd::desktop::wallpaper::{SetOn, WallpaperRequest};
//!
//! async fn run() -> ashpd::Result<()> {
//!     let file = File::open("/home/bilelmoussaoui/adwaita-day.jpg").unwrap();
//!     WallpaperRequest::default()
//!         .set_on(SetOn::Both)
//!         .show_preview(true)
//!         .build_file(&file)
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Sets a wallpaper from a URI:
//!
//! ```rust,no_run
//! use ashpd::desktop::wallpaper::{SetOn, WallpaperRequest};
//!
//! async fn run() -> ashpd::Result<()> {
//!     let uri =
//!         url::Url::parse("file:///home/bilelmoussaoui/Downloads/adwaita-night.jpg").unwrap();
//!     WallpaperRequest::default()
//!         .set_on(SetOn::Both)
//!         .show_preview(true)
//!         .build_uri(&uri)
//!         .await?;
//!     Ok(())
//! }
//! ```

use std::{fmt, os::unix::prelude::AsRawFd, str::FromStr};

use serde::{self, Deserialize, Serialize};
use zbus::zvariant::{Fd, SerializeDict, Type};

use super::Request;
use crate::{desktop::HandleToken, proxy::Proxy, Error, WindowIdentifier};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Type)]
#[zvariant(signature = "s")]
#[serde(rename_all = "lowercase")]
/// Where to set the wallpaper on.
pub enum SetOn {
    /// Set the wallpaper only on the lock-screen.
    Lockscreen,
    /// Set the wallpaper only on the background.
    Background,
    /// Set the wallpaper on both lock-screen and background.
    Both,
}

impl fmt::Display for SetOn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lockscreen => write!(f, "Lockscreen"),
            Self::Background => write!(f, "Background"),
            Self::Both => write!(f, "Both"),
        }
    }
}

impl AsRef<str> for SetOn {
    fn as_ref(&self) -> &str {
        match self {
            Self::Lockscreen => "Lockscreen",
            Self::Background => "Background",
            Self::Both => "Both",
        }
    }
}

impl From<SetOn> for &'static str {
    fn from(s: SetOn) -> Self {
        match s {
            SetOn::Lockscreen => "Lockscreen",
            SetOn::Background => "Background",
            SetOn::Both => "Both",
        }
    }
}

impl FromStr for SetOn {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Lockscreen" => Ok(SetOn::Lockscreen),
            "Background" => Ok(SetOn::Background),
            "Both" => Ok(SetOn::Both),
            _ => Err(Error::ParseError("Failed to parse SetOn, invalid value")),
        }
    }
}

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct WallpaperOptions {
    handle_token: HandleToken,
    #[zvariant(rename = "show-preview")]
    show_preview: Option<bool>,
    #[zvariant(rename = "set-on")]
    set_on: Option<SetOn>,
}

struct WallpaperProxy<'a>(Proxy<'a>);

impl<'a> WallpaperProxy<'a> {
    pub async fn new() -> Result<WallpaperProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Wallpaper").await?;
        Ok(Self(proxy))
    }

    pub async fn set_wallpaper_file(
        &self,
        identifier: &WindowIdentifier,
        file: &impl AsRawFd,
        options: WallpaperOptions,
    ) -> Result<Request<()>, Error> {
        self.0
            .empty_request(
                &options.handle_token,
                "SetWallpaperFile",
                &(&identifier, Fd::from(file.as_raw_fd()), &options),
            )
            .await
    }

    pub async fn set_wallpaper_uri(
        &self,
        identifier: &WindowIdentifier,
        uri: &url::Url,
        options: WallpaperOptions,
    ) -> Result<Request<()>, Error> {
        self.0
            .empty_request(
                &options.handle_token,
                "SetWallpaperURI",
                &(&identifier, uri, &options),
            )
            .await
    }
}

#[derive(Debug, Default)]
#[doc(alias = "xdp_portal_set_wallpaper")]
#[doc(alias = "org.freedesktop.portal.Wallpaper")]
/// A [builder-pattern] type to set the wallpaper.
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
pub struct WallpaperRequest {
    identifier: WindowIdentifier,
    options: WallpaperOptions,
}

impl WallpaperRequest {
    #[must_use]
    /// Sets a window identifier.
    pub fn identifier(mut self, identifier: impl Into<Option<WindowIdentifier>>) -> Self {
        self.identifier = identifier.into().unwrap_or_default();
        self
    }

    /// Whether to show a preview of the picture.
    /// **Note** the portal may decide to show a preview even if this option is
    /// not set.
    #[must_use]
    pub fn show_preview(mut self, show_preview: impl Into<Option<bool>>) -> Self {
        self.options.show_preview = show_preview.into();
        self
    }

    /// Sets where to set the wallpaper on.
    #[must_use]
    pub fn set_on(mut self, set_on: impl Into<Option<SetOn>>) -> Self {
        self.options.set_on = set_on.into();
        self
    }

    /// Build using a URI.
    pub async fn build_uri(self, uri: &url::Url) -> Result<Request<()>, Error> {
        let proxy = WallpaperProxy::new().await?;
        proxy
            .set_wallpaper_uri(&self.identifier, uri, self.options)
            .await
    }

    /// Build using a file.
    pub async fn build_file(self, file: &impl AsRawFd) -> Result<Request<()>, Error> {
        let proxy = WallpaperProxy::new().await?;
        proxy
            .set_wallpaper_file(&self.identifier, file, self.options)
            .await
    }
}
#[cfg(test)]
mod tests {
    use super::SetOn;

    #[test]
    fn serialize_deserialize() {
        let set_on = SetOn::Both;
        let string = serde_json::to_string(&set_on).unwrap();
        assert_eq!(string, "\"both\"");

        let decoded = serde_json::from_str(&string).unwrap();
        assert_eq!(set_on, decoded);
    }
}
