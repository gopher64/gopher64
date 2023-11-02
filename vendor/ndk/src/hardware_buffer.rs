//! Bindings for [`AHardwareBuffer`]
//!
//! [`AHardwareBuffer`]: https://developer.android.com/ndk/reference/group/a-hardware-buffer#ahardwarebuffer

#![cfg(feature = "api-level-26")]

use crate::utils::status_to_io_result;

pub use super::hardware_buffer_format::HardwareBufferFormat;
use jni_sys::{jobject, JNIEnv};
use std::{
    convert::TryInto, io::Result, mem::MaybeUninit, ops::Deref, os::raw::c_void,
    os::unix::io::RawFd, ptr::NonNull,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HardwareBufferUsage(pub ffi::AHardwareBuffer_UsageFlags);

impl HardwareBufferUsage {
    pub const CPU_READ_NEVER: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_READ_NEVER);
    pub const CPU_READ_RARELY: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_READ_RARELY);
    pub const CPU_READ_OFTEN: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_READ_OFTEN);
    pub const CPU_READ_MASK: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_READ_MASK);

    pub const CPU_WRITE_NEVER: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_WRITE_NEVER);
    pub const CPU_WRITE_RARELY: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_WRITE_RARELY);
    pub const CPU_WRITE_OFTEN: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_WRITE_OFTEN);
    pub const CPU_WRITE_MASK: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_WRITE_MASK);

    pub const GPU_SAMPLED_IMAGE: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE);
    pub const GPU_FRAMEBUFFER: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_GPU_FRAMEBUFFER);
    pub const COMPOSER_OVERLAY: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_COMPOSER_OVERLAY);
    pub const PROTECTED_CONTENT: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_PROTECTED_CONTENT);
    pub const VIDEO_ENCODE: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VIDEO_ENCODE);
    pub const SENSOR_DIRECT_DATA: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_SENSOR_DIRECT_DATA);
    pub const GPU_DATA_BUFFER: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_GPU_DATA_BUFFER);
    pub const GPU_CUBE_MAP: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_GPU_CUBE_MAP);
    pub const GPU_MIPMAP_COMPLETE: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_GPU_MIPMAP_COMPLETE);

    pub const VENDOR_0: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_0);
    pub const VENDOR_1: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_1);
    pub const VENDOR_2: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_2);
    pub const VENDOR_3: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_3);
    pub const VENDOR_4: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_4);
    pub const VENDOR_5: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_5);
    pub const VENDOR_6: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_6);
    pub const VENDOR_7: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_7);
    pub const VENDOR_8: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_8);
    pub const VENDOR_9: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_9);
    pub const VENDOR_10: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_10);
    pub const VENDOR_11: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_11);
    pub const VENDOR_12: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_12);
    pub const VENDOR_13: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_13);
    pub const VENDOR_14: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_14);
    pub const VENDOR_15: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_15);
    pub const VENDOR_16: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_16);
    pub const VENDOR_17: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_17);
    pub const VENDOR_18: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_18);
    pub const VENDOR_19: Self =
        Self(ffi::AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_VENDOR_19);
}

pub type Rect = ffi::ARect;

fn construct<T>(with_ptr: impl FnOnce(*mut T) -> i32) -> Result<T> {
    let mut result = MaybeUninit::uninit();
    let status = with_ptr(result.as_mut_ptr());
    status_to_io_result(status, unsafe { result.assume_init() })
}

/// A native [`AHardwareBuffer *`]
///
/// [`HardwareBuffer`] objects represent chunks of memory that can be accessed by various hardware
/// components in the system.
///
/// It can be easily converted to the Java counterpart [`android.hardware.HardwareBuffer`] and
/// passed between processes using Binder. All operations involving [`HardwareBuffer`] and
/// [`android.hardware.HardwareBuffer`] are zero-copy, i.e., passing [`HardwareBuffer`] to another
/// process creates a shared view of the same region of memory.
///
/// [`HardwareBuffer`] can be bound to EGL/OpenGL and Vulkan primitives. For EGL, use the extension
/// function [`eglGetNativeClientBufferANDROID`] to obtain an `EGLClientBuffer` and pass it
/// directly to [`eglCreateImageKHR`]. Refer to the EGL extensions
/// [`EGL_ANDROID_get_native_client_buffer`] and [`EGL_ANDROID_image_native_buffer`] for more
/// information. In Vulkan, the contents of the [`HardwareBuffer`] can be accessed as [external
/// memory]. See the [`VK_ANDROID_external_memory_android_hardware_buffer`] extension for details.
///
/// [`AHardwareBuffer *`]: https://developer.android.com/ndk/reference/group/a-hardware-buffer#ahardwarebuffer
/// [`android.hardware.HardwareBuffer`]: https://developer.android.com/reference/android/hardware/HardwareBuffer
/// [`eglGetNativeClientBufferANDROID`]: https://www.khronos.org/registry/EGL/extensions/ANDROID/EGL_ANDROID_get_native_client_buffer.txt
/// [`eglCreateImageKHR`]: https://www.khronos.org/registry/EGL/extensions/KHR/EGL_KHR_image_base.txt
/// [`EGL_ANDROID_get_native_client_buffer`]: https://www.khronos.org/registry/EGL/extensions/ANDROID/EGL_ANDROID_get_native_client_buffer.txt
/// [`EGL_ANDROID_image_native_buffer`]: https://www.khronos.org/registry/EGL/extensions/ANDROID/EGL_ANDROID_image_native_buffer.txt
/// [external memory]: https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VK_KHR_external_memory.html
/// [`VK_ANDROID_external_memory_android_hardware_buffer`]: https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VK_ANDROID_external_memory_android_hardware_buffer.html
#[derive(Debug)]
pub struct HardwareBuffer {
    inner: NonNull<ffi::AHardwareBuffer>,
}

impl HardwareBuffer {
    /// Create an _unowned_ [`HardwareBuffer`] from a native pointer
    ///
    /// To wrap a strong reference (that is `release`d on [`Drop`]), call
    /// [`HardwareBufferRef::from_ptr()`] instead.
    ///
    /// # Safety
    /// By calling this function, you assert that it is a valid pointer to an NDK
    /// [`ffi::AHardwareBuffer`] that is kept alive externally, or retrieve a strong reference
    /// using [`HardwareBuffer::acquire()`].
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AHardwareBuffer>) -> Self {
        Self { inner: ptr }
    }

    /// Returns the underlying [`ffi::AHardwareBuffer`] pointer
    ///
    /// See the top-level [`HardwareBuffer`] struct documentation for (graphics) APIs that accept
    /// this pointer.
    pub fn as_ptr(&self) -> *mut ffi::AHardwareBuffer {
        self.inner.as_ptr()
    }

    /// Allocates a buffer that matches the passed [`HardwareBufferDesc`].
    ///
    /// If allocation succeeds, the buffer can be used according to the usage flags specified in
    /// its description. If a buffer is used in ways not compatible with its usage flags, the
    /// results are undefined and may include program termination.
    pub fn allocate(desc: HardwareBufferDesc) -> Result<HardwareBufferRef> {
        unsafe {
            let ptr = construct(|res| ffi::AHardwareBuffer_allocate(&desc.into_native(), res))?;

            Ok(HardwareBufferRef::from_ptr(NonNull::new_unchecked(ptr)))
        }
    }

    /// Create a [`HardwareBuffer`] from JNI pointers
    ///
    /// # Safety
    /// By calling this function, you assert that these are valid pointers to JNI objects.
    ///
    /// This method does not acquire any additional reference to the AHardwareBuffer that is
    /// returned. To keep the [`HardwareBuffer`] alive after the [Java `HardwareBuffer`] object
    /// is closed, explicitly or by the garbage collector, be sure to retrieve a strong reference
    /// using [`HardwareBuffer::acquire()`].
    ///
    /// [Java `HardwareBuffer`]: https://developer.android.com/reference/android/hardware/HardwareBuffer
    pub unsafe fn from_jni(env: *mut JNIEnv, hardware_buffer: jobject) -> Self {
        let ptr = ffi::AHardwareBuffer_fromHardwareBuffer(env, hardware_buffer);

        Self::from_ptr(NonNull::new_unchecked(ptr))
    }

    /// # Safety
    /// By calling this function, you assert that `env` is a valid pointer to a [`JNIEnv`].
    pub unsafe fn to_jni(&self, env: *mut JNIEnv) -> jobject {
        ffi::AHardwareBuffer_toHardwareBuffer(env, self.as_ptr())
    }

    /// Return a description of the [`HardwareBuffer`] in the passed [`HardwareBufferDesc`] struct.
    pub fn describe(&self) -> HardwareBufferDesc {
        let desc = unsafe {
            let mut result = MaybeUninit::uninit();
            ffi::AHardwareBuffer_describe(self.as_ptr(), result.as_mut_ptr());
            result.assume_init()
        };

        HardwareBufferDesc {
            width: desc.width,
            height: desc.height,
            layers: desc.layers,
            format: desc.format.try_into().unwrap(),
            usage: HardwareBufferUsage(ffi::AHardwareBuffer_UsageFlags(desc.usage)),
            stride: desc.stride,
        }
    }

    /// Test whether the given format and usage flag combination is allocatable.
    ///
    /// If this function returns [`true`], it means that a buffer with the given description can
    /// be allocated on this implementation, unless resource exhaustion occurs. If this function
    /// returns [`false`], it means that the allocation of the given description will never
    /// succeed.
    ///
    /// The return value of this function may depend on all fields in the description, except
    /// [`HardwareBufferDesc::stride`], which is always ignored. For example, some implementations
    /// have implementation-defined limits on texture size and layer count.
    #[cfg(feature = "api-level-29")]
    pub fn is_supported(desc: HardwareBufferDesc) -> bool {
        let res = unsafe { ffi::AHardwareBuffer_isSupported(&desc.into_native()) };
        res == 1
    }

    /// Lock the [`HardwareBuffer`] for direct CPU access.
    ///
    /// This function can lock the buffer for either reading or writing. It may block if the
    /// hardware needs to finish rendering, if CPU caches need to be synchronized, or possibly for
    /// other implementation-specific reasons.
    ///
    /// The [`HardwareBuffer`] must have one layer, otherwise the call will fail.
    ///
    /// If `fence` is not [`None`], it specifies a fence file descriptor on which to wait before
    /// locking the buffer. If it's [`None`], the caller is responsible for ensuring that writes
    /// to the buffer have completed before calling this function. Using this parameter is more
    /// efficient than waiting on the fence and then calling this function.
    ///
    /// The `usage` parameter may only specify `HardwareBufferUsage::CPU_*`. If set, then the
    /// address of the buffer in virtual memory is returned. The flags must also be compatible with
    /// usage flags specified at buffer creation: if a read flag is passed, the buffer must have
    /// been created with [`HardwareBufferUsage::CPU_READ_RARELY`] or
    /// [`HardwareBufferUsage::CPU_READ_OFTEN`]. If a write flag is passed, it must have been
    /// created with [`HardwareBufferUsage::CPU_WRITE_RARELY`] or
    /// [`HardwareBufferUsage::CPU_WRITE_OFTEN`].
    ///
    /// If `rect` is not [`None`], the caller promises to modify only data in the area specified by
    /// `rect`. If rect is [`None`], the caller may modify the contents of the entire buffer. The
    /// content of the buffer outside of the specified rect is NOT modified by this call.
    ///
    /// It is legal for several different threads to lock a buffer for read access; none of the
    /// threads are blocked.
    ///
    /// Locking a buffer simultaneously for write or read/write is undefined, but will neither
    /// terminate the process nor block the caller. This function may return an error or leave the
    /// buffer's content in an indeterminate state.
    ///
    /// If the buffer has [`HardwareBufferFormat::BLOB`], it is legal lock it for reading and
    /// writing in multiple threads and/or processes simultaneously, and the contents of the buffer
    /// behave like shared memory.
    pub fn lock(
        &self,
        usage: HardwareBufferUsage,
        fence: Option<RawFd>,
        rect: Option<Rect>,
    ) -> Result<*mut c_void> {
        let fence = fence.unwrap_or(-1);
        let rect = match rect {
            Some(rect) => &rect,
            None => std::ptr::null(),
        };
        construct(|res| unsafe {
            ffi::AHardwareBuffer_lock(self.as_ptr(), usage.0 .0, fence, rect, res)
        })
    }

    /// Lock a [`HardwareBuffer`] for direct CPU access.
    ///
    /// This function is the same as the above [`lock()`][Self::lock()] function, but passes back
    /// additional information about the bytes per pixel and the bytes per stride of the locked
    /// buffer. If the bytes per pixel or bytes per stride are unknown or variable, or if the
    /// underlying mapper implementation does not support returning additional information, then
    /// this call will fail with [`std::io::Error::kind()`] = [`std::io::ErrorKind::Unsupported`].
    #[cfg(feature = "api-level-29")]
    pub fn lock_and_get_info(
        &self,
        usage: HardwareBufferUsage,
        fence: Option<RawFd>,
        rect: Option<Rect>,
    ) -> Result<LockedPlaneInfo> {
        let fence = fence.unwrap_or(-1);
        let rect = match rect {
            Some(rect) => &rect,
            None => std::ptr::null(),
        };
        let mut virtual_address = MaybeUninit::uninit();
        let mut bytes_per_pixel = MaybeUninit::uninit();
        let mut bytes_per_stride = MaybeUninit::uninit();
        let status = unsafe {
            ffi::AHardwareBuffer_lockAndGetInfo(
                self.as_ptr(),
                usage.0 .0,
                fence,
                rect,
                virtual_address.as_mut_ptr(),
                bytes_per_pixel.as_mut_ptr(),
                bytes_per_stride.as_mut_ptr(),
            )
        };
        status_to_io_result(status, ()).map(|()| unsafe {
            LockedPlaneInfo {
                virtual_address: virtual_address.assume_init(),
                bytes_per_pixel: bytes_per_pixel.assume_init() as u32,
                bytes_per_stride: bytes_per_stride.assume_init() as u32,
            }
        })
    }

    /// Lock a potentially multi-planar [`HardwareBuffer`] for direct CPU access.
    ///
    /// This function is similar to [`lock()`][Self::lock()], but can lock multi-planar formats.
    /// Note, that multi-planar should not be confused with multi-layer images, which this locking
    /// function does not support.
    ///
    /// YUV formats are always represented by three separate planes of data, one for each color
    /// plane. The order of planes in the array is guaranteed such that plane #0 is always `Y`,
    /// plane #1 is always `U` (`Cb`), and plane #2 is always `V` (`Cr`). All other formats are
    /// represented by a single plane.
    ///
    /// Additional information always accompanies the buffers, describing the row stride and the
    /// pixel stride for each plane.
    ///
    /// In case the buffer cannot be locked, this will return zero planes.
    ///
    /// See the [`lock()`][Self::lock()] documentation for all other locking semantics.
    #[cfg(feature = "api-level-29")]
    pub fn lock_planes(
        &self,
        usage: HardwareBufferUsage,
        fence: Option<RawFd>,
        rect: Option<Rect>,
    ) -> Result<HardwareBufferPlanes> {
        let fence = fence.unwrap_or(-1);
        let rect = match rect {
            Some(rect) => &rect,
            None => std::ptr::null(),
        };
        let planes = construct(|res| unsafe {
            ffi::AHardwareBuffer_lockPlanes(self.as_ptr(), usage.0 .0, fence, rect, res)
        })?;

        Ok(HardwareBufferPlanes {
            inner: planes,
            index: 0,
        })
    }

    /// Unlock the [`HardwareBuffer`] from direct CPU access.
    ///
    /// Must be called after all changes to the buffer are completed by the caller. The function
    /// will block until all work is completed. See [`unlock_async()`][Self::unlock_async()] for
    /// a non-blocking variant that returns a file descriptor to be signaled on unlocking instead.
    pub fn unlock(&self) -> Result<()> {
        let status = unsafe { ffi::AHardwareBuffer_unlock(self.as_ptr(), std::ptr::null_mut()) };
        status_to_io_result(status, ())
    }

    /// Unlock the [`HardwareBuffer`] from direct CPU access.
    ///
    /// Returns a fence file descriptor that will become signaled when unlocking is completed, or
    /// [`None`] if unlocking is already finished. The caller is responsible for closing the file
    /// descriptor once it's no longer needed. See [`unlock()`][Self::unlock()] for a variant that
    /// blocks instead.
    pub fn unlock_async(&self) -> Result<Option<RawFd>> {
        let fence = construct(|res| unsafe { ffi::AHardwareBuffer_unlock(self.as_ptr(), res) })?;
        Ok(match fence {
            -1 => None,
            fence => Some(fence),
        })
    }

    /// Receive a [`HardwareBuffer`] from an `AF_UNIX` socket.
    ///
    /// `AF_UNIX` sockets are wrapped by [`std::os::unix::net::UnixListener`] in Rust.
    pub fn recv_handle_from_unix_socket(socket_fd: RawFd) -> Result<Self> {
        unsafe {
            let ptr =
                construct(|res| ffi::AHardwareBuffer_recvHandleFromUnixSocket(socket_fd, res))?;

            Ok(Self::from_ptr(NonNull::new_unchecked(ptr)))
        }
    }

    /// Send the [`HardwareBuffer`] to an `AF_UNIX` socket.
    ///
    /// `AF_UNIX` sockets are wrapped by [`std::os::unix::net::UnixListener`] in Rust.
    pub fn send_handle_to_unix_socket(&self, socket_fd: RawFd) -> Result<()> {
        let status =
            unsafe { ffi::AHardwareBuffer_sendHandleToUnixSocket(self.as_ptr(), socket_fd) };
        status_to_io_result(status, ())
    }

    /// Acquire a reference on the given [`HardwareBuffer`] object.
    ///
    /// This prevents the object from being deleted until the last strong reference, represented
    /// by [`HardwareBufferRef`], is [`drop()`]ped.
    pub fn acquire(&self) -> HardwareBufferRef {
        unsafe {
            ffi::AHardwareBuffer_acquire(self.as_ptr());
            HardwareBufferRef::from_ptr(self.inner)
        }
    }
}

/// A [`HardwareBuffer`] with an owned reference, that is released when dropped.
/// It behaves much like a strong [`std::rc::Rc`] reference.
#[derive(Debug)]
pub struct HardwareBufferRef {
    inner: HardwareBuffer,
}

impl HardwareBufferRef {
    /// Create an _owned_ [`HardwareBuffer`] from a native pointer
    ///
    /// To wrap a weak reference (that is **not** `release`d on [`Drop`]), call
    /// [`HardwareBuffer::from_ptr()`] instead.
    ///
    /// # Safety
    /// By calling this function, you assert that it is a valid pointer to an NDK
    /// [`ffi::AHardwareBuffer`].
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AHardwareBuffer>) -> Self {
        Self {
            inner: HardwareBuffer { inner: ptr },
        }
    }
}

impl Deref for HardwareBufferRef {
    type Target = HardwareBuffer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for HardwareBufferRef {
    fn drop(&mut self) {
        unsafe { ffi::AHardwareBuffer_release(self.inner.as_ptr()) }
    }
}

impl Clone for HardwareBufferRef {
    fn clone(&self) -> Self {
        self.acquire()
    }
}

/// Buffer description.
///
/// Used for allocating new buffers and querying parameters of existing ones.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HardwareBufferDesc {
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    pub format: HardwareBufferFormat,
    pub usage: HardwareBufferUsage,
    pub stride: u32,
}

impl HardwareBufferDesc {
    fn into_native(self) -> ffi::AHardwareBuffer_Desc {
        ffi::AHardwareBuffer_Desc {
            width: self.width,
            height: self.height,
            layers: self.layers,
            format: self.format.try_into().unwrap(),
            usage: self.usage.0 .0,
            stride: self.stride,
            rfu0: 0,
            rfu1: 0,
        }
    }
}

/// A native [`AHardwareBuffer_Plane`]
///
/// Contains the same fields as [`ffi::AHardwareBuffer_Plane`].
///
/// [`AHardwareBuffer_Plane`]: https://developer.android.com/ndk/reference/group/a-hardware-buffer#ahardwarebuffer_plane
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LockedPlaneInfo {
    pub virtual_address: *mut c_void,
    pub bytes_per_pixel: u32,
    pub bytes_per_stride: u32,
}

/// Iterator over [`ffi::AHardwareBuffer_Planes`], containing a list of [`LockedPlaneInfo`].
#[derive(Debug)]
pub struct HardwareBufferPlanes {
    inner: ffi::AHardwareBuffer_Planes,
    index: u32,
}

impl Iterator for HardwareBufferPlanes {
    type Item = LockedPlaneInfo;

    fn next(&mut self) -> Option<LockedPlaneInfo> {
        if self.index == self.inner.planeCount {
            None
        } else {
            let plane = self.inner.planes[self.index as usize];
            self.index += 1;
            Some(LockedPlaneInfo {
                virtual_address: plane.data,
                bytes_per_pixel: plane.pixelStride,
                bytes_per_stride: plane.rowStride,
            })
        }
    }
}
