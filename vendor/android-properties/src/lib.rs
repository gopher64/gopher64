//! android-properties is a rust wrapper for bionic property-related syscalls

#![deny(missing_docs, missing_debug_implementations, unused)]

use std::{fmt, os::raw::c_void};

#[cfg(target_os = "android")]
use crate::android::*;
#[cfg(not(target_os = "android"))]
use crate::mock::*;

#[cfg(target_os = "android")]
/// The implementation of property API for Android bionic-based systems
pub mod android;

#[cfg(not(target_os = "android"))]
/// The mock implementation of property API for non-Android based systems
pub mod mock;

/// A struct representing android properties
///
/// This struct consists from a name-value pair
#[derive(Debug)]
pub struct AndroidProperty {
    /// Property name
    name: String,
    /// Property info pointer
    property_info: *const c_void,
}

impl AndroidProperty {
    /// Initializes and returns struct representing android properties
    pub fn new(name: &str) -> Self {
        AndroidProperty {
            name: name.to_string(),
            property_info: std::ptr::null(),
        }
    }

    /// Return property name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return property value
    pub fn value(&mut self) -> Option<String> {
        if self.property_info.is_null() {
            self.property_info = plat_get_property_info(&self.name);
        }
        plat_getprop(&self.name, self.property_info)
    }

    /// Set property value
    pub fn set_value(&self, value: &str) -> Result<(), String> {
        plat_setprop(&self.name, value)
    }
}

impl fmt::Display for AndroidProperty {
    // Output in format [<name>]: [<value>]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut property_info = self.property_info;
        if property_info.is_null() {
            property_info = plat_get_property_info(&self.name);
        }
        write!(
            f,
            "[{}]: [{}]",
            self.name,
            plat_getprop(&self.name, property_info).unwrap_or_else(|| "".into())
        )
    }
}

/// Returns the property value if it exists
pub fn getprop(name: &str) -> AndroidProperty {
    AndroidProperty::new(name)
}

/// Sets the property value if it exists or creates new one with specified value
pub fn setprop(name: &str, value: &str) -> Result<(), String> {
    AndroidProperty::new(name).set_value(value)
}

/// Returns an iterator to vector, which contains all properties present in a system
pub fn prop_values() -> impl Iterator<Item = AndroidProperty> {
    #[cfg(target_os = "android")]
    return crate::android::plat_prop_values();

    #[cfg(not(target_os = "android"))]
    return crate::mock::plat_prop_values();
}
