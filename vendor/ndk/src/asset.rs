//! Bindings for [`AAsset`], [`AAssetDir`] and [`AAssetManager`]
//!
//! [`AAsset`]: https://developer.android.com/ndk/reference/group/asset#aasset
//! [`AAssetDir`]: https://developer.android.com/ndk/reference/group/asset#aassetdir
//! [`AAssetManager`]: https://developer.android.com/ndk/reference/group/asset#aassetmanager

use std::ffi::{CStr, CString};
use std::io;
use std::ptr::NonNull;

/// A native [`AAssetManager *`]
///
/// [`AAssetManager *`]: https://developer.android.com/ndk/reference/group/asset#aassetmanager
#[derive(Debug)]
pub struct AssetManager {
    ptr: NonNull<ffi::AAssetManager>,
}

// AAssetManager is thread safe.
// See https://developer.android.com/ndk/reference/group/asset#aassetmanager
unsafe impl Send for AssetManager {}
unsafe impl Sync for AssetManager {}

impl AssetManager {
    /// Create an `AssetManager` from a pointer
    ///
    /// # Safety
    /// By calling this function, you assert that the pointer is a valid pointer to a native
    /// `AAssetManager`.
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AAssetManager>) -> Self {
        Self { ptr }
    }

    /// Returns the pointer to the native `AAssetManager`.
    pub fn ptr(&self) -> NonNull<ffi::AAssetManager> {
        self.ptr
    }

    /// Open the asset. Returns `None` if opening the asset fails.
    ///
    /// This currently always opens the asset in the streaming mode.
    pub fn open(&self, filename: &CStr) -> Option<Asset> {
        unsafe {
            let ptr = ffi::AAssetManager_open(
                self.ptr.as_ptr(),
                filename.as_ptr(),
                ffi::AASSET_MODE_STREAMING as i32,
            );
            Some(Asset::from_ptr(NonNull::new(ptr)?))
        }
    }

    /// Open an asset directory. Returns `None` if opening the directory fails.
    pub fn open_dir(&self, filename: &CStr) -> Option<AssetDir> {
        unsafe {
            let ptr = ffi::AAssetManager_openDir(self.ptr.as_ptr(), filename.as_ptr());
            Some(AssetDir::from_ptr(NonNull::new(ptr)?))
        }
    }
}

/// A native [`AAssetDir *`]
///
/// ```no_run
/// # use std::ffi::CString;
/// # use ndk::asset::AssetManager;
/// # let asset_manager: AssetManager = unimplemented!();
/// use std::io::Read;
///
/// let mut my_dir = asset_manager
///     .open_dir(&CString::new("my_dir").unwrap())
///     .expect("Could not open directory");
///
/// // Use it as an iterator
/// let all_files = my_dir.collect::<Vec<CString>>();
///
/// // Reset the iterator
/// my_dir.rewind();
///
/// // Use .with_next() to iterate without allocating `CString`s
/// while let Some(asset) = my_dir.with_next(|cstr| asset_manager.open(cstr).unwrap()) {
///     let mut text = String::new();
///     asset.read_to_string(&mut text);
///     // ...
/// }
/// ```
///
/// [`AAssetDir *`]: https://developer.android.com/ndk/reference/group/asset#aassetdir
#[derive(Debug)]
pub struct AssetDir {
    ptr: NonNull<ffi::AAssetDir>,
}

// It's unclear if AAssetDir is thread safe.
// However, AAsset is not, so there's a good chance that AAssetDir is not either.

impl Drop for AssetDir {
    fn drop(&mut self) {
        unsafe { ffi::AAssetDir_close(self.ptr.as_ptr()) }
    }
}

impl AssetDir {
    /// Construct an `AssetDir` from the native `AAssetDir *`.  This gives ownership of the
    /// `AAssetDir *` to the `AssetDir`, which will handle closing the asset.  Avoid using
    /// the pointer after calling this function.
    ///
    /// # Safety
    /// By calling this function, you assert that it points to a valid native `AAssetDir`.
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AAssetDir>) -> Self {
        Self { ptr }
    }

    /// The corresponding native `AAssetDir *`
    pub fn ptr(&self) -> NonNull<ffi::AAssetDir> {
        self.ptr
    }

    /// Get the next filename, if any, and process it.  Like `.next()`, but performs no additional
    /// allocation.
    ///
    /// The filenames are in the correct format to be passed to `AssetManager::open`
    pub fn with_next<T>(&mut self, f: impl for<'a> FnOnce(&'a CStr) -> T) -> Option<T> {
        unsafe {
            let next_name = ffi::AAssetDir_getNextFileName(self.ptr.as_ptr());
            if next_name.is_null() {
                None
            } else {
                Some(f(CStr::from_ptr(next_name)))
            }
        }
    }

    /// Reset the iteration state
    pub fn rewind(&mut self) {
        unsafe {
            ffi::AAssetDir_rewind(self.ptr.as_ptr());
        }
    }
}

impl Iterator for AssetDir {
    type Item = CString;

    fn next(&mut self) -> Option<CString> {
        self.with_next(|cstr| cstr.to_owned())
    }
}

/// A native [`AAsset *`], opened in streaming mode
///
/// ```no_run
/// # use std::ffi::CString;
/// # use ndk::asset::AssetManager;
/// # let asset_manager: AssetManager = unimplemented!();
/// use std::io::Read;
///
/// let asset = asset_manager
///     .open(&CString::new("path/to/asset").unwrap())
///     .expect("Could not open asset");
///
/// let mut data = vec![];
/// asset.read_to_end(&mut data);
/// // ... use data ...
/// ```
///
/// [`AAsset *`]: https://developer.android.com/ndk/reference/group/asset#aasset
#[derive(Debug)]
pub struct Asset {
    ptr: NonNull<ffi::AAsset>,
}

// AAsset is *not* thread safe.
// See https://developer.android.com/ndk/reference/group/asset#aasset

impl Drop for Asset {
    fn drop(&mut self) {
        unsafe { ffi::AAsset_close(self.ptr.as_ptr()) }
    }
}

impl Asset {
    /// Construct an `Asset` from the native `AAsset *`.  This gives ownership of the `AAsset *` to
    /// the `Asset`, which will handle closing the asset.  Avoid using the pointer after calling
    /// this function.
    ///
    /// # Safety
    /// By calling this function, you assert that it points to a valid native `AAsset`, open
    /// in the streaming mode.
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AAsset>) -> Self {
        Self { ptr }
    }

    /// The corresponding native `AAsset *`
    pub fn ptr(&self) -> NonNull<ffi::AAsset> {
        self.ptr
    }

    /// Returns the total length of the asset, in bytes
    pub fn get_length(&self) -> usize {
        unsafe { ffi::AAsset_getLength64(self.ptr.as_ptr()) as usize }
    }

    /// Returns the remaining length of the asset, in bytes
    pub fn get_remaining_length(&self) -> usize {
        unsafe { ffi::AAsset_getRemainingLength64(self.ptr.as_ptr()) as usize }
    }

    /// Reads all data into a buffer and returns it
    pub fn get_buffer(&mut self) -> io::Result<&[u8]> {
        unsafe {
            let buf_ptr = ffi::AAsset_getBuffer(self.ptr.as_ptr());
            if buf_ptr.is_null() {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Android Asset error creating buffer",
                ))
            } else {
                Ok(std::slice::from_raw_parts(
                    buf_ptr as *const u8,
                    self.get_remaining_length(),
                ))
            }
        }
    }

    //pub fn open_file_descriptor(&self) -> TODO
}

impl io::Read for Asset {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let res = ffi::AAsset_read(
                self.ptr.as_ptr(),
                buf.as_mut_ptr() as *mut _,
                buf.len() as ffi::size_t,
            );
            if res >= 0 {
                Ok(res as usize)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Android Asset read error",
                ))
            }
        }
    }
}

impl io::Seek for Asset {
    fn seek(&mut self, seek: io::SeekFrom) -> io::Result<u64> {
        unsafe {
            let res = match seek {
                io::SeekFrom::Start(x) => {
                    ffi::AAsset_seek64(self.ptr.as_ptr(), x as i64, ffi::SEEK_SET as i32)
                }
                io::SeekFrom::Current(x) => {
                    ffi::AAsset_seek64(self.ptr.as_ptr(), x, ffi::SEEK_CUR as i32)
                }
                io::SeekFrom::End(x) => {
                    ffi::AAsset_seek64(self.ptr.as_ptr(), x, ffi::SEEK_END as i32)
                }
            };
            if res < 0 {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Android Asset seek error",
                ))
            } else {
                Ok(res as u64)
            }
        }
    }
}
