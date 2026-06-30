// boxart.rs — auto-download N64 box art / media from the libretro thumbnails server.
//
// Matching is CRC-primary: the ROM's CRC32 (No-Intro) maps to the exact game
// name via the libretro DAT; ROMs not in the DAT (homebrew, hacks, bad dumps)
// fall back to a fuzzy filename match against the box-art name index. Both
// indexes are fetched once and cached on disk.
//
// Box art / snaps / title screens live on disk under the cache dir — the disk IS
// the cache, so this module holds no in-memory state (lock-free; nothing is
// shared across threads). The only perf-cache (ROM path -> CRC, to skip
// re-reading unchanged ROMs) is a plain map the caller owns and persists.
//
// Sources:
//   https://thumbnails.libretro.com/Nintendo - Nintendo 64/Named_{Boxarts,Snaps,Titles}/
//   https://raw.githubusercontent.com/libretro/libretro-database/.../Nintendo - Nintendo 64.dat
// No API key or account required.

use crate::ui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TREE_URL: &str = "https://api.github.com/repos/libretro-thumbnails/Nintendo_-_Nintendo_64/git/trees/master?recursive=1";
const THUMB_BASE: &str = "https://thumbnails.libretro.com/";
const N64_DAT_URL: &str = "https://raw.githubusercontent.com/libretro/libretro-database/master/metadat/no-intro/Nintendo - Nintendo 64.dat";

// Normalized box-art name -> candidate libretro filenames.
pub type Index = HashMap<String, Vec<String>>;
// CRC32 (No-Intro) -> canonical game name.
pub type CrcIndex = HashMap<u32, String>;
// ROM path -> (size, mtime-secs, crc32) so unchanged ROMs aren't re-read each run.
pub type CrcCache = HashMap<String, (u64, u64, u32)>;

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

fn index_cache_path() -> PathBuf {
    ui::get_dirs().cache_dir.join("n64_boxart_index.json")
}

fn crc_index_cache_path() -> PathBuf {
    ui::get_dirs().cache_dir.join("n64_crc_index.json")
}

fn crc_cache_path() -> PathBuf {
    ui::get_dirs().cache_dir.join("n64_rom_crc.json")
}

// ---- filename matching (fallback for ROMs absent from the No-Intro DAT) ----

// Drop extension + region/tag groups, lowercase, drop articles, keep [a-z0-9].
fn normalize(name: &str) -> String {
    let mut s = name
        .rsplit_once('.')
        .map(|(a, _)| a.to_string())
        .unwrap_or_else(|| name.to_string());
    if let Some(i) = s.find(" (") {
        s.truncate(i);
    }
    if let Some(i) = s.find(" [") {
        s.truncate(i);
    }
    s.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty() && !matches!(*t, "the" | "a" | "an"))
        .collect()
}

fn region_rank(s: &str) -> u8 {
    if s.contains("(USA") {
        0
    } else if s.contains("(World") {
        1
    } else if s.contains("(Europe") {
        2
    } else if s.contains("(Japan") {
        3
    } else {
        4
    }
}

fn is_variant(s: &str) -> bool {
    const BAD: [&str; 12] = [
        "(Beta",
        "(Proto",
        "(Demo",
        "(Debug",
        "(Kiosk",
        "(Sample",
        "(Pirate",
        "(Promo",
        "(Competition",
        "(Aftermarket",
        "(Unl",
        "(Overdump",
    ];
    BAD.iter().any(|b| s.contains(b))
}

// Best candidate filename: prefer clean retail, then region, then shortest.
fn pick(cands: &[String]) -> Option<&str> {
    let clean: Vec<&String> = cands.iter().filter(|c| !is_variant(c)).collect();
    let pool: Vec<&String> = if clean.is_empty() {
        cands.iter().collect()
    } else {
        clean
    };
    pool.into_iter()
        .min_by(|a, b| {
            region_rank(a)
                .cmp(&region_rank(b))
                .then(a.len().cmp(&b.len()))
        })
        .map(|s| s.as_str())
}

fn match_name(index: &Index, rom_name: &str) -> Option<String> {
    let key = normalize(rom_name);
    if key.is_empty() {
        return None;
    }
    if let Some(cands) = index.get(&key)
        && let Some(p) = pick(cands)
    {
        return Some(p.to_string());
    }
    // guarded substring fallback (aliases like "Zelda Ocarina of Time")
    if key.len() >= 8 {
        let mut best: Option<&Vec<String>> = None;
        let mut best_len = usize::MAX;
        for (ik, cands) in index {
            if (ik.contains(&key) || key.contains(ik.as_str())) && ik.len() < best_len {
                best_len = ik.len();
                best = Some(cands);
            }
        }
        if let Some(cands) = best
            && let Some(p) = pick(cands)
        {
            return Some(p.to_string());
        }
    }
    None
}

fn build_index(names: Vec<String>) -> Index {
    let mut map: Index = HashMap::new();
    for n in names {
        map.entry(normalize(&n)).or_default().push(n);
    }
    map
}

/// Load the N64 box-art name index (cached on disk, else fetched from GitHub).
pub async fn load_index() -> Index {
    if let Ok(bytes) = tokio::fs::read(index_cache_path()).await
        && let Ok(names) = serde_json::from_slice::<Vec<String>>(&bytes)
        && !names.is_empty()
    {
        return build_index(names);
    }
    let mut names = Vec::new();
    if let Ok(resp) = ui::WEB_CLIENT.get(TREE_URL).send().await
        && let Ok(json) = resp.json::<serde_json::Value>().await
        && let Some(tree) = json.get("tree").and_then(|t| t.as_array())
    {
        for entry in tree {
            if let Some(path) = entry.get("path").and_then(|p| p.as_str())
                && let Some(name) = path.strip_prefix("Named_Boxarts/")
                && name.ends_with(".png")
            {
                names.push(name.to_string());
            }
        }
    }
    if !names.is_empty()
        && let Ok(bytes) = serde_json::to_vec(&names)
    {
        let _ = tokio::fs::write(index_cache_path(), bytes).await;
    }
    build_index(names)
}

// ---- CRC index (No-Intro DAT) ----

/// Parse the libretro N64 DAT (clrmamepro) into CRC32 -> No-Intro name. Cached on
/// disk; the DAT lists a separate CRC per byte-order, all sharing one game name.
/// Returns an empty map when the DAT is unavailable (matching then degrades to
/// the filename fallback and homebrew detection is skipped).
pub async fn load_crc_index() -> CrcIndex {
    if let Ok(bytes) = tokio::fs::read(crc_index_cache_path()).await
        && let Ok(map) = serde_json::from_slice::<CrcIndex>(&bytes)
        && !map.is_empty()
    {
        return map;
    }
    let mut map = CrcIndex::new();
    if let Ok(resp) = ui::WEB_CLIENT.get(N64_DAT_URL).send().await
        && let Ok(text) = resp.text().await
    {
        let mut name: Option<String> = None;
        for line in text.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("name \"")
                && let Some(n) = rest.strip_suffix('"')
            {
                name = Some(n.to_string());
            } else if t.starts_with("rom (")
                && let Some(n) = &name
                && let Some(crc) = t
                    .find(" crc ")
                    .and_then(|i| t.get(i + 5..i + 13))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
            {
                map.insert(crc, n.clone());
            }
        }
    }
    if !map.is_empty()
        && let Ok(bytes) = serde_json::to_vec(&map)
    {
        let _ = tokio::fs::write(crc_index_cache_path(), bytes).await;
    }
    map
}

// ---- CRC perf-cache (caller-owned; persisted to disk, no global state) ----

/// Load the persisted ROM-CRC cache. Empty when absent or unreadable.
pub fn load_crc_cache() -> CrcCache {
    std::fs::read(crc_cache_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Persist the ROM-CRC cache (call once after a scan completes).
pub fn save_crc_cache(cache: &CrcCache) {
    if let Ok(bytes) = serde_json::to_vec(cache) {
        let _ = std::fs::write(crc_cache_path(), bytes);
    }
}

fn file_sig(path: &str) -> Option<(u64, u64)> {
    let m = std::fs::metadata(path).ok()?;
    let mtime = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((m.len(), mtime))
}

/// The cached CRC for a ROM whose on-disk signature is unchanged. Only stats the
/// file (no whole-ROM read), so it is safe on any thread.
pub fn cached_crc(cache: &CrcCache, path: &str) -> Option<u32> {
    let (size, mtime) = file_sig(path)?;
    cache
        .get(path)
        .and_then(|&(s, t, crc)| (s == size && t == mtime).then_some(crc))
}

/// CRC32 of the emulator-normalized ROM bytes (archive-extracted + byteswapped to
/// z64) plus the file's signature for caching. Reads the WHOLE ROM, so callers
/// MUST run this inside `spawn_blocking` — never on the UI thread or a bare
/// runtime worker.
pub fn compute_crc(path: &str) -> Option<(u64, u64, u32)> {
    let (size, mtime) = file_sig(path)?;
    let bytes = crate::device::get_rom_contents(&PathBuf::from(path))?;
    Some((size, mtime, crc32fast::hash(&bytes)))
}

/// Resolve the libretro thumbnail filename for a ROM. Hash-primary: the ROM's
/// CRC32 -> exact No-Intro name; falls back to the fuzzy filename match for ROMs
/// not in the DAT. Pure — `crc` is supplied by the caller, no I/O here.
pub fn resolve_filename(
    index: &Index,
    crc_index: &CrcIndex,
    crc: Option<u32>,
    title: &str,
) -> Option<String> {
    if let Some(crc) = crc
        && let Some(name) = crc_index.get(&crc)
    {
        return Some(format!("{name}.png"));
    }
    match_name(index, title)
}

/// Whether a ROM is unofficial: it has a CRC that the (non-empty) No-Intro DAT
/// doesn't list. Returns false when the DAT or CRC is unavailable.
pub fn is_homebrew(crc_index: &CrcIndex, crc: Option<u32>) -> bool {
    !crc_index.is_empty() && crc.map(|c| !crc_index.contains_key(&c)).unwrap_or(false)
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
    match ui::WEB_CLIENT.get(url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.bytes().await {
            Ok(bytes) => tokio::fs::write(dest, &bytes).await.is_ok(),
            Err(_) => false,
        },
        _ => false,
    }
}

/// Ensure box art for `filename` is downloaded; returns the on-disk PNG path, or
/// `None` if the download failed. Network-free when already cached on disk.
pub async fn download_art(filename: &str) -> Option<PathBuf> {
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
/// Named_Titles — so no separate index is needed. Network-free when cached.
// ponytail: reuses the boxart match; add a dedicated Snaps/Titles index only if hit-rate disappoints.
pub async fn download_media(filename: &str) -> (Option<PathBuf>, Option<PathBuf>) {
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

    fn idx(names: &[&str]) -> Index {
        build_index(names.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn matching() {
        let index = idx(&[
            "Super Mario 64 (USA).png",
            "Legend of Zelda, The - Ocarina of Time (USA).png",
            "Legend of Zelda, The - Ocarina of Time (USA) (Beta).png",
            "Mario Kart 64 (Europe).png",
            "Mario Kart 64 (USA).png",
            "GoldenEye 007 (USA).png",
        ]);
        // exact
        assert_eq!(
            match_name(&index, "Super Mario 64.z64").as_deref(),
            Some("Super Mario 64 (USA).png")
        );
        // article reorder; retail preferred over Beta
        assert_eq!(
            match_name(&index, "The Legend of Zelda - Ocarina of Time (USA).z64").as_deref(),
            Some("Legend of Zelda, The - Ocarina of Time (USA).png")
        );
        // alias (substring fallback)
        assert_eq!(
            match_name(&index, "Zelda Ocarina of Time").as_deref(),
            Some("Legend of Zelda, The - Ocarina of Time (USA).png")
        );
        // region preference USA > Europe
        assert_eq!(
            match_name(&index, "Mario Kart 64").as_deref(),
            Some("Mario Kart 64 (USA).png")
        );
        assert!(match_name(&index, "Totally Fake Game 9000").is_none());
    }

    #[test]
    fn crc_primary_then_filename_fallback() {
        let index = idx(&["Super Mario 64 (USA).png", "Mario Kart 64 (USA).png"]);
        let mut crc_index = CrcIndex::new();
        crc_index.insert(0x1234_5678, "Super Mario 64 (USA)".to_string());
        // CRC hit wins, even if the title would fuzzy-match something else.
        assert_eq!(
            resolve_filename(&index, &crc_index, Some(0x1234_5678), "garbage name").as_deref(),
            Some("Super Mario 64 (USA).png")
        );
        // CRC miss -> filename fallback.
        assert_eq!(
            resolve_filename(&index, &crc_index, Some(0xDEAD_BEEF), "Mario Kart 64").as_deref(),
            Some("Mario Kart 64 (USA).png")
        );
        // Unknown CRC + non-empty DAT => homebrew; known CRC => not.
        assert!(is_homebrew(&crc_index, Some(0xDEAD_BEEF)));
        assert!(!is_homebrew(&crc_index, Some(0x1234_5678)));
        // No DAT => never flag homebrew.
        assert!(!is_homebrew(&CrcIndex::new(), Some(0xDEAD_BEEF)));
    }

    // Hits the network; run explicitly: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn network_download() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let index = load_index().await;
            assert!(!index.is_empty(), "index empty");
            let filename =
                resolve_filename(&index, &CrcIndex::new(), None, "Super Mario 64").unwrap();
            assert!(
                download_art(&filename).await.is_some(),
                "art download failed"
            );
        });
    }

    // Hits the network; run explicitly: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn network_media() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let index = load_index().await;
            assert!(!index.is_empty(), "index empty");
            let filename =
                resolve_filename(&index, &CrcIndex::new(), None, "Super Mario 64").unwrap();
            // Super Mario 64 has both a snap and a title screen in the libretro N64 repo.
            let (snap, title) = download_media(&filename).await;
            assert!(snap.is_some(), "snap not downloaded");
            assert!(title.is_some(), "title not downloaded");
        });
    }

    // Hits the network; run explicitly: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn network_crc() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let map = load_crc_index().await;
            assert!(!map.is_empty(), "crc index empty");
            // Super Smash Bros. (USA), z64 CRC from the libretro No-Intro DAT.
            assert_eq!(
                map.get(&0xEB97929E).map(String::as_str),
                Some("Super Smash Bros. (USA)")
            );
        });
    }
}
