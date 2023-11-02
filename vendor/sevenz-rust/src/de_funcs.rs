use crate::{password::Password, Error, *};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
///
/// decompress a 7z file
/// # Example
/// ```no_run
/// sevenz_rust::decompress_file("sample.7z", "sample").expect("complete");
///
/// ```
///
#[inline]
pub fn decompress_file(src_path: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<(), Error> {
    let file = std::fs::File::open(src_path.as_ref())
        .map_err(|e| Error::file_open(e, src_path.as_ref().to_string_lossy().to_string()))?;
    decompress(file, dest)
}

#[inline]
pub fn decompress_file_with_extract_fn(
    src_path: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    extract_fn: impl FnMut(&SevenZArchiveEntry, &mut dyn Read, &PathBuf) -> Result<bool, Error>,
) -> Result<(), Error> {
    let file = std::fs::File::open(src_path.as_ref())
        .map_err(|e| Error::file_open(e, src_path.as_ref().to_string_lossy().to_string()))?;
    decompress_with_extract_fn(file, dest, extract_fn)
}

/// decompress a source reader to [dest] path
#[inline]
pub fn decompress<R: Read + Seek>(src_reader: R, dest: impl AsRef<Path>) -> Result<(), Error> {
    decompress_with_extract_fn(src_reader, dest, default_entry_extract_fn)
}

#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub fn decompress_with_extract_fn<R: Read + Seek>(
    src_reader: R,
    dest: impl AsRef<Path>,
    extract_fn: impl FnMut(&SevenZArchiveEntry, &mut dyn Read, &PathBuf) -> Result<bool, Error>,
) -> Result<(), Error> {
    decompress_impl(src_reader, dest, Password::empty(), extract_fn)
}

#[cfg(all(feature = "aes256", not(target_arch = "wasm32")))]
/// decompress a encrypted file with password
/// # Example
/// ```no_run
/// sevenz_rust::decompress_file_with_password("sample.7z", "sample", "password".into()).expect("complete");
///
/// ```
#[inline]
pub fn decompress_file_with_password(
    src_path: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    password: Password,
) -> Result<(), Error> {
    let file = std::fs::File::open(src_path.as_ref())
        .map_err(|e| Error::file_open(e, src_path.as_ref().to_string_lossy().to_string()))?;
    decompress_with_password(file, dest, password)
}

#[cfg(all(feature = "aes256", not(target_arch = "wasm32")))]
#[inline]
pub fn decompress_with_password<R: Read + Seek>(
    src_reader: R,
    dest: impl AsRef<Path>,
    password: Password,
) -> Result<(), Error> {
    decompress_impl(src_reader, dest, password, default_entry_extract_fn)
}

#[cfg(all(feature = "aes256", not(target_arch = "wasm32")))]
pub fn decompress_with_extract_fn_and_password<R: Read + Seek>(
    src_reader: R,
    dest: impl AsRef<Path>,
    password: Password,
    extract_fn: impl FnMut(&SevenZArchiveEntry, &mut dyn Read, &PathBuf) -> Result<bool, Error>,
) -> Result<(), Error> {
    decompress_impl(src_reader, dest, password, extract_fn)
}

#[cfg(not(target_arch = "wasm32"))]
fn decompress_impl<R: Read + Seek>(
    mut src_reader: R,
    dest: impl AsRef<Path>,
    password: Password,
    mut extract_fn: impl FnMut(&SevenZArchiveEntry, &mut dyn Read, &PathBuf) -> Result<bool, Error>,
) -> Result<(), Error> {
    use std::io::SeekFrom;

    let pos = src_reader.stream_position().map_err(Error::io)?;
    let len = src_reader.seek(SeekFrom::End(0)).map_err(Error::io)?;
    src_reader.seek(SeekFrom::Start(pos)).map_err(Error::io)?;
    let mut seven = SevenZReader::new(src_reader, len, password)?;
    let dest = PathBuf::from(dest.as_ref());
    if !dest.exists() {
        std::fs::create_dir_all(&dest).map_err(Error::io)?;
    }
    seven.for_each_entries(|entry, reader| {
        let dest_path = dest.join(entry.name());
        extract_fn(entry, reader, &dest_path)
    })?;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn default_entry_extract_fn(
    entry: &SevenZArchiveEntry,
    reader: &mut dyn Read,
    dest: &PathBuf,
) -> Result<bool, Error> {
    use std::{fs::File, io::BufWriter};

    if entry.is_directory() {
        let dir = dest;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(Error::io)?;
        }
    } else {
        let path = dest;
        path.parent().and_then(|p| {
            if !p.exists() {
                std::fs::create_dir_all(p).ok()
            } else {
                None
            }
        });
        let file = File::create(&path)
            .map_err(|e| Error::file_open(e, path.to_string_lossy().to_string()))?;
        if entry.size() > 0 {
            let mut writer = BufWriter::new(file);
            std::io::copy(reader, &mut writer).map_err(Error::io)?;
            ft::set_file_handle_times(
                writer.get_ref(),
                Some(ft::FileTime::from_system_time(entry.access_date().into())),
                Some(ft::FileTime::from_system_time(
                    entry.last_modified_date().into(),
                )),
                Some(ft::FileTime::from_system_time(entry.creation_date().into())),
            )
            .unwrap_or_default();
        }
    }
    Ok(true)
}
