#!/bin/sh

while read ARCH && read TARGET ; do
    bindgen wrapper.h -o src/ffi_$ARCH.rs \
        --blocklist-item 'JNI\w+' \
        --blocklist-item 'C?_?JNIEnv' \
        --blocklist-item '_?JavaVM' \
        --blocklist-item '_?j\w+' \
        --newtype-enum '\w+_(result|status)_t' \
        --newtype-enum 'ACameraDevice_request_template' \
        --newtype-enum 'ADataSpace' \
        --newtype-enum 'AHardwareBuffer_Format' \
        --newtype-enum 'AHardwareBuffer_UsageFlags' \
        --newtype-enum 'AHdrMetadataType' \
        --newtype-enum 'AIMAGE_FORMATS' \
        --newtype-enum 'AMediaDrmEventType' \
        --newtype-enum 'AMediaDrmKeyType' \
        --newtype-enum 'AMediaKeyStatusType' \
        --newtype-enum 'ANativeWindowTransform' \
        --newtype-enum 'ANativeWindow_ChangeFrameRateStrategy' \
        --newtype-enum 'ANativeWindow_FrameRateCompatibility' \
        --newtype-enum 'ANativeWindow_LegacyFormat' \
        --newtype-enum 'AndroidBitmapCompressFormat' \
        --newtype-enum 'AndroidBitmapFormat' \
        --newtype-enum 'AppendMode' \
        --newtype-enum 'DeviceTypeCode' \
        --newtype-enum 'DurationCode' \
        --newtype-enum 'FeatureLevelCode' \
        --newtype-enum 'FuseCode' \
        --newtype-enum 'HeapTaggingLevel' \
        --newtype-enum 'OperandCode' \
        --newtype-enum 'OperationCode' \
        --newtype-enum 'OutputFormat' \
        --newtype-enum 'PaddingCode' \
        --newtype-enum 'PreferenceCode' \
        --newtype-enum 'PriorityCode' \
        --newtype-enum 'ResNsendFlags' \
        --newtype-enum 'ResultCode' \
        --newtype-enum 'SeekMode' \
        --newtype-enum 'acamera_\w+' \
        --newtype-enum 'android_LogPriority' \
        --newtype-enum 'android_fdsan_error_level' \
        --newtype-enum 'android_fdsan_owner_type' \
        --newtype-enum 'cryptoinfo_mode_t' \
        --newtype-enum 'log_id' \
        -- \
        --sysroot="${ANDROID_NDK_ROOT}"/toolchains/llvm/prebuilt/linux-x86_64/sysroot/ --target=$TARGET

        # --newtype-enum '_bindgen_ty_63' \
done << EOF
arm
arm-linux-androideabi
aarch64
aarch64-linux-android
i686
i686-linux-android
x86_64
x86_64-linux-android
EOF
