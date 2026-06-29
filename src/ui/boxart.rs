// boxart.rs — auto-download N64 box art from the libretro thumbnails server.
//
// Source: https://thumbnails.libretro.com/Nintendo - Nintendo 64/Named_Boxarts/
// Matching: the box-art filename index (fetched once from GitHub, cached on
// disk) is keyed by a normalized name so real ROM filenames — which rarely use
// the exact No-Intro name — still resolve (handles region tags, articles, etc.).
// No API key or account required. Cached on disk; no in-memory state.

use crate::ui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TREE_URL: &str = "https://api.github.com/repos/libretro-thumbnails/Nintendo_-_Nintendo_64/git/trees/master?recursive=1";
const THUMB_BASE: &str = "https://thumbnails.libretro.com/";

pub type Index = HashMap<String, Vec<String>>;

fn boxart_dir() -> PathBuf {
    let dir = ui::get_dirs().cache_dir.join("boxart");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn index_cache_path() -> PathBuf {
    ui::get_dirs().cache_dir.join("n64_boxart_index.json")
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

async fn download(filename: &str, dest: &Path) -> bool {
    let Ok(mut url) = reqwest::Url::parse(THUMB_BASE) else {
        return false;
    };
    if let Ok(mut segs) = url.path_segments_mut() {
        segs.pop_if_empty()
            .extend(["Nintendo - Nintendo 64", "Named_Boxarts", filename]);
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

/// Resolve + download box art for a ROM title; returns the on-disk PNG path, or
/// `None` if no match / download failed. Network-free when already on disk.
/// Filename-based match against the libretro index (No-Intro naming).
pub async fn resolve_art(index: &Index, title: &str) -> Option<PathBuf> {
    let filename = match_name(index, title)?;
    let dest = boxart_dir().join(&filename);
    if tokio::fs::try_exists(&dest).await.unwrap_or(false) {
        return Some(dest);
    }
    download(&filename, &dest).await.then_some(dest)
}
