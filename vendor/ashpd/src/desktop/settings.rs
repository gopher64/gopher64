//! ```rust,no_run
//! use ashpd::desktop::settings::Settings;
//! use futures_util::StreamExt;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = Settings::new().await?;
//!
//!     let clock_format = proxy
//!         .read::<String>("org.gnome.desktop.interface", "clock-format")
//!         .await?;
//!     println!("{:#?}", clock_format);
//!
//!     let settings = proxy.read_all(&["org.gnome.desktop.interface"]).await?;
//!     println!("{:#?}", settings);
//!
//!     let setting = proxy
//!         .receive_setting_changed()
//!         .await?
//!         .next()
//!         .await
//!         .expect("Stream exhausted");
//!     println!("{}", setting.namespace());
//!     println!("{}", setting.key());
//!     println!("{:#?}", setting.value());
//!
//!     Ok(())
//! }
//! ```

use std::{collections::HashMap, convert::TryFrom, fmt::Debug, future::ready};

use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::{proxy::Proxy, Error};

/// A HashMap of the <key, value> settings found on a specific namespace.
pub type Namespace = HashMap<String, OwnedValue>;

#[derive(Clone, Deserialize, Type)]
/// A specific `namespace.key = value` setting.
pub struct Setting(String, String, OwnedValue);

impl Setting {
    /// The setting namespace.
    pub fn namespace(&self) -> &str {
        &self.0
    }

    /// The setting key.
    pub fn key(&self) -> &str {
        &self.1
    }

    /// The setting value.
    pub fn value(&self) -> &OwnedValue {
        &self.2
    }
}

impl std::fmt::Debug for Setting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Setting")
            .field("namespace", &self.namespace())
            .field("key", &self.key())
            .field("value", self.value())
            .finish()
    }
}

/// The system's preferred color scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorScheme {
    /// No preference
    NoPreference,
    /// Prefers dark appearance
    PreferDark,
    /// Prefers light appearance
    PreferLight,
}

impl TryFrom<OwnedValue> for ColorScheme {
    type Error = Error;

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        TryFrom::<Value>::try_from(value.into())
    }
}

impl TryFrom<Value<'_>> for ColorScheme {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(match u32::try_from(value)? {
            1 => ColorScheme::PreferDark,
            2 => ColorScheme::PreferLight,
            _ => ColorScheme::NoPreference,
        })
    }
}

impl TryFrom<Setting> for ColorScheme {
    type Error = Error;

    fn try_from(value: Setting) -> Result<Self, Self::Error> {
        Self::try_from(value.2)
    }
}

const APPEARANCE_NAMESPACE: &str = "org.freedesktop.appearance";
const COLOR_SCHEME_KEY: &str = "color-scheme";

/// The interface provides read-only access to a small number of host settings
/// required for toolkits similar to XSettings. It is not for general purpose
/// settings.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Settings`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Settings).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Settings")]
pub struct Settings<'a>(Proxy<'a>);

impl<'a> Settings<'a> {
    /// Create a new instance of [`Settings`].
    pub async fn new() -> Result<Settings<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Settings").await?;
        Ok(Self(proxy))
    }

    /// Reads a single value. Returns an error on any unknown namespace or key.
    ///
    /// # Arguments
    ///
    /// * `namespaces` - List of namespaces to filter results by.
    ///
    /// If `namespaces` is an empty array or contains an empty string it matches
    /// all. Globing is supported but only for trailing sections, e.g.
    /// `org.example.*`.
    ///
    /// # Returns
    ///
    /// A `HashMap` of namespaces to its keys and values.
    ///
    /// # Specifications
    ///
    /// See also [`ReadAll`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Settings.ReadAll).
    #[doc(alias = "ReadAll")]
    pub async fn read_all(
        &self,
        namespaces: &[impl AsRef<str> + Type + Serialize + Debug],
    ) -> Result<HashMap<String, Namespace>, Error> {
        self.0.call("ReadAll", &(namespaces)).await
    }

    /// Reads a single value. Returns an error on any unknown namespace or key.
    ///
    /// # Arguments
    ///
    /// * `namespace` - Namespace to look up key in.
    /// * `key` - The key to get.
    ///
    /// # Returns
    ///
    /// The value for `key` as a `zvariant::OwnedValue`.
    ///
    /// # Specifications
    ///
    /// See also [`Read`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Settings.Read).
    #[doc(alias = "Read")]
    pub async fn read<T>(&self, namespace: &str, key: &str) -> Result<T, Error>
    where
        T: TryFrom<OwnedValue>,
        Error: From<<T as TryFrom<OwnedValue>>::Error>,
    {
        let value = self.0.call::<OwnedValue>("Read", &(namespace, key)).await?;
        if let Some(v) = value.downcast_ref::<Value>() {
            T::try_from(v.to_owned()).map_err(From::from)
        } else {
            T::try_from(value).map_err(From::from)
        }
    }

    /// Retrieves the system's preferred color scheme
    pub async fn color_scheme(&self) -> Result<ColorScheme, Error> {
        self.read::<ColorScheme>(APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY)
            .await
    }

    /// Listen to changes of the system's preferred color scheme
    pub async fn receive_color_scheme_changed(
        &self,
    ) -> Result<impl Stream<Item = ColorScheme>, Error> {
        Ok(self
            .receive_setting_changed_with_args(APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY)
            .await?
            .filter_map(|x| ready(ColorScheme::try_from(x).ok())))
    }

    /// Signal emitted when a setting changes.
    ///
    /// # Specifications
    ///
    /// See also [`SettingChanged`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-Settings.SettingChanged).
    #[doc(alias = "SettingChanged")]
    pub async fn receive_setting_changed(&self) -> Result<impl Stream<Item = Setting>, Error> {
        self.0.signal("SettingChanged").await
    }

    /// Similar to (receive_setting_changed)[Self::receive_setting_changed]
    /// but allows you to filter specific settings.
    ///
    /// # Example
    /// ```rust,no_run
    /// use ashpd::desktop::settings::{ColorScheme, Settings};
    /// use futures_util::StreamExt;
    ///
    /// # async fn run() -> ashpd::Result<()> {
    /// let settings = Settings::new().await?;
    /// while let Some(setting) = settings
    ///     .receive_setting_changed_with_args("org.freedesktop.appearance", "color-scheme")
    ///     .await?
    ///     .next()
    ///     .await
    /// {
    ///     assert_eq!(setting.namespace(), "org.freedesktop.appearance");
    ///     assert_eq!(setting.key(), "color-scheme");
    ///     assert!(ColorScheme::try_from(setting.value().to_owned()).is_ok());
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    pub async fn receive_setting_changed_with_args(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<impl Stream<Item = Setting>, Error> {
        self.0
            .signal_with_args("SettingChanged", &[(0, namespace), (1, key)])
            .await
    }
}
