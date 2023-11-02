use crate::AndroidProperty;

use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int, c_void},
};

type Callback = unsafe fn(*mut ValuePair, *const c_char, *const c_char, u32);
type ForEachCallback = unsafe fn(*const c_void, *mut Vec<AndroidProperty>);

struct ValuePair {
    name: String,
    value: String,
}

unsafe fn property_callback(cookie: *mut ValuePair, name: *const c_char, value: *const c_char, _serial: u32) {
    let cname = CStr::from_ptr(name);
    let cvalue = CStr::from_ptr(value);
    (*cookie).name = cname.to_str().unwrap().to_string();
    (*cookie).value = cvalue.to_str().unwrap().to_string();
}

unsafe fn foreach_property_callback(pi: *const c_void, cookie: *mut Vec<AndroidProperty>) {
    let mut result = Box::new(ValuePair {
        name: String::new(),
        value: String::new(),
    });
    __system_property_read_callback(pi, property_callback, &mut *result);
    (*cookie).push(AndroidProperty {
        name: (*result).name,
        property_info: pi,
    });
}

extern "C" {
    fn __system_property_set(name: *const c_char, value: *const c_char) -> c_int;
    fn __system_property_find(name: *const c_char) -> *const c_void;
    fn __system_property_read_callback(pi: *const c_void, callback: Callback, cookie: *mut ValuePair);
    fn __system_property_foreach(callback: ForEachCallback, cookie: *mut Vec<AndroidProperty>) -> c_int;
}

#[cfg(feature = "bionic-deprecated")]
extern "C" {
    /* Deprecated. Use __system_property_read_callback instead. */
    fn __system_property_get(name: *const c_char, value: *mut c_char) -> c_int;
}

/// Set system property `name` to `value`, creating the system property if it doesn't already exist
pub fn plat_setprop(name: &str, value: &str) -> Result<(), String> {
    let cname = CString::new(name).unwrap();
    let cvalue = CString::new(value).unwrap();
    let ret = unsafe { __system_property_set(cname.as_ptr(), cvalue.as_ptr()) };
    if ret >= 0 {
        Ok(())
    } else {
        Err(format!("Failed to set Android property \"{}\" to \"{}\"", name, value))
    }
}

/// Retrieve a property with name `name`. Returns None if the operation fails.
#[cfg(not(feature = "bionic-deprecated"))]
pub fn plat_getprop(_: &str, property_info: *const c_void) -> Option<String> {
    let mut result = Box::new(ValuePair {
        name: String::new(),
        value: String::new(),
    });
    if !property_info.is_null() {
        unsafe { __system_property_read_callback(property_info, property_callback, &mut *result) };
    }
    Some((*result).value)
}

/// Retrieve a property with name `name`. Returns None if the operation fails.
#[cfg(feature = "bionic-deprecated")]
pub fn plat_getprop(name: &str, _: *const c_void) -> Option<String> {
    const PROPERTY_VALUE_MAX: usize = 92;
    let cname = CString::new(name).unwrap();
    let cvalue = CString::new(Vec::with_capacity(PROPERTY_VALUE_MAX)).unwrap();
    let raw = cvalue.into_raw();
    let ret = unsafe { __system_property_get(cname.as_ptr(), raw) };
    match ret {
        len if len > 0 => unsafe { Some(String::from_raw_parts(raw as *mut u8, len as usize, PROPERTY_VALUE_MAX)) },
        _ => None,
    }
}

/// Returns an iterator to vector, which contains all properties present in a system
pub fn plat_prop_values() -> impl Iterator<Item = AndroidProperty> {
    let mut properties: Box<Vec<AndroidProperty>> = Box::new(Vec::new());
    unsafe {
        __system_property_foreach(foreach_property_callback, &mut *properties);
    }
    properties.into_iter()
}

/// Find property_info pointer using bionic syscall
///
/// returns nullptr if not found, otherwise valid pointer
#[cfg(not(feature = "bionic-deprecated"))]
pub fn plat_get_property_info(name: &str) -> *const c_void {
    let cname = CString::new(name).unwrap();
    unsafe { __system_property_find(cname.as_ptr()) }
}

/// Deprecated version to find property_info pointer
///
/// Always returns nullptr
#[cfg(feature = "bionic-deprecated")]
pub fn plat_get_property_info(_name: &str) -> *const c_void {
    std::ptr::null()
}
