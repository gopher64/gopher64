// boxart.rs — auto-download N64 box art from the libretro thumbnails server.
//
// Source: https://thumbnails.libretro.com/Nintendo - Nintendo 64/Named_Boxarts/
// Matching: the box-art filename index (fetched once from GitHub, cached on
// disk) is keyed by a normalized name so real ROM filenames — which rarely use
// the exact No-Intro name — still resolve (handles region tags, articles, etc.).
// No API key or account required.

use crate::ui;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const TREE_URL: &str = "https://api.github.com/repos/libretro-thumbnails/Nintendo_-_Nintendo_64/git/trees/master?recursive=1";
const THUMB_BASE: &str = "https://thumbnails.libretro.com/";

pub type Index = HashMap<String, Vec<String>>;

// rom path -> downloaded box-art png path
static ART_CACHE: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// rom path -> downloaded gameplay snap / title-screen png (kept separate from
// ART_CACHE so existing box-art behavior is untouched).
static SNAP_CACHE: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static TITLE_CACHE: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn boxart_dir() -> PathBuf {
    let dir = ui::get_dirs().cache_dir.join("boxart");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn index_cache_path() -> PathBuf {
    ui::get_dirs().cache_dir.join("n64_boxart_index.json")
}

/// The already-downloaded box art for a ROM, if any. Network-free (UI thread).
pub fn art_path(rom_path: &str) -> Option<PathBuf> {
    ART_CACHE.lock().get(rom_path).cloned()
}

fn media_dir(sub: &str) -> PathBuf {
    let dir = boxart_dir().join(sub);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// The already-downloaded gameplay snap for a ROM, if any. Network-free.
pub fn snap_path(rom_path: &str) -> Option<PathBuf> {
    SNAP_CACHE.lock().get(rom_path).cloned()
}

/// The already-downloaded title screen for a ROM, if any. Network-free.
pub fn title_path(rom_path: &str) -> Option<PathBuf> {
    TITLE_CACHE.lock().get(rom_path).cloned()
}

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

const N64_DAT_URL: &str = "https://raw.githubusercontent.com/libretro/libretro-database/master/metadat/no-intro/Nintendo - Nintendo 64.dat";

fn crc_index_cache_path() -> PathBuf {
    ui::get_dirs().cache_dir.join("n64_crc_index.json")
}

// CRC32 (No-Intro) -> canonical game name, fetched once from the libretro DAT.
type CrcIndex = std::sync::Arc<HashMap<u32, String>>;
static CRC_INDEX: LazyLock<Mutex<Option<CrcIndex>>> = LazyLock::new(|| Mutex::new(None));

async fn crc_index() -> CrcIndex {
    if let Some(m) = CRC_INDEX.lock().clone() {
        return m;
    }
    let m = std::sync::Arc::new(load_crc_index().await);
    *CRC_INDEX.lock() = Some(m.clone());
    m
}

/// Parse the libretro N64 DAT (clrmamepro) into CRC32 -> No-Intro name. Cached on
/// disk; the DAT lists a separate CRC per byte-order, all sharing one game name.
async fn load_crc_index() -> HashMap<u32, String> {
    if let Ok(bytes) = tokio::fs::read(crc_index_cache_path()).await
        && let Ok(map) = serde_json::from_slice::<HashMap<u32, String>>(&bytes)
        && !map.is_empty()
    {
        return map;
    }
    let mut map = HashMap::new();
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

fn crc_cache_path() -> PathBuf {
    ui::get_dirs().cache_dir.join("n64_rom_crc.json")
}

// rom path -> (size, mtime-secs, crc32) so unchanged ROMs aren't re-read each run.
type CrcCache = HashMap<String, (u64, u64, u32)>;
static CRC_CACHE: LazyLock<Mutex<CrcCache>> = LazyLock::new(|| {
    let map = std::fs::read(crc_cache_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    Mutex::new(map)
});

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

// CRC32 of the emulator-normalized ROM bytes (archive-extracted + byteswapped to
// z64), matching the DAT. Cached by file signature; reads the ROM only on a miss.
fn crc_of_rom(rom_path: &str) -> Option<u32> {
    let (size, mtime) = file_sig(rom_path)?;
    if let Some(&(s, t, crc)) = CRC_CACHE.lock().get(rom_path)
        && s == size
        && t == mtime
    {
        return Some(crc);
    }
    let bytes = crate::device::get_rom_contents(&std::path::PathBuf::from(rom_path))?;
    let crc = crc32fast::hash(&bytes);
    CRC_CACHE
        .lock()
        .insert(rom_path.to_string(), (size, mtime, crc));
    Some(crc)
}

/// Persist the ROM CRC cache (call after a scan completes).
pub fn persist_crc_cache() {
    if let Ok(bytes) = serde_json::to_vec(&*CRC_CACHE.lock()) {
        let _ = std::fs::write(crc_cache_path(), bytes);
    }
}

// rom path -> unofficial? (CRC absent from the No-Intro DAT), set during the scan.
static HOMEBREW: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Whether a ROM is unofficial (its CRC isn't in the No-Intro DAT). Network-free;
/// false until `mark_homebrew` has run for it (or if the DAT is unavailable).
pub fn is_homebrew(rom_path: &str) -> bool {
    HOMEBREW.lock().get(rom_path).copied().unwrap_or(false)
}

/// Record whether a ROM is unofficial, reusing the cached CRC + DAT (cheap once
/// box art resolved for it). No-op if the DAT couldn't be loaded.
pub async fn mark_homebrew(rom_path: &str) {
    let Some(crc) = crc_of_rom(rom_path) else {
        return;
    };
    let ci = crc_index().await;
    if !ci.is_empty() {
        HOMEBREW
            .lock()
            .insert(rom_path.to_string(), !ci.contains_key(&crc));
    }
}

/// Resolve the libretro thumbnail filename for a ROM. Hash-primary: the ROM's
/// CRC32 -> exact No-Intro name; falls back to the filename fuzzy match for ROMs
/// not in the DAT (homebrew, hacks, bad dumps).
async fn resolve_filename(index: &Index, rom_path: &str, title: &str) -> Option<String> {
    let mut name = None;
    if let Some(crc) = crc_of_rom(rom_path) {
        let ci = crc_index().await;
        if let Some(n) = ci.get(&crc) {
            name = Some(format!("{n}.png"));
        }
    }
    let name = name.or_else(|| match_name(index, title))?;
    // name is remote-derived (libretro DAT/index); reject path separators so it
    // can't escape the cache dir when join()ed.
    (!name.contains('/') && !name.contains('\\')).then_some(name)
}

/// Ensure box art for a ROM is downloaded and registered. Returns true when art
/// is available afterwards. Network-free when already cached on disk.
pub async fn resolve_and_cache(index: &Index, rom_path: &str, title: &str) -> bool {
    if ART_CACHE.lock().contains_key(rom_path) {
        return true;
    }
    let Some(filename) = resolve_filename(index, rom_path, title).await else {
        return false;
    };
    // Cache under the resolved (canonical No-Intro) filename, not the ROM title,
    // so region/variant is preserved and a wrong name-based cache can't stick.
    let dest = boxart_dir().join(&filename);
    if tokio::fs::try_exists(&dest).await.unwrap_or(false) {
        ART_CACHE.lock().insert(rom_path.to_string(), dest);
        return true;
    }
    if download("Named_Boxarts", &filename, &dest).await {
        ART_CACHE.lock().insert(rom_path.to_string(), dest);
        true
    } else {
        false
    }
}

/// Best-effort download of this ROM's gameplay snap + title screen. Reuses the
/// box-art filename match — libretro uses the identical No-Intro name across
/// Named_Boxarts / Named_Snaps / Named_Titles — so no separate index is needed;
/// a game missing a snap/title simply 404s and is skipped.
// ponytail: reuses the boxart match; add a dedicated Snaps/Titles index only if hit-rate disappoints.
pub async fn resolve_media(index: &Index, rom_path: &str, title: &str) {
    let Some(filename) = resolve_filename(index, rom_path, title).await else {
        return;
    };
    for (cache, sub, category) in [
        (&SNAP_CACHE, "snap", "Named_Snaps"),
        (&TITLE_CACHE, "title", "Named_Titles"),
    ] {
        if cache.lock().contains_key(rom_path) {
            continue;
        }
        let dest = media_dir(sub).join(&filename);
        if tokio::fs::try_exists(&dest).await.unwrap_or(false)
            || download(category, &filename, &dest).await
        {
            cache.lock().insert(rom_path.to_string(), dest);
        }
    }
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

    // Hits the network; run explicitly: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn network_download() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let index = load_index().await;
            assert!(!index.is_empty(), "index empty");
            let ok =
                resolve_and_cache(&index, "/tmp/fake/Super Mario 64.z64", "Super Mario 64").await;
            assert!(ok, "resolve_and_cache failed");
            assert!(art_path("/tmp/fake/Super Mario 64.z64").is_some());
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
            let rom = "/tmp/fake/Super Mario 64 (media).z64";
            // Clear any cached files so this proves the real download path each run.
            // resolve_media now caches under the canonical No-Intro filename.
            let fname = "Super Mario 64 (USA).png";
            let snap_dest = media_dir("snap").join(fname);
            let title_dest = media_dir("title").join(fname);
            let _ = std::fs::remove_file(&snap_dest);
            let _ = std::fs::remove_file(&title_dest);
            assert!(
                !snap_dest.exists() && !title_dest.exists(),
                "failed to clear cache"
            );
            resolve_media(&index, rom, "Super Mario 64").await;
            // Super Mario 64 has a snap and a title screen in the libretro N64 repo.
            assert!(snap_dest.exists(), "snap not downloaded");
            assert!(title_dest.exists(), "title not downloaded");
            assert!(snap_path(rom).is_some(), "snap not registered");
            assert!(title_path(rom).is_some(), "title not registered");
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
