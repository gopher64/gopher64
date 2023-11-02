#![doc = include_str!("../README.md")]
#![doc(test(attr(
    warn(unused),
    deny(warnings),
    // W/o this, we seem to get some bogus warning about `extern crate ..`.
    allow(unused_extern_crates),
)))]

use std::path::PathBuf;

/// Get the path of the current user's home directory.
///
/// See the library documentation for more information.
pub fn home_dir() -> Option<PathBuf> {
    match std::env::var("HOME") {
        Ok(home) => Some(home.into()),
        Err(_) => {
            #[cfg(unix)]
            {
                unix::home_dir()
            }

            #[cfg(windows)]
            {
                win32::home_dir()
            }
        }
    }
}

#[cfg(unix)]
mod unix {
    use nix::unistd::{Uid, User};
    use std::path::PathBuf;

    pub(super) fn home_dir() -> Option<PathBuf> {
        let uid = Uid::effective();

        User::from_uid(uid).ok().flatten().map(|u| u.dir)
    }
}

#[cfg(windows)]
mod win32 {
    use std::{path::PathBuf, ptr};

    use winapi::{
        shared::winerror::S_OK,
        um::{
            combaseapi::CoTaskMemFree, knownfolders::FOLDERID_Profile, shlobj::SHGetKnownFolderPath,
        },
    };

    pub(super) fn home_dir() -> Option<PathBuf> {
        let mut psz_path = ptr::null_mut();
        let res = unsafe {
            SHGetKnownFolderPath(
                &FOLDERID_Profile,
                0,
                ptr::null_mut(),
                &mut psz_path as *mut _,
            )
        };
        if res != S_OK {
            return None;
        }

        // Determine the length of the UTF-16 string.
        let mut len = 0;
        // SAFETY: `psz_path` guaranteed to be a valid pointer to a null-terminated UTF-16 string.
        while unsafe { *(psz_path as *const u16).offset(len) } != 0 {
            len += 1;
        }
        let slice = unsafe { std::slice::from_raw_parts(psz_path, len as usize) };
        let path = String::from_utf16(slice).ok()?;
        unsafe {
            CoTaskMemFree(psz_path as *mut _);
        }

        Some(PathBuf::from(path))
    }
}
