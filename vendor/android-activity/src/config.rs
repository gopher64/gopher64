use core::fmt;
use std::sync::{Arc, RwLock};

use ndk::configuration::{
    Configuration, Keyboard, KeysHidden, LayoutDir, NavHidden, Navigation, Orientation, ScreenLong,
    ScreenSize, Touchscreen, UiModeNight, UiModeType,
};

/// A (cheaply clonable) reference to this application's [`ndk::configuration::Configuration`]
///
/// This provides a thread-safe way to access the latest configuration state for
/// an application without deeply copying the large [`ndk::configuration::Configuration`] struct.
///
/// If the application is notified of configuration changes then those changes
/// will become visible via pre-existing configuration references.
#[derive(Clone)]
pub struct ConfigurationRef {
    config: Arc<RwLock<Configuration>>,
}
impl PartialEq for ConfigurationRef {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.config, &other.config) {
            true
        } else {
            let other_guard = other.config.read().unwrap();
            self.config.read().unwrap().eq(&*other_guard)
        }
    }
}
impl Eq for ConfigurationRef {}
unsafe impl Send for ConfigurationRef {}
unsafe impl Sync for ConfigurationRef {}

impl fmt::Debug for ConfigurationRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.config.read().unwrap().fmt(f)
    }
}

impl ConfigurationRef {
    pub(crate) fn new(config: Configuration) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub(crate) fn replace(&self, src: Configuration) {
        self.config.write().unwrap().copy(&src);
    }

    // Returns a deep copy of the full application configuration
    pub fn copy(&self) -> Configuration {
        let mut dest = Configuration::new();
        dest.copy(&self.config.read().unwrap());
        dest
    }
    /// Returns the country code, as a [`String`] of two characters, if set
    pub fn country(&self) -> Option<String> {
        self.config.read().unwrap().country()
    }

    /// Returns the screen density in dpi.
    ///
    /// On some devices it can return values outside of the density enum.
    pub fn density(&self) -> Option<u32> {
        self.config.read().unwrap().density()
    }

    /// Returns the keyboard type.
    pub fn keyboard(&self) -> Keyboard {
        self.config.read().unwrap().keyboard()
    }

    /// Returns keyboard visibility/availability.
    pub fn keys_hidden(&self) -> KeysHidden {
        self.config.read().unwrap().keys_hidden()
    }

    /// Returns the language, as a `String` of two characters, if a language is set
    pub fn language(&self) -> Option<String> {
        self.config.read().unwrap().language()
    }

    /// Returns the layout direction
    pub fn layout_direction(&self) -> LayoutDir {
        self.config.read().unwrap().layout_direction()
    }

    /// Returns the mobile country code.
    pub fn mcc(&self) -> i32 {
        self.config.read().unwrap().mcc()
    }

    /// Returns the mobile network code, if one is defined
    pub fn mnc(&self) -> Option<i32> {
        self.config.read().unwrap().mnc()
    }

    pub fn nav_hidden(&self) -> NavHidden {
        self.config.read().unwrap().nav_hidden()
    }

    pub fn navigation(&self) -> Navigation {
        self.config.read().unwrap().navigation()
    }

    pub fn orientation(&self) -> Orientation {
        self.config.read().unwrap().orientation()
    }

    pub fn screen_height_dp(&self) -> Option<i32> {
        self.config.read().unwrap().screen_height_dp()
    }

    pub fn screen_width_dp(&self) -> Option<i32> {
        self.config.read().unwrap().screen_width_dp()
    }

    pub fn screen_long(&self) -> ScreenLong {
        self.config.read().unwrap().screen_long()
    }

    #[cfg(feature = "api-level-30")]
    pub fn screen_round(&self) -> ScreenRound {
        self.config.read().unwrap().screen_round()
    }

    pub fn screen_size(&self) -> ScreenSize {
        self.config.read().unwrap().screen_size()
    }

    pub fn sdk_version(&self) -> i32 {
        self.config.read().unwrap().sdk_version()
    }

    pub fn smallest_screen_width_dp(&self) -> Option<i32> {
        self.config.read().unwrap().smallest_screen_width_dp()
    }

    pub fn touchscreen(&self) -> Touchscreen {
        self.config.read().unwrap().touchscreen()
    }

    pub fn ui_mode_night(&self) -> UiModeNight {
        self.config.read().unwrap().ui_mode_night()
    }

    pub fn ui_mode_type(&self) -> UiModeType {
        self.config.read().unwrap().ui_mode_type()
    }
}
