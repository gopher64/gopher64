// boxart.rs — auto-download N64 box art / media from the libretro thumbnails server.
//
// libretro's thumbnail filenames ARE the No-Intro game names with a fixed 1:1
// character escaping, so there is no matching to do: the caller hands us the
// No-Intro name (resolved via `ui::gui::get_nointro_name` — ROM hash -> name,
// header-name fallback), we escape it the way libretro does, and request
// "<name>.png" directly. No DAT, no CRC, no filename index, no fuzzy guessing.
//
// Box art / snaps / title screens live on disk under the cache dir — the disk IS
// the cache, so this module holds no in-memory state (lock-free; nothing is
// shared across threads).
//
// Source:
//   https://thumbnails.libretro.com/Nintendo - Nintendo 64/Named_{Boxarts,Snaps,Titles}/
// No API key or account required.

use crate::ui;
use std::path::{Path, PathBuf};

const THUMB_BASE: &str = "https://thumbnails.libretro.com/";

const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

fn boxart_dir() -> PathBuf {
    let dir = ui::get_dirs().cache_dir.join("boxart");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn media_dir(sub: &str) -> PathBuf {
    let dir = boxart_dir().join(sub);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// Reject remote-derived names that could escape the cache dir via a path separator.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.contains('\\')
}

/// The libretro thumbnail filename for a No-Intro game name. libretro replaces a
/// fixed set of filesystem/URL-unsafe characters with '_'; this is a deterministic
/// 1:1 transform, NOT a fuzzy match. Returns "<escaped name>.png".
pub fn escaped_filename(name: &str) -> String {
    let escaped: String = name
        .chars()
        .map(|c| {
            if matches!(
                c,
                '&' | '*' | '/' | ':' | '`' | '<' | '>' | '?' | '\\' | '|' | '"'
            ) {
                '_'
            } else {
                c
            }
        })
        .collect();
    format!("{escaped}.png")
}

async fn download(category: &str, filename: &str, dest: &Path) -> bool {
    let Ok(mut url) = reqwest::Url::parse(THUMB_BASE) else {
        return false;
    };
    if let Ok(mut segs) = url.path_segments_mut() {
        segs.pop_if_empty()
            .extend(["Nintendo - Nintendo 64", category, filename]);
    } else {
        return false;
    }
    match ui::WEB_CLIENT.get(url).timeout(HTTP_TIMEOUT).send().await {
        Ok(resp) if resp.status().is_success() => match resp.bytes().await {
            // Atomic: write a temp sibling then rename, so a crash mid-write can't
            // leave a truncated PNG that try_exists() would serve as valid forever.
            Ok(bytes) => {
                let tmp = dest.with_extension("part");
                tokio::fs::write(&tmp, &bytes).await.is_ok()
                    && tokio::fs::rename(&tmp, dest).await.is_ok()
            }
            Err(_) => false,
        },
        _ => false,
    }
}

/// Ensure box art for `filename` is downloaded; returns the on-disk PNG path, or
/// `None` if the download failed / 404. Network-free when already cached on disk.
pub async fn download_art(filename: &str) -> Option<PathBuf> {
    if !is_safe_name(filename) {
        return None;
    }
    let dest = boxart_dir().join(filename);
    if tokio::fs::try_exists(&dest).await.unwrap_or(false) {
        return Some(dest);
    }
    download("Named_Boxarts", filename, &dest)
        .await
        .then_some(dest)
}

/// Best-effort download of a ROM's gameplay snap + title screen, returning their
/// on-disk paths (`None` each when missing / 404). Reuses the box-art filename —
/// libretro uses the identical No-Intro name across Named_Boxarts / Named_Snaps /
/// Named_Titles. Network-free when cached.
pub async fn download_media(filename: &str) -> (Option<PathBuf>, Option<PathBuf>) {
    if !is_safe_name(filename) {
        return (None, None);
    }
    let snap_dest = media_dir("snap").join(filename);
    let snap = if tokio::fs::try_exists(&snap_dest).await.unwrap_or(false)
        || download("Named_Snaps", filename, &snap_dest).await
    {
        Some(snap_dest)
    } else {
        None
    };
    let title_dest = media_dir("title").join(filename);
    let title = if tokio::fs::try_exists(&title_dest).await.unwrap_or(false)
        || download("Named_Titles", filename, &title_dest).await
    {
        Some(title_dest)
    } else {
        None
    };
    (snap, title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_names() {
        assert!(is_safe_name("Super Mario 64 (USA).png"));
        assert!(!is_safe_name(""));
        assert!(!is_safe_name("../escape.png"));
        assert!(!is_safe_name("sub/dir.png"));
        assert!(!is_safe_name("sub\\dir.png"));
    }

    #[test]
    fn escaping_is_exact_and_safe() {
        // Common names with no special chars: just "<name>.png".
        assert_eq!(
            escaped_filename("Super Mario 64 (USA)"),
            "Super Mario 64 (USA).png"
        );
        assert_eq!(
            escaped_filename("Legend of Zelda, The - Ocarina of Time (USA)"),
            "Legend of Zelda, The - Ocarina of Time (USA).png"
        );
        // Each escaped char -> '_'.
        assert_eq!(
            escaped_filename(r#"A&B*C/D:E`F<G>H?I\J|K"L"#),
            "A_B_C_D_E_F_G_H_I_J_K_L.png"
        );
        // Path separators are escaped away, so the result is always cache-safe.
        assert!(is_safe_name(&escaped_filename("Foo/Bar: Baz")));
    }
}
