//! Timestamps for files in Rust
//!
//! An enhanced version of [filetime](https://docs.rs/filetime), which can set file creation time on Windows.
//!
//! Internally, this crate use [SetFileTime](https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfiletime)
//! Win32 API to set the file creation time on Windows.
//!
//! On other platforms, all functions will just call the corresponding [filetime](https://docs.rs/filetime)'s function, and
//! ignore the file creation time.
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! filetime_creation = "0.1"
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use std::fs;
//! use filetime_creation::{FileTime, set_file_ctime};
//!
//! let now = FileTime::now();
//!
//! set_file_ctime("test.txt", now);
//! assert_eq!(now, FileTime::from(fs::metadata("test.txt").unwrap().created().unwrap()));
//! ```

pub use filetime::{set_file_atime, set_file_mtime, FileTime};

use std::fs;
use std::io;
use std::path::Path;

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        use fs::OpenOptions;
        use std::ptr;
        use std::os::windows::prelude::*;
        use windows_sys::Win32::Foundation::{FILETIME, HANDLE};
        use windows_sys::Win32::Storage::FileSystem::*;
    }
}

/// Set the last access, modification, and creation times for a file on the filesystem.
#[cfg(windows)]
pub fn set_file_times<P>(p: P, atime: FileTime, mtime: FileTime, ctime: FileTime) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let f = OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(p)?;
    set_file_handle_times(&f, Some(atime), Some(mtime), Some(ctime))
}

/// Set the creation time for a file on Windows, returning any error encountered.
///
/// # Platform support
///
/// This function is only supported on Windows, other platforms will do nothing
/// and return Err.
#[cfg(windows)]
pub fn set_file_ctime<P>(p: P, ctime: FileTime) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let f = OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(p)?;
    set_file_handle_times(&f, None, None, Some(ctime))
}

/// Set the last access, modification and creation times for a file handle.
#[cfg(windows)]
pub fn set_file_handle_times(
    f: &fs::File,
    atime: Option<FileTime>,
    mtime: Option<FileTime>,
    ctime: Option<FileTime>,
) -> io::Result<()> {
    let atime = atime.map(to_filetime);
    let mtime = mtime.map(to_filetime);
    let ctime = ctime.map(to_filetime);
    return unsafe {
        let ret = SetFileTime(
            f.as_raw_handle() as HANDLE,
            ctime
                .as_ref()
                .map(|p| p as *const FILETIME)
                .unwrap_or(ptr::null()),
            atime
                .as_ref()
                .map(|p| p as *const FILETIME)
                .unwrap_or(ptr::null()),
            mtime
                .as_ref()
                .map(|p| p as *const FILETIME)
                .unwrap_or(ptr::null()),
        );
        if ret != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    };

    fn to_filetime(ft: FileTime) -> FILETIME {
        let intervals = ft.seconds() * (1_000_000_000 / 100) + ((ft.nanoseconds() as i64) / 100);
        FILETIME {
            dwLowDateTime: intervals as u32,
            dwHighDateTime: (intervals >> 32) as u32,
        }
    }
}

/// Set the last access, modification and creation times for a file on the filesystem.
/// This function does not follow symlink.
#[cfg(windows)]
pub fn set_symlink_file_times<P>(
    p: P,
    atime: FileTime,
    mtime: FileTime,
    ctime: FileTime,
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let f = OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(p)?;
    set_file_handle_times(&f, Some(atime), Some(mtime), Some(ctime))
}

/// Set the last access, modification, and creation times for a file on the filesystem.
#[cfg(not(windows))]
pub fn set_file_times<P>(p: P, atime: FileTime, mtime: FileTime, _ctime: FileTime) -> io::Result<()>
where
    P: AsRef<Path>,
{
    filetime::set_file_times(p, atime, mtime)
}

/// Set the creation time for a file on Windows, returning any error encountered.
///
/// # Platform support
///
/// This function is only supported on Windows, other platforms will do nothing
/// and return Err.
#[cfg(not(windows))]
pub fn set_file_ctime<P>(_p: P, _ctime: FileTime) -> io::Result<()>
where
    P: AsRef<Path>,
{
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Platform unsupported",
    ))
}

/// Set the last access, modification and creation times for a file handle.
#[cfg(not(windows))]
pub fn set_file_handle_times(
    f: &fs::File,
    atime: Option<FileTime>,
    mtime: Option<FileTime>,
    _ctime: Option<FileTime>,
) -> io::Result<()> {
    filetime::set_file_handle_times(f, atime, mtime)
}

/// Set the last access, modification and creation times for a file on the filesystem.
/// This function does not follow symlink.
#[cfg(not(windows))]
pub fn set_symlink_file_times<P>(
    p: P,
    atime: FileTime,
    mtime: FileTime,
    _ctime: FileTime,
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    filetime::set_symlink_file_times(p, atime, mtime)
}

#[cfg(test)]
mod tests {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            use super::*;
            use fs::File;
            use tempfile::Builder;
        }
    }

    #[cfg(windows)]
    fn make_symlink_file<P, Q>(src: P, dst: Q) -> io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        use std::os::windows::fs::symlink_file;
        symlink_file(src, dst)
    }

    #[cfg(windows)]
    fn make_symlink_dir<P, Q>(src: P, dst: Q) -> io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        use std::os::windows::fs::symlink_dir;
        symlink_dir(src, dst)
    }

    #[test]
    #[cfg(windows)]
    fn set_file_times_test() -> io::Result<()> {
        let td = Builder::new().prefix("filetime").tempdir()?;
        let path = td.path().join("foo.txt");
        let mut f = File::create(&path)?;

        let metadata = fs::metadata(&path)?;
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        set_file_times(&path, atime, mtime, ctime)?;

        let new_ctime = FileTime::from_unix_time(10_000, 0);
        set_file_times(&path, atime, mtime, new_ctime)?;

        let metadata = fs::metadata(&path)?;
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation should be updated");

        // Update just mtime
        let new_mtime = FileTime::from_unix_time(20_000, 0);
        set_file_handle_times(&mut f, None, Some(new_mtime), None)?;
        let metadata = f.metadata()?;
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime, "modification time should be updated");
        let new_atime = FileTime::from_last_access_time(&metadata);
        assert_eq!(atime, new_atime, "accessed time should not be updated");
        let new_ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation time should not be updated");

        // Update just atime
        let new_atime = FileTime::from_unix_time(30_000, 0);
        set_file_handle_times(&mut f, Some(new_atime), None, None)?;
        let metadata = f.metadata()?;
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime, "modification time should not be updated");
        let atime = FileTime::from_last_access_time(&metadata);
        assert_eq!(atime, new_atime, "accessed time should be updated");
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation time should not be updated");

        // Update just ctime
        let new_ctime = FileTime::from_unix_time(40_000, 0);
        set_file_handle_times(&mut f, None, None, Some(new_ctime))?;
        let metadata = f.metadata()?;
        let new_mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime, "modification time should not be updated");
        let new_atime = FileTime::from_last_access_time(&metadata);
        assert_eq!(atime, new_atime, "accessed time should not be updated");
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation time should be updated");

        let spath = td.path().join("bar.txt");
        make_symlink_file(&path, &spath)?;
        let metadata = fs::symlink_metadata(&spath)?;
        let sctime = FileTime::from_creation_time(&metadata).unwrap();

        set_file_times(&spath, atime, mtime, ctime)?;

        let metadata = fs::metadata(&path)?;
        let cur_ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, cur_ctime);

        let metadata = fs::symlink_metadata(&spath)?;
        let cur_ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(sctime, cur_ctime);

        set_file_times(&spath, atime, mtime, new_ctime)?;

        let metadata = fs::metadata(&path)?;
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let metadata = fs::symlink_metadata(&spath)?;
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, sctime);
        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn set_dir_times_test() -> io::Result<()> {
        let td = Builder::new().prefix("filetime").tempdir()?;
        let path = td.path().join("foo");
        fs::create_dir(&path)?;

        let metadata = fs::metadata(&path)?;
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        set_file_times(&path, atime, mtime, ctime)?;

        let new_ctime = FileTime::from_unix_time(10_000, 0);
        set_file_times(&path, atime, mtime, new_ctime)?;

        let metadata = fs::metadata(&path)?;
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation should be updated");

        // Update just mtime
        let new_mtime = FileTime::from_unix_time(20_000, 0);
        set_file_mtime(&path, new_mtime)?;
        let metadata = fs::metadata(&path)?;
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime, "modification time should be updated");
        let new_atime = FileTime::from_last_access_time(&metadata);
        assert_eq!(atime, new_atime, "accessed time should not be updated");
        let new_ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation time should not be updated");

        // Update just atime
        let new_atime = FileTime::from_unix_time(30_000, 0);
        set_file_atime(&path, new_atime)?;
        let metadata = fs::metadata(&path)?;
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime, "modification time should not be updated");
        let atime = FileTime::from_last_access_time(&metadata);
        assert_eq!(atime, new_atime, "accessed time should be updated");
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation time should not be updated");

        // Update just ctime
        let new_ctime = FileTime::from_unix_time(40_000, 0);
        set_file_ctime(&path, new_ctime)?;
        let metadata = fs::metadata(&path)?;
        let new_mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime, "modification time should not be updated");
        let new_atime = FileTime::from_last_access_time(&metadata);
        assert_eq!(atime, new_atime, "accessed time should not be updated");
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation time should be updated");

        let spath = td.path().join("bar");
        make_symlink_dir(&path, &spath)?;
        let metadata = fs::symlink_metadata(&spath)?;
        let sctime = FileTime::from_creation_time(&metadata).unwrap();

        set_file_times(&spath, atime, mtime, ctime)?;

        let metadata = fs::metadata(&path)?;
        let cur_ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, cur_ctime);

        let metadata = fs::symlink_metadata(&spath)?;
        let cur_ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(sctime, cur_ctime);

        set_file_times(&spath, atime, mtime, new_ctime)?;

        let metadata = fs::metadata(&path)?;
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let metadata = fs::symlink_metadata(&spath)?;
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, sctime);
        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn set_file_times_pre_unix_epoch_test() {
        let td = Builder::new().prefix("filetime").tempdir().unwrap();
        let path = td.path().join("foo.txt");
        File::create(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        set_file_times(&path, atime, mtime, ctime).unwrap();

        let new_ctime = FileTime::from_unix_time(-10_000, 0);
        set_file_times(&path, atime, mtime, new_ctime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);
    }

    #[test]
    #[cfg(windows)]
    fn set_file_times_pre_windows_epoch_test() {
        let td = Builder::new().prefix("filetime").tempdir().unwrap();
        let path = td.path().join("foo.txt");
        File::create(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        set_file_times(&path, atime, mtime, ctime).unwrap();

        let new_ctime = FileTime::from_unix_time(-12_000_000_000, 0);
        assert!(set_file_times(&path, atime, mtime, new_ctime).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn set_symlink_file_times_test() {
        let td = Builder::new().prefix("filetime").tempdir().unwrap();
        let path = td.path().join("foo.txt");
        File::create(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        set_symlink_file_times(&path, atime, mtime, ctime).unwrap();

        let new_ctime = FileTime::from_unix_time(10_000, 0);
        set_symlink_file_times(&path, atime, mtime, new_ctime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let spath = td.path().join("bar.txt");
        make_symlink_file(&path, &spath).unwrap();

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let smtime = FileTime::from_last_modification_time(&metadata);
        let satime = FileTime::from_last_access_time(&metadata);
        let sctime = FileTime::from_creation_time(&metadata).unwrap();
        set_symlink_file_times(&spath, smtime, satime, sctime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let new_sctime = FileTime::from_unix_time(20_000, 0);
        set_symlink_file_times(&spath, atime, mtime, new_sctime).unwrap();

        let metadata = fs::metadata(&spath).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_sctime);
    }

    #[test]
    #[cfg(windows)]
    fn set_symlink_dir_times_test() {
        let td = Builder::new().prefix("filetime").tempdir().unwrap();
        let path = td.path().join("foo");
        fs::create_dir(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        set_symlink_file_times(&path, atime, mtime, ctime).unwrap();

        let new_ctime = FileTime::from_unix_time(10_000, 0);
        set_symlink_file_times(&path, atime, mtime, new_ctime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let spath = td.path().join("bar");
        make_symlink_dir(&path, &spath).unwrap();

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let smtime = FileTime::from_last_modification_time(&metadata);
        let satime = FileTime::from_last_access_time(&metadata);
        let sctime = FileTime::from_creation_time(&metadata).unwrap();
        set_symlink_file_times(&spath, smtime, satime, sctime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let new_sctime = FileTime::from_unix_time(20_000, 0);
        set_symlink_file_times(&spath, atime, mtime, new_sctime).unwrap();

        let metadata = fs::metadata(&spath).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime);

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_sctime);
    }

    #[test]
    #[cfg(windows)]
    fn set_single_time_test() {
        use super::{set_file_atime, set_file_ctime, set_file_mtime};

        let td = Builder::new().prefix("filetime").tempdir().unwrap();
        let path = td.path().join("foo.txt");
        File::create(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        set_file_times(&path, atime, mtime, ctime).unwrap();

        let new_mtime = FileTime::from_unix_time(10_000, 0);
        set_file_mtime(&path, new_mtime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime, "modification time should be updated");
        assert_eq!(
            atime,
            FileTime::from_last_access_time(&metadata),
            "access time should not be updated",
        );
        assert_eq!(
            ctime,
            FileTime::from_creation_time(&metadata).unwrap(),
            "creation time should not be updated",
        );

        let new_atime = FileTime::from_unix_time(20_000, 0);
        set_file_atime(&path, new_atime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let atime = FileTime::from_last_access_time(&metadata);
        assert_eq!(atime, new_atime, "access time should be updated");
        assert_eq!(
            mtime,
            FileTime::from_last_modification_time(&metadata),
            "modification time should not be updated"
        );
        assert_eq!(
            ctime,
            FileTime::from_creation_time(&metadata).unwrap(),
            "creation time should not be updated",
        );

        let new_ctime = FileTime::from_unix_time(30_000, 0);
        set_file_ctime(&path, new_ctime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let ctime = FileTime::from_creation_time(&metadata).unwrap();
        assert_eq!(ctime, new_ctime, "creation time should be updated");
        assert_eq!(
            mtime,
            FileTime::from_last_modification_time(&metadata),
            "modification time should not be updated"
        );
        assert_eq!(
            atime,
            FileTime::from_last_access_time(&metadata),
            "access time should not be updated",
        );
    }
}
