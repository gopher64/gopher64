//! Bindings for [`AHardwareBuffer_Format`]
//!
//! [`AHardwareBuffer_Format`]: https://developer.android.com/ndk/reference/group/a-hardware-buffer#ahardwarebuffer_format
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Buffer pixel formats.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[allow(non_camel_case_types)]
pub enum HardwareBufferFormat {
    /// Matches deprecated [`ffi::ANativeWindow_LegacyFormat::WINDOW_FORMAT_RGBA_8888`].
    R8G8B8A8_UNORM = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM.0,
    /// Matches deprecated [`ffi::ANativeWindow_LegacyFormat::WINDOW_FORMAT_RGBX_8888`].
    R8G8B8X8_UNORM = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R8G8B8X8_UNORM.0,
    #[cfg(feature = "api-level-26")]
    R8G8B8_UNORM = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R8G8B8_UNORM.0,
    /// Matches deprecated [`ffi::ANativeWindow_LegacyFormat::WINDOW_FORMAT_RGB_565`].
    R5G6B5_UNORM = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R5G6B5_UNORM.0,
    R16G16B16A16_FLOAT = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R16G16B16A16_FLOAT.0,
    #[cfg(feature = "api-level-26")]
    R10G10B10A2_UNORM = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R10G10B10A2_UNORM.0,
    #[cfg(feature = "api-level-26")]
    BLOB = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_BLOB.0,
    #[cfg(feature = "api-level-26")]
    D16_UNORM = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_D16_UNORM.0,
    #[cfg(feature = "api-level-26")]
    D24_UNORM = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_D24_UNORM.0,
    #[cfg(feature = "api-level-26")]
    D24_UNORM_S8_UINT = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_D24_UNORM_S8_UINT.0,
    #[cfg(feature = "api-level-26")]
    D32_FLOAT = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_D32_FLOAT.0,
    #[cfg(feature = "api-level-26")]
    D32_FLOAT_S8_UINT = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_D32_FLOAT_S8_UINT.0,
    #[cfg(feature = "api-level-26")]
    S8_UINT = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_S8_UINT.0,
    #[cfg(feature = "api-level-26")]
    Y8Cb8Cr8_420 = ffi::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_Y8Cb8Cr8_420.0,
}
