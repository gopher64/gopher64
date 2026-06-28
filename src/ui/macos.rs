// macos.rs — security-scoped bookmarks for the macOS App Sandbox.
//
// Under the sandbox, a path the user picks via the open panel is only accessible
// for that session; to reach it again next launch the app must persist a
// *security-scoped bookmark* and resolve it on startup. `rfd` doesn't expose
// this, so we keep a bookmark for the scanned ROM folder in the config and
// restore it here. Files *inside* that folder (the scanned library and recent
// ROMs under it) are covered by the folder's scope.
//
// Backed by src/ui/macos_bookmark.m (compiled by build.rs, macOS-only).
//
// NOTE: authored without a macOS toolchain on hand — the FFI needs a
// `cargo build` on a Mac to confirm. Every other platform excludes this module.

use crate::ui;
use parking_lot::Mutex;
use std::path::PathBuf;

unsafe extern "C" {
    fn gopher64_bookmark_create(path: *const std::os::raw::c_char, out_len: *mut usize) -> *mut u8;
    fn gopher64_bookmark_resolve(
        bytes: *const u8,
        len: usize,
        out_stale: *mut std::os::raw::c_int,
    ) -> *mut std::os::raw::c_char;
    fn gopher64_bookmark_free(buf: *mut u8);
    fn gopher64_string_free(s: *mut std::os::raw::c_char);
}

// The active rom_dir bookmark, held so `save_settings` can re-persist it on every
// config write (not just when the folder is re-picked).
static ROM_DIR_BOOKMARK: Mutex<Option<Vec<u8>>> = Mutex::new(None);

/// Create a security-scoped bookmark for a just-picked folder and remember it.
/// Must be called right after the open panel grants access to `path`. Replaces
/// the stored bookmark unconditionally — on failure it clears it, so a new
/// rom_dir is never paired with a stale bookmark on the next config write.
pub fn remember_rom_dir(path: &str) {
    *ROM_DIR_BOOKMARK.lock() = create(path);
}

/// The current rom_dir bookmark, for persisting into `Config`.
pub fn rom_dir_bookmark() -> Option<Vec<u8>> {
    ROM_DIR_BOOKMARK.lock().clone()
}

/// Startup: seed the in-memory bookmark from `config`, resolve it (which opens the
/// security scope for the process lifetime), and rewrite `config.rom_dir` to the
/// resolved path so the folder scan can read it.
pub fn restore(config: &mut ui::config::Config) {
    let Some(bytes) = config.macos_rom_dir_bookmark.clone() else {
        return;
    };
    match resolve(&bytes) {
        Some((path, stale)) => {
            // A stale (but resolvable) bookmark still works this session; recreate
            // it from the now-accessible resolved path so it doesn't degrade.
            let fresh = if stale {
                create(&path.to_string_lossy()).or(Some(bytes))
            } else {
                Some(bytes)
            };
            *ROM_DIR_BOOKMARK.lock() = fresh.clone();
            config.macos_rom_dir_bookmark = fresh;
            config.rom_dir = path;
        }
        None => {
            // Couldn't resolve (folder moved/deleted/unmounted): drop the dead
            // bookmark so it's never re-persisted; the user re-picks to restore
            // access, which mints a fresh bookmark via remember_rom_dir.
            *ROM_DIR_BOOKMARK.lock() = None;
            config.macos_rom_dir_bookmark = None;
        }
    }
}

fn create(path: &str) -> Option<Vec<u8>> {
    let c = std::ffi::CString::new(path).ok()?;
    let mut len: usize = 0;
    unsafe {
        let buf = gopher64_bookmark_create(c.as_ptr(), &mut len);
        if buf.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(buf, len).to_vec();
        gopher64_bookmark_free(buf);
        Some(bytes)
    }
}

/// Resolve a bookmark and begin accessing the security-scoped resource. The scope
/// is intentionally held open for the process lifetime (released by the OS on
/// exit), matching how the folder is used throughout the session.
fn resolve(bytes: &[u8]) -> Option<(PathBuf, bool)> {
    let mut stale: std::os::raw::c_int = 0;
    unsafe {
        let p = gopher64_bookmark_resolve(bytes.as_ptr(), bytes.len(), &mut stale);
        if p.is_null() {
            return None;
        }
        let path = std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned();
        gopher64_string_free(p);
        Some((PathBuf::from(path), stale != 0))
    }
}
