//! Bindings for [`AMediaFormat`] and [`AMediaCodec`]
//!
//! [`AMediaFormat`]: https://developer.android.com/ndk/reference/group/media#amediaformat
//! [`AMediaCodec`]: https://developer.android.com/ndk/reference/group/media#amediacodec

use super::{get_unlikely_to_be_null, NdkMediaError, Result};
use crate::native_window::NativeWindow;
use std::{
    convert::TryInto,
    ffi::{CStr, CString},
    fmt::Display,
    ptr::{self, NonNull},
    slice,
    time::Duration,
};

#[derive(Debug, PartialEq, Eq)]
pub enum MediaCodecDirection {
    Decoder,
    Encoder,
}

/// A native [`AMediaFormat *`]
///
/// [`AMediaFormat *`]: https://developer.android.com/ndk/reference/group/media#amediaformat
#[derive(Debug)]
pub struct MediaFormat {
    inner: NonNull<ffi::AMediaFormat>,
}

impl Display for MediaFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c_str = unsafe { CStr::from_ptr(ffi::AMediaFormat_toString(self.as_ptr())) };
        f.write_str(c_str.to_str().unwrap())
    }
}

impl Default for MediaFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaFormat {
    fn as_ptr(&self) -> *mut ffi::AMediaFormat {
        self.inner.as_ptr()
    }

    pub fn new() -> Self {
        Self {
            inner: NonNull::new(unsafe { ffi::AMediaFormat_new() }).unwrap(),
        }
    }

    pub fn i32(&self, key: &str) -> Option<i32> {
        let name = CString::new(key).unwrap();
        let mut out = 0;
        unsafe { ffi::AMediaFormat_getInt32(self.as_ptr(), name.as_ptr(), &mut out) }.then(|| out)
    }

    pub fn i64(&self, key: &str) -> Option<i64> {
        let name = CString::new(key).unwrap();
        let mut out = 0;
        unsafe { ffi::AMediaFormat_getInt64(self.as_ptr(), name.as_ptr(), &mut out) }.then(|| out)
    }

    pub fn f32(&self, key: &str) -> Option<f32> {
        let name = CString::new(key).unwrap();
        let mut out = 0.0;
        unsafe { ffi::AMediaFormat_getFloat(self.as_ptr(), name.as_ptr(), &mut out) }.then(|| out)
    }

    pub fn usize(&self, key: &str) -> Option<usize> {
        let name = CString::new(key).unwrap();
        let mut out = 0;
        unsafe { ffi::AMediaFormat_getSize(self.as_ptr(), name.as_ptr(), &mut out) }
            .then(|| out as usize)
    }

    pub fn buffer(&self, key: &str) -> Option<&[u8]> {
        let name = CString::new(key).unwrap();
        let mut out_buffer = ptr::null_mut();
        let mut out_size = 0;
        unsafe {
            ffi::AMediaFormat_getBuffer(
                self.as_ptr(),
                name.as_ptr(),
                &mut out_buffer,
                &mut out_size,
            )
        }
        .then(|| unsafe { slice::from_raw_parts(out_buffer.cast(), out_size as usize) })
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        let name = CString::new(key).unwrap();
        let mut out = ptr::null();
        unsafe { ffi::AMediaFormat_getString(self.as_ptr(), name.as_ptr(), &mut out) }
            .then(|| unsafe { CStr::from_ptr(out) }.to_str().unwrap())
    }

    pub fn set_i32(&self, key: &str, value: i32) {
        let name = CString::new(key).unwrap();
        unsafe { ffi::AMediaFormat_setInt32(self.as_ptr(), name.as_ptr(), value) };
    }

    pub fn set_i64(&self, key: &str, value: i64) {
        let name = CString::new(key).unwrap();
        unsafe { ffi::AMediaFormat_setInt64(self.as_ptr(), name.as_ptr(), value) };
    }

    pub fn set_f32(&self, key: &str, value: f32) {
        let name = CString::new(key).unwrap();
        unsafe { ffi::AMediaFormat_setFloat(self.as_ptr(), name.as_ptr(), value) };
    }

    pub fn set_str(&self, key: &str, value: &str) {
        let name = CString::new(key).unwrap();
        let c_string = CString::new(value).unwrap();
        unsafe { ffi::AMediaFormat_setString(self.as_ptr(), name.as_ptr(), c_string.as_ptr()) };
    }

    pub fn set_buffer(&self, key: &str, value: &[u8]) {
        let name = CString::new(key).unwrap();
        unsafe {
            ffi::AMediaFormat_setBuffer(
                self.as_ptr(),
                name.as_ptr(),
                value.as_ptr().cast(),
                value.len() as ffi::size_t,
            )
        };
    }

    #[cfg(feature = "api-level-28")]
    pub fn f64(&self, key: &str) -> Option<f64> {
        let name = CString::new(key).unwrap();
        let mut out = 0.0;
        unsafe { ffi::AMediaFormat_getDouble(self.as_ptr(), name.as_ptr(), &mut out) }.then(|| out)
    }

    /// Returns (left, top, right, bottom)
    #[cfg(feature = "api-level-28")]
    pub fn rect(&self, key: &str) -> Option<(i32, i32, i32, i32)> {
        let name = CString::new(key).unwrap();
        let mut left = 0;
        let mut top = 0;
        let mut right = 0;
        let mut bottom = 0;
        unsafe {
            ffi::AMediaFormat_getRect(
                self.as_ptr(),
                name.as_ptr(),
                &mut left,
                &mut top,
                &mut right,
                &mut bottom,
            )
        }
        .then(|| (left, top, right, bottom))
    }

    #[cfg(feature = "api-level-28")]
    pub fn set_f64(&self, key: &str, value: f64) {
        let name = CString::new(key).unwrap();
        unsafe { ffi::AMediaFormat_setDouble(self.as_ptr(), name.as_ptr(), value) };
    }

    #[cfg(feature = "api-level-28")]
    pub fn set_rect(&self, key: &str, left: i32, top: i32, right: i32, bottom: i32) {
        let name = CString::new(key).unwrap();
        unsafe {
            ffi::AMediaFormat_setRect(self.as_ptr(), name.as_ptr(), left, top, right, bottom)
        };
    }

    #[cfg(feature = "api-level-28")]
    pub fn set_usize(&self, key: &str, value: usize) {
        let name = CString::new(key).unwrap();
        unsafe { ffi::AMediaFormat_setSize(self.as_ptr(), name.as_ptr(), value as ffi::size_t) };
    }
}

impl Drop for MediaFormat {
    fn drop(&mut self) {
        let status = unsafe { ffi::AMediaFormat_delete(self.as_ptr()) };
        NdkMediaError::from_status(status).unwrap();
    }
}

/// A native [`AMediaCodec *`]
///
/// [`AMediaCodec *`]: https://developer.android.com/ndk/reference/group/media#amediacodec
#[derive(Debug)]
pub struct MediaCodec {
    inner: NonNull<ffi::AMediaCodec>,
}

impl MediaCodec {
    fn as_ptr(&self) -> *mut ffi::AMediaCodec {
        self.inner.as_ptr()
    }

    pub fn from_codec_name(name: &str) -> Option<Self> {
        let c_string = CString::new(name).unwrap();
        Some(Self {
            inner: NonNull::new(unsafe { ffi::AMediaCodec_createCodecByName(c_string.as_ptr()) })?,
        })
    }

    pub fn from_decoder_type(mime_type: &str) -> Option<Self> {
        let c_string = CString::new(mime_type).unwrap();
        Some(Self {
            inner: NonNull::new(unsafe {
                ffi::AMediaCodec_createDecoderByType(c_string.as_ptr())
            })?,
        })
    }

    pub fn from_encoder_type(mime_type: &str) -> Option<Self> {
        let c_string = CString::new(mime_type).unwrap();
        Some(Self {
            inner: NonNull::new(unsafe {
                ffi::AMediaCodec_createEncoderByType(c_string.as_ptr())
            })?,
        })
    }

    pub fn configure(
        &self,
        format: &MediaFormat,
        surface: Option<&NativeWindow>,
        direction: MediaCodecDirection,
    ) -> Result<()> {
        let status = unsafe {
            ffi::AMediaCodec_configure(
                self.as_ptr(),
                format.as_ptr(),
                surface.map_or(ptr::null_mut(), |s| s.ptr().as_ptr()),
                ptr::null_mut(),
                if direction == MediaCodecDirection::Encoder {
                    ffi::AMEDIACODEC_CONFIGURE_FLAG_ENCODE as u32
                } else {
                    0
                },
            )
        };
        NdkMediaError::from_status(status)
    }

    #[cfg(feature = "api-level-26")]
    pub fn create_input_surface(&self) -> Result<NativeWindow> {
        use super::construct_never_null;
        unsafe {
            let ptr = construct_never_null(|res| {
                ffi::AMediaCodec_createInputSurface(self.as_ptr(), res)
            })?;
            Ok(NativeWindow::from_ptr(ptr))
        }
    }

    #[cfg(feature = "api-level-26")]
    pub fn create_persistent_input_surface() -> Result<NativeWindow> {
        use super::construct_never_null;
        unsafe {
            let ptr =
                construct_never_null(|res| ffi::AMediaCodec_createPersistentInputSurface(res))?;
            Ok(NativeWindow::from_ptr(ptr))
        }
    }

    /// Returns [`None`] if timeout is reached.
    pub fn dequeue_input_buffer(&self, timeout: Duration) -> Result<Option<InputBuffer>> {
        let result = unsafe {
            ffi::AMediaCodec_dequeueInputBuffer(
                self.as_ptr(),
                timeout
                    .as_micros()
                    .try_into()
                    .expect("Supplied timeout is too large"),
            )
        };

        if result == ffi::AMEDIACODEC_INFO_TRY_AGAIN_LATER as ffi::ssize_t {
            Ok(None)
        } else if result >= 0 {
            Ok(Some(InputBuffer {
                codec: self,
                index: result as ffi::size_t,
            }))
        } else {
            NdkMediaError::from_status(ffi::media_status_t(result as _)).map(|()| None)
        }
    }

    /// Returns [`None`] if timeout is reached.
    pub fn dequeue_output_buffer(&self, timeout: Duration) -> Result<Option<OutputBuffer>> {
        let mut info: ffi::AMediaCodecBufferInfo = unsafe { std::mem::zeroed() };

        let result = unsafe {
            ffi::AMediaCodec_dequeueOutputBuffer(
                self.as_ptr(),
                &mut info,
                timeout
                    .as_micros()
                    .try_into()
                    .expect("Supplied timeout is too large"),
            )
        };

        if result == ffi::AMEDIACODEC_INFO_TRY_AGAIN_LATER as ffi::ssize_t {
            Ok(None)
        } else if result >= 0 {
            Ok(Some(OutputBuffer {
                codec: self,
                index: result as ffi::size_t,
                info,
            }))
        } else {
            NdkMediaError::from_status(ffi::media_status_t(result as _)).map(|()| None)
        }
    }

    pub fn flush(&self) -> Result<()> {
        let status = unsafe { ffi::AMediaCodec_flush(self.as_ptr()) };
        NdkMediaError::from_status(status)
    }

    #[cfg(feature = "api-level-28")]
    pub fn input_format(&self) -> MediaFormat {
        let inner =
            get_unlikely_to_be_null(|| unsafe { ffi::AMediaCodec_getInputFormat(self.as_ptr()) });
        MediaFormat { inner }
    }

    pub fn output_format(&self) -> MediaFormat {
        let inner =
            get_unlikely_to_be_null(|| unsafe { ffi::AMediaCodec_getOutputFormat(self.as_ptr()) });
        MediaFormat { inner }
    }

    #[cfg(feature = "api-level-28")]
    pub fn name(&self) -> Result<String> {
        use super::construct;
        unsafe {
            let name_ptr = construct(|name| ffi::AMediaCodec_getName(self.as_ptr(), name))?;
            let name = CStr::from_ptr(name_ptr).to_str().unwrap().to_owned();
            ffi::AMediaCodec_releaseName(self.as_ptr(), name_ptr);

            Ok(name)
        }
    }

    pub fn queue_input_buffer(
        &self,
        buffer: InputBuffer,
        offset: usize,
        size: usize,
        time: u64,
        flags: u32,
    ) -> Result<()> {
        let status = unsafe {
            ffi::AMediaCodec_queueInputBuffer(
                self.as_ptr(),
                buffer.index as ffi::size_t,
                offset as ffi::off_t,
                size as ffi::size_t,
                time,
                flags,
            )
        };
        NdkMediaError::from_status(status)
    }

    pub fn release_output_buffer(&self, buffer: OutputBuffer, render: bool) -> Result<()> {
        let status = unsafe {
            ffi::AMediaCodec_releaseOutputBuffer(self.as_ptr(), buffer.index as ffi::size_t, render)
        };
        NdkMediaError::from_status(status)
    }

    pub fn release_output_buffer_at_time(
        &self,
        buffer: OutputBuffer,
        timestamp_ns: i64,
    ) -> Result<()> {
        let status = unsafe {
            ffi::AMediaCodec_releaseOutputBufferAtTime(
                self.as_ptr(),
                buffer.index as ffi::size_t,
                timestamp_ns,
            )
        };
        NdkMediaError::from_status(status)
    }

    #[cfg(feature = "api-level-26")]
    pub fn set_input_surface(&self, surface: &NativeWindow) -> Result<()> {
        let status =
            unsafe { ffi::AMediaCodec_setInputSurface(self.as_ptr(), surface.ptr().as_ptr()) };
        NdkMediaError::from_status(status)
    }

    pub fn set_output_surface(&self, surface: &NativeWindow) -> Result<()> {
        let status =
            unsafe { ffi::AMediaCodec_setOutputSurface(self.as_ptr(), surface.ptr().as_ptr()) };
        NdkMediaError::from_status(status)
    }

    #[cfg(feature = "api-level-26")]
    pub fn set_parameters(&self, params: MediaFormat) -> Result<()> {
        let status = unsafe { ffi::AMediaCodec_setParameters(self.as_ptr(), params.as_ptr()) };
        NdkMediaError::from_status(status)
    }

    #[cfg(feature = "api-level-26")]
    pub fn set_signal_end_of_input_stream(&self) -> Result<()> {
        let status = unsafe { ffi::AMediaCodec_signalEndOfInputStream(self.as_ptr()) };
        NdkMediaError::from_status(status)
    }

    pub fn start(&self) -> Result<()> {
        let status = unsafe { ffi::AMediaCodec_start(self.as_ptr()) };
        NdkMediaError::from_status(status)
    }

    pub fn stop(&self) -> Result<()> {
        let status = unsafe { ffi::AMediaCodec_stop(self.as_ptr()) };
        NdkMediaError::from_status(status)
    }
}

impl Drop for MediaCodec {
    fn drop(&mut self) {
        let status = unsafe { ffi::AMediaCodec_delete(self.as_ptr()) };
        NdkMediaError::from_status(status).unwrap();
    }
}

#[derive(Debug)]
pub struct InputBuffer<'a> {
    codec: &'a MediaCodec,
    index: ffi::size_t,
}

impl InputBuffer<'_> {
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut out_size = 0;
            let buffer_ptr =
                ffi::AMediaCodec_getInputBuffer(self.codec.as_ptr(), self.index, &mut out_size);
            assert!(!buffer_ptr.is_null());
            slice::from_raw_parts_mut(buffer_ptr, out_size as usize)
        }
    }
}

#[derive(Debug)]
pub struct OutputBuffer<'a> {
    codec: &'a MediaCodec,
    index: ffi::size_t,
    info: ffi::AMediaCodecBufferInfo,
}

impl OutputBuffer<'_> {
    pub fn buffer(&self) -> &[u8] {
        unsafe {
            let mut _out_size = 0;
            let buffer_ptr =
                ffi::AMediaCodec_getOutputBuffer(self.codec.as_ptr(), self.index, &mut _out_size);
            assert!(!buffer_ptr.is_null());
            slice::from_raw_parts(
                buffer_ptr.add(self.info.offset as usize),
                self.info.size as usize,
            )
        }
    }

    #[cfg(feature = "api-level-28")]
    pub fn format(&self) -> MediaFormat {
        let inner = get_unlikely_to_be_null(|| unsafe {
            ffi::AMediaCodec_getBufferFormat(self.codec.as_ptr(), self.index)
        });
        MediaFormat { inner }
    }

    pub fn flags(&self) -> u32 {
        self.info.flags
    }

    pub fn presentation_time_us(&self) -> i64 {
        self.info.presentationTimeUs
    }
}
