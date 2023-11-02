use log::Level;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

pub fn try_get_path_from_ptr(path: *const c_char) -> Option<std::path::PathBuf> {
    if path.is_null() {
        return None;
    }
    let cstr = unsafe {
        let cstr_slice = CStr::from_ptr(path.cast());
        cstr_slice.to_str().ok()?
    };
    if cstr.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(cstr))
}

pub(crate) fn android_log(level: Level, tag: &CStr, msg: &CStr) {
    let prio = match level {
        Level::Error => ndk_sys::android_LogPriority::ANDROID_LOG_ERROR,
        Level::Warn => ndk_sys::android_LogPriority::ANDROID_LOG_WARN,
        Level::Info => ndk_sys::android_LogPriority::ANDROID_LOG_INFO,
        Level::Debug => ndk_sys::android_LogPriority::ANDROID_LOG_DEBUG,
        Level::Trace => ndk_sys::android_LogPriority::ANDROID_LOG_VERBOSE,
    };
    unsafe {
        ndk_sys::__android_log_write(prio.0 as libc::c_int, tag.as_ptr(), msg.as_ptr());
    }
}

pub(crate) fn log_panic(panic: Box<dyn std::any::Any + Send>) {
    let rust_panic = unsafe { CStr::from_bytes_with_nul_unchecked(b"RustPanic\0") };

    if let Some(panic) = panic.downcast_ref::<String>() {
        if let Ok(msg) = CString::new(panic.clone()) {
            android_log(Level::Error, rust_panic, &msg);
        }
    } else if let Ok(panic) = panic.downcast::<&str>() {
        if let Ok(msg) = CString::new(*panic) {
            android_log(Level::Error, rust_panic, &msg);
        }
    } else {
        let unknown_panic = unsafe { CStr::from_bytes_with_nul_unchecked(b"UnknownPanic\0") };
        android_log(Level::Error, unknown_panic, unsafe {
            CStr::from_bytes_with_nul_unchecked(b"\0")
        });
    }
}

/// Run a closure and abort the program if it panics.
///
/// This is generally used to ensure Rust callbacks won't unwind past the JNI boundary, which leads
/// to undefined behaviour.
///
/// TODO(rib): throw a Java exception instead of aborting. An Android Activity does not necessarily
/// own the entire process because other application Services (or even Activities) may run in
/// threads within the same process, and so we're tearing down too much by aborting the process.
pub(crate) fn abort_on_panic<R>(f: impl FnOnce() -> R) -> R {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|panic| {
        // Try logging the panic before aborting
        //
        // Just in case our attempt to log a panic could itself cause a panic we use a
        // second catch_unwind here.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| log_panic(panic)));
        std::process::abort();
    })
}
