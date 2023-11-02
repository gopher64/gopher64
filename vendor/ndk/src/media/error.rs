use super::Result;
use thiserror::Error;

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MediaErrorResult {
    CodecErrorInsufficientResource = ffi::media_status_t::AMEDIACODEC_ERROR_INSUFFICIENT_RESOURCE.0,
    CodecErrorReclaimed = ffi::media_status_t::AMEDIACODEC_ERROR_RECLAIMED.0,
    ErrorUnknown = ffi::media_status_t::AMEDIA_ERROR_UNKNOWN.0,
    ErrorMalformed = ffi::media_status_t::AMEDIA_ERROR_MALFORMED.0,
    ErrorUnsupported = ffi::media_status_t::AMEDIA_ERROR_UNSUPPORTED.0,
    ErrorInvalidObject = ffi::media_status_t::AMEDIA_ERROR_INVALID_OBJECT.0,
    ErrorInvalidParameter = ffi::media_status_t::AMEDIA_ERROR_INVALID_PARAMETER.0,
    ErrorInvalidOperation = ffi::media_status_t::AMEDIA_ERROR_INVALID_OPERATION.0,
    ErrorEndOfStream = ffi::media_status_t::AMEDIA_ERROR_END_OF_STREAM.0,
    ErrorIo = ffi::media_status_t::AMEDIA_ERROR_IO.0,
    ErrorWouldBlock = ffi::media_status_t::AMEDIA_ERROR_WOULD_BLOCK.0,
    DrmErrorBase = ffi::media_status_t::AMEDIA_DRM_ERROR_BASE.0,
    DrmNotProvisioned = ffi::media_status_t::AMEDIA_DRM_NOT_PROVISIONED.0,
    DrmResourceBusy = ffi::media_status_t::AMEDIA_DRM_RESOURCE_BUSY.0,
    DrmDeviceRevoked = ffi::media_status_t::AMEDIA_DRM_DEVICE_REVOKED.0,
    DrmShortBuffer = ffi::media_status_t::AMEDIA_DRM_SHORT_BUFFER.0,
    DrmSessionNotOpened = ffi::media_status_t::AMEDIA_DRM_SESSION_NOT_OPENED.0,
    DrmTamperDetected = ffi::media_status_t::AMEDIA_DRM_TAMPER_DETECTED.0,
    DrmVerifyFailed = ffi::media_status_t::AMEDIA_DRM_VERIFY_FAILED.0,
    DrmNeedKey = ffi::media_status_t::AMEDIA_DRM_NEED_KEY.0,
    DrmLicenseExpired = ffi::media_status_t::AMEDIA_DRM_LICENSE_EXPIRED.0,
    ImgreaderErrorBase = ffi::media_status_t::AMEDIA_IMGREADER_ERROR_BASE.0,
    ImgreaderNoBufferAvailable = ffi::media_status_t::AMEDIA_IMGREADER_NO_BUFFER_AVAILABLE.0,
    ImgreaderMaxImagesAcquired = ffi::media_status_t::AMEDIA_IMGREADER_MAX_IMAGES_ACQUIRED.0,
    ImgreaderCannotLockImage = ffi::media_status_t::AMEDIA_IMGREADER_CANNOT_LOCK_IMAGE.0,
    ImgreaderCannotUnlockImage = ffi::media_status_t::AMEDIA_IMGREADER_CANNOT_UNLOCK_IMAGE.0,
    ImgreaderImageNotLocked = ffi::media_status_t::AMEDIA_IMGREADER_IMAGE_NOT_LOCKED.0,
}

#[derive(Debug, Error)]
pub enum NdkMediaError {
    #[error("error Media result ({0:?})")]
    ErrorResult(MediaErrorResult),
    #[error("unknown Media error result ({0:?})")]
    UnknownResult(ffi::media_status_t),
}

impl NdkMediaError {
    pub(crate) fn from_status(status: ffi::media_status_t) -> Result<()> {
        use MediaErrorResult::*;
        let result = match status {
            ffi::media_status_t::AMEDIA_OK => return Ok(()),
            ffi::media_status_t::AMEDIACODEC_ERROR_INSUFFICIENT_RESOURCE => {
                CodecErrorInsufficientResource
            }
            ffi::media_status_t::AMEDIACODEC_ERROR_RECLAIMED => CodecErrorReclaimed,
            ffi::media_status_t::AMEDIA_ERROR_UNKNOWN => ErrorUnknown,
            ffi::media_status_t::AMEDIA_ERROR_MALFORMED => ErrorMalformed,
            ffi::media_status_t::AMEDIA_ERROR_UNSUPPORTED => ErrorUnsupported,
            ffi::media_status_t::AMEDIA_ERROR_INVALID_OBJECT => ErrorInvalidObject,
            ffi::media_status_t::AMEDIA_ERROR_INVALID_PARAMETER => ErrorInvalidParameter,
            ffi::media_status_t::AMEDIA_ERROR_INVALID_OPERATION => ErrorInvalidOperation,
            ffi::media_status_t::AMEDIA_ERROR_END_OF_STREAM => ErrorEndOfStream,
            ffi::media_status_t::AMEDIA_ERROR_IO => ErrorIo,
            ffi::media_status_t::AMEDIA_ERROR_WOULD_BLOCK => ErrorWouldBlock,
            ffi::media_status_t::AMEDIA_DRM_ERROR_BASE => DrmErrorBase,
            ffi::media_status_t::AMEDIA_DRM_NOT_PROVISIONED => DrmNotProvisioned,
            ffi::media_status_t::AMEDIA_DRM_RESOURCE_BUSY => DrmResourceBusy,
            ffi::media_status_t::AMEDIA_DRM_DEVICE_REVOKED => DrmDeviceRevoked,
            ffi::media_status_t::AMEDIA_DRM_SHORT_BUFFER => DrmShortBuffer,
            ffi::media_status_t::AMEDIA_DRM_SESSION_NOT_OPENED => DrmSessionNotOpened,
            ffi::media_status_t::AMEDIA_DRM_TAMPER_DETECTED => DrmTamperDetected,
            ffi::media_status_t::AMEDIA_DRM_VERIFY_FAILED => DrmVerifyFailed,
            ffi::media_status_t::AMEDIA_DRM_NEED_KEY => DrmNeedKey,
            ffi::media_status_t::AMEDIA_DRM_LICENSE_EXPIRED => DrmLicenseExpired,
            ffi::media_status_t::AMEDIA_IMGREADER_ERROR_BASE => ImgreaderErrorBase,
            ffi::media_status_t::AMEDIA_IMGREADER_NO_BUFFER_AVAILABLE => ImgreaderNoBufferAvailable,
            ffi::media_status_t::AMEDIA_IMGREADER_MAX_IMAGES_ACQUIRED => ImgreaderMaxImagesAcquired,
            ffi::media_status_t::AMEDIA_IMGREADER_CANNOT_LOCK_IMAGE => ImgreaderCannotLockImage,
            ffi::media_status_t::AMEDIA_IMGREADER_CANNOT_UNLOCK_IMAGE => ImgreaderCannotUnlockImage,
            ffi::media_status_t::AMEDIA_IMGREADER_IMAGE_NOT_LOCKED => ImgreaderImageNotLocked,
            _ => return Err(NdkMediaError::UnknownResult(status)),
        };
        Err(NdkMediaError::ErrorResult(result))
    }
}
