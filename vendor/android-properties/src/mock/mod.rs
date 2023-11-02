use crate::AndroidProperty;
use std::os::raw::c_void;

/// Mock implementation for getprop
///
/// Always returns None
pub fn plat_getprop(_: &str, _: *const c_void) -> Option<String> {
    None
}

/// Mock implementation for setprop
///
/// Always returns Err
pub fn plat_setprop(_name: &str, _value: &str) -> Result<(), String> {
    Err("Failed to set android property (OS not supported)".to_string())
}

/// Mock implementation for prop_values
///
/// Always returns iterator to empty vector
pub fn plat_prop_values() -> impl Iterator<Item = AndroidProperty> {
    let properties: Box<Vec<AndroidProperty>> = Box::new(Vec::new());
    properties.into_iter()
}

/// Mock implementation to find property_info pointer
///
/// Always returns nullptr
pub fn plat_get_property_info(_name: &str) -> *const c_void {
    std::ptr::null()
}
