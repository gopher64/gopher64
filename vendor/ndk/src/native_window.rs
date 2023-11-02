//! Bindings for [`ANativeWindow`]
//!
//! [`ANativeWindow`]: https://developer.android.com/ndk/reference/group/a-native-window#anativewindow

use crate::utils::status_to_io_result;

pub use super::hardware_buffer_format::HardwareBufferFormat;
use jni_sys::{jobject, JNIEnv};
use raw_window_handle::{AndroidNdkWindowHandle, HasRawWindowHandle, RawWindowHandle};
use std::{convert::TryFrom, ffi::c_void, io::Result, ptr::NonNull};

// [`NativeWindow`] represents the producer end of an image queue
///
/// It is the C counterpart of the [`android.view.Surface`] object in Java, and can be converted
/// both ways. Depending on the consumer, images submitted to [`NativeWindow`] can be shown on the
/// display or sent to other consumers, such as video encoders.
///
/// [`android.view.Surface`]: https://developer.android.com/reference/android/view/Surface
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NativeWindow {
    ptr: NonNull<ffi::ANativeWindow>,
}

unsafe impl Send for NativeWindow {}
unsafe impl Sync for NativeWindow {}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe { ffi::ANativeWindow_release(self.ptr.as_ptr()) }
    }
}

impl Clone for NativeWindow {
    fn clone(&self) -> Self {
        unsafe { ffi::ANativeWindow_acquire(self.ptr.as_ptr()) }
        Self { ptr: self.ptr }
    }
}

unsafe impl HasRawWindowHandle for NativeWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = AndroidNdkWindowHandle::empty();
        handle.a_native_window = self.ptr.as_ptr() as *mut c_void;
        RawWindowHandle::AndroidNdk(handle)
    }
}

impl NativeWindow {
    /// Assumes ownership of `ptr`
    ///
    /// # Safety
    /// `ptr` must be a valid pointer to an Android [`ffi::ANativeWindow`].
    pub unsafe fn from_ptr(ptr: NonNull<ffi::ANativeWindow>) -> Self {
        Self { ptr }
    }

    /// Acquires ownership of `ptr`
    ///
    /// # Safety
    /// `ptr` must be a valid pointer to an Android [`ffi::ANativeWindow`].
    pub unsafe fn clone_from_ptr(ptr: NonNull<ffi::ANativeWindow>) -> Self {
        ffi::ANativeWindow_acquire(ptr.as_ptr());
        Self::from_ptr(ptr)
    }

    pub fn ptr(&self) -> NonNull<ffi::ANativeWindow> {
        self.ptr
    }

    pub fn height(&self) -> i32 {
        unsafe { ffi::ANativeWindow_getHeight(self.ptr.as_ptr()) }
    }

    pub fn width(&self) -> i32 {
        unsafe { ffi::ANativeWindow_getWidth(self.ptr.as_ptr()) }
    }

    /// Return the current pixel format ([`HardwareBufferFormat`]) of the window surface.
    pub fn format(&self) -> HardwareBufferFormat {
        let value = unsafe { ffi::ANativeWindow_getFormat(self.ptr.as_ptr()) };
        let value = u32::try_from(value).unwrap();
        HardwareBufferFormat::try_from(value).unwrap()
    }

    /// Change the format and size of the window buffers.
    ///
    /// The width and height control the number of pixels in the buffers, not the dimensions of the
    /// window on screen. If these are different than the window's physical size, then its buffer
    /// will be scaled to match that size when compositing it to the screen. The width and height
    /// must be either both zero or both non-zero.
    ///
    /// For all of these parameters, if `0` or [`None`] is supplied then the window's base value
    /// will come back in force.
    pub fn set_buffers_geometry(
        &self,
        width: i32,
        height: i32,
        format: Option<HardwareBufferFormat>,
    ) -> Result<()> {
        let format: u32 = format.map_or(0, |f| f.into());
        let status = unsafe {
            ffi::ANativeWindow_setBuffersGeometry(self.ptr.as_ptr(), width, height, format as i32)
        };
        status_to_io_result(status, ())
    }

    /// Return the [`NativeWindow`] associated with a JNI [`android.view.Surface`] pointer.
    ///
    /// # Safety
    /// By calling this function, you assert that `env` is a valid pointer to a [`JNIEnv`] and
    /// `surface` is a valid pointer to an [`android.view.Surface`].
    ///
    /// [`android.view.Surface`]: https://developer.android.com/reference/android/view/Surface
    pub unsafe fn from_surface(env: *mut JNIEnv, surface: jobject) -> Option<Self> {
        let ptr = ffi::ANativeWindow_fromSurface(env, surface);
        Some(Self::from_ptr(NonNull::new(ptr)?))
    }

    /// Return a JNI [`android.view.Surface`] pointer derived from this [`NativeWindow`].
    ///
    /// # Safety
    /// By calling this function, you assert that `env` is a valid pointer to a [`JNIEnv`].
    ///
    /// [`android.view.Surface`]: https://developer.android.com/reference/android/view/Surface
    #[cfg(feature = "api-level-26")]
    pub unsafe fn to_surface(&self, env: *mut JNIEnv) -> jobject {
        ffi::ANativeWindow_toSurface(env, self.ptr().as_ptr())
    }
}
