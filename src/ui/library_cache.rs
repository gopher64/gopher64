// Incremental library-scan cache.
//
// The box-art / homebrew lookup needs the full-file SHA-256 of each ROM (the
// No-Intro map is keyed by it), which means decompressing + hashing every ROM —
// the dominant cost of a folder scan. This cache lets a rescan skip that work
// for files that have not changed, using the industry-standard fast-path filter:
// a file whose (size, mtime) still match the cached entry is assumed unchanged,
// so its stored sha256 / name / homebrew flag are reused without any read.
//
// Persisted with postcard (already a dependency) under `cache_dir`, keyed by the
// absolute ROM path. A miss (new file, or size/mtime changed) is the only case
// that falls through to a real read + hash.

use crate::ui;

const CACHE_FILE: &str = "library_cache.postcard";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    /// File size in bytes at the time of hashing.
    pub size: u64,
    /// File mtime in nanoseconds since the Unix epoch at the time of hashing.
    pub mtime_ns: u64,
    /// Full-file SHA-256 (lowercase hex) of the big-endian ROM.
    pub sha256: String,
    /// Resolved No-Intro name (or header/filename fallback for homebrew).
    pub name: String,
    /// True when the hash was absent from the No-Intro map.
    pub homebrew: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct LibraryCache {
    entries: rustc_hash::FxHashMap<String, CacheEntry>,
}

/// `(size, mtime_ns)` for `path`, or None if the file is unreadable.
pub fn file_signature(path: &std::path::Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len();
    let mtime_ns = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    Some((size, mtime_ns))
}

impl LibraryCache {
    fn path() -> std::path::PathBuf {
        ui::get_dirs().cache_dir.join(CACHE_FILE)
    }

    /// Load the cache from disk, or an empty cache if absent/corrupt (never fails:
    /// a stale/garbage cache only costs a re-hash, never correctness).
    pub fn load() -> LibraryCache {
        match std::fs::read(Self::path()) {
            Ok(bytes) => postcard::from_bytes(&bytes).unwrap_or_default(),
            Err(_) => LibraryCache::default(),
        }
    }

    /// Persist the cache (best-effort; a write failure only means the next scan
    /// re-hashes). Creates `cache_dir` if needed.
    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = postcard::to_stdvec(self) {
            let _ = std::fs::write(path, bytes);
        }
    }

    /// Cached entry for `path` IF the file's current `(size, mtime_ns)` still
    /// match what was hashed — i.e. the fast-path hit that skips read + hash.
    pub fn get_fresh(&self, path: &str, size: u64, mtime_ns: u64) -> Option<&CacheEntry> {
        self.entries
            .get(path)
            .filter(|e| e.size == size && e.mtime_ns == mtime_ns)
    }

    /// Record (or refresh) the entry for `path` after a real hash.
    pub fn insert(
        &mut self,
        path: String,
        size: u64,
        mtime_ns: u64,
        sha256: String,
        name: String,
        homebrew: bool,
    ) {
        self.entries.insert(
            path,
            CacheEntry {
                size,
                mtime_ns,
                sha256,
                name,
                homebrew,
            },
        );
    }

    /// Drop entries whose path is no longer in `live_paths`, so a cache for a
    /// churning folder cannot grow without bound.
    pub fn retain_paths(&mut self, live_paths: &std::collections::HashSet<String>) {
        self.entries.retain(|k, _| live_paths.contains(k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_fresh_matches_only_on_size_and_mtime() {
        let mut c = LibraryCache::default();
        c.insert(
            "/roms/a.z64".into(),
            1024,
            42,
            "abc".into(),
            "Game A".into(),
            false,
        );
        // Exact match -> hit.
        assert!(c.get_fresh("/roms/a.z64", 1024, 42).is_some());
        // Changed size -> miss (must re-hash).
        assert!(c.get_fresh("/roms/a.z64", 2048, 42).is_none());
        // Changed mtime -> miss (must re-hash).
        assert!(c.get_fresh("/roms/a.z64", 1024, 99).is_none());
        // Unknown path -> miss.
        assert!(c.get_fresh("/roms/b.z64", 1024, 42).is_none());
    }

    #[test]
    fn retain_paths_prunes_stale_entries() {
        let mut c = LibraryCache::default();
        c.insert("/roms/a.z64".into(), 1, 1, "h".into(), "A".into(), false);
        c.insert("/roms/b.z64".into(), 1, 1, "h".into(), "B".into(), true);
        let mut live = std::collections::HashSet::new();
        live.insert("/roms/a.z64".to_string());
        c.retain_paths(&live);
        assert!(c.get_fresh("/roms/a.z64", 1, 1).is_some());
        assert!(c.get_fresh("/roms/b.z64", 1, 1).is_none());
    }

    #[test]
    fn roundtrip_serialization_preserves_entries() {
        let mut c = LibraryCache::default();
        c.insert(
            "/roms/a.z64".into(),
            1024,
            42,
            "deadbeef".into(),
            "Game A".into(),
            true,
        );
        let bytes = postcard::to_stdvec(&c).unwrap();
        let back: LibraryCache = postcard::from_bytes(&bytes).unwrap();
        let e = back.get_fresh("/roms/a.z64", 1024, 42).unwrap();
        assert_eq!(e.sha256, "deadbeef");
        assert_eq!(e.name, "Game A");
        assert!(e.homebrew);
    }

    #[test]
    fn reinsert_same_path_overwrites() {
        // A modified ROM (same path, new size/mtime/hash) must refresh its entry,
        // not leave a stale duplicate keyed by the old signature.
        let mut c = LibraryCache::default();
        let p = "/roms/game.z64";
        c.insert(p.into(), 1, 1, "a".into(), "A".into(), false);
        c.insert(p.into(), 2, 2, "b".into(), "B".into(), true);
        // Old signature is gone -> forces a re-hash.
        assert!(c.get_fresh(p, 1, 1).is_none());
        // New signature hits and carries the refreshed fields.
        let e = c.get_fresh(p, 2, 2).unwrap();
        assert_eq!(e.sha256, "b");
        assert_eq!(e.name, "B");
        assert!(e.homebrew);
    }

    #[test]
    fn multi_entry_roundtrip() {
        // Serialization is lossless for a realistic multi-game cache.
        let mut c = LibraryCache::default();
        c.insert(
            "/roms/a.z64".into(),
            1024,
            10,
            "aaa".into(),
            "Game A".into(),
            false,
        );
        c.insert(
            "/roms/b.z64".into(),
            2048,
            20,
            "bbb".into(),
            "Homebrew B".into(),
            true,
        );
        c.insert(
            "/roms/c.z64".into(),
            4096,
            30,
            "ccc".into(),
            "Game C".into(),
            false,
        );
        let bytes = postcard::to_stdvec(&c).unwrap();
        let back: LibraryCache = postcard::from_bytes(&bytes).unwrap();
        let a = back.get_fresh("/roms/a.z64", 1024, 10).unwrap();
        assert_eq!(a.sha256, "aaa");
        assert_eq!(a.name, "Game A");
        assert!(!a.homebrew);
        let b = back.get_fresh("/roms/b.z64", 2048, 20).unwrap();
        assert_eq!(b.sha256, "bbb");
        assert_eq!(b.name, "Homebrew B");
        assert!(b.homebrew);
        let c3 = back.get_fresh("/roms/c.z64", 4096, 30).unwrap();
        assert_eq!(c3.sha256, "ccc");
        assert_eq!(c3.name, "Game C");
        assert!(!c3.homebrew);
    }

    #[test]
    fn from_bytes_garbage_is_err_or_empty() {
        // load() relies on unwrap_or_default() to survive a corrupt cache file;
        // this proves that fallback path is real, not dead code.
        let res = postcard::from_bytes::<LibraryCache>(&[0xFF, 0xFF, 0xFF]);
        match res {
            Err(_) => {} // expected: garbage does not deserialize.
            Ok(cache) => {
                // Surprise: it parsed. Still safe as long as nothing reads fresh
                // from it -> the caller would just re-hash.
                assert!(cache.get_fresh("/roms/anything.z64", 1, 1).is_none());
            }
        }
    }

    #[test]
    fn retain_paths_empty_clears_all() {
        // An empty live set means the folder is gone -> drop everything.
        let mut c = LibraryCache::default();
        c.insert("/roms/a.z64".into(), 1, 1, "h".into(), "A".into(), false);
        c.insert("/roms/b.z64".into(), 1, 1, "h".into(), "B".into(), true);
        c.retain_paths(&std::collections::HashSet::new());
        assert!(c.get_fresh("/roms/a.z64", 1, 1).is_none());
        assert!(c.get_fresh("/roms/b.z64", 1, 1).is_none());
    }

    #[test]
    fn retain_paths_superset_keeps_all() {
        // A live set covering every cached path (plus extras) prunes nothing.
        let mut c = LibraryCache::default();
        c.insert("/roms/a.z64".into(), 1, 1, "h".into(), "A".into(), false);
        c.insert("/roms/b.z64".into(), 1, 1, "h".into(), "B".into(), true);
        let mut live = std::collections::HashSet::new();
        live.insert("/roms/a.z64".to_string());
        live.insert("/roms/b.z64".to_string());
        live.insert("/roms/unused.z64".to_string());
        c.retain_paths(&live);
        assert!(c.get_fresh("/roms/a.z64", 1, 1).is_some());
        assert!(c.get_fresh("/roms/b.z64", 1, 1).is_some());
    }

    #[test]
    fn empty_cache_get_fresh_none_and_roundtrips() {
        let c = LibraryCache::default();
        assert!(c.get_fresh("/roms/anything.z64", 1, 1).is_none());
        let bytes = postcard::to_stdvec(&c).unwrap();
        let back: LibraryCache = postcard::from_bytes(&bytes).unwrap();
        assert!(back.get_fresh("/roms/anything.z64", 1, 1).is_none());
    }

    // Performance harness for the folder-scan pipeline. Not a correctness test —
    // it prints timings, so run it explicitly:
    //
    //   # synthetic (default: 50 ROMs x 16 MiB, no copyrighted content needed —
    //   # SHA-256 + read cost depend on byte count, not bytes):
    //   cargo test --lib scan_perf_benchmark -- --ignored --nocapture
    //
    //   # tune the synthetic set:
    //   G64_SCAN_BENCH_COUNT=200 G64_SCAN_BENCH_MB=32 \
    //     cargo test --lib scan_perf_benchmark -- --ignored --nocapture
    //
    //   # or point it at a REAL folder of your own ROMs (recursive):
    //   G64_SCAN_BENCH_DIR=/path/to/roms \
    //     cargo test --lib scan_perf_benchmark -- --ignored --nocapture
    //
    // Reports three numbers that map 1:1 to the optimization:
    //   cold-serial   = the old path (read + SHA-256 every ROM, one at a time)
    //   cold-parallel = first scan now (bounded across cores)
    //   warm-cached   = every rescan now (size+mtime check, zero read/hash)
    #[test]
    #[ignore]
    fn scan_perf_benchmark() {
        use std::time::Instant;

        // Collect the ROM paths to measure: a real folder if given, else a
        // freshly generated synthetic set in a temp dir.
        let (paths, cleanup_dir): (Vec<std::path::PathBuf>, Option<std::path::PathBuf>) =
            if let Ok(dir) = std::env::var("G64_SCAN_BENCH_DIR") {
                let mut v = Vec::new();
                let mut stack = vec![std::path::PathBuf::from(&dir)];
                while let Some(d) = stack.pop() {
                    let Ok(rd) = std::fs::read_dir(&d) else {
                        continue;
                    };
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.is_dir() {
                            stack.push(p);
                        } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| {
                            // Cart ROMs only: this bench measures get_rom_contents +
                            // cart SHA-256. 64DD disks (.ndd/.d64) take a different
                            // path and would skew throughput, so exclude them.
                            ["n64", "v64", "z64", "zip", "7z"].contains(&x.to_lowercase().as_str())
                        }) {
                            v.push(p);
                        }
                    }
                }
                println!("[bench] using real folder {dir}: {} ROMs", v.len());
                (v, None)
            } else {
                let count: usize = std::env::var("G64_SCAN_BENCH_COUNT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(50);
                let mb: usize = std::env::var("G64_SCAN_BENCH_MB")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(16);
                let dir =
                    std::env::temp_dir().join(format!("g64_scanbench_{}", std::process::id()));
                std::fs::create_dir_all(&dir).unwrap();
                let size = mb * 1024 * 1024;
                let mut buf = vec![0u8; size];
                buf[0..4].copy_from_slice(&0x8037_1240u32.to_be_bytes()); // z64 magic
                let mut v = Vec::with_capacity(count);
                for i in 0..count {
                    // Vary a byte so hashes differ (irrelevant to timing, realistic).
                    buf[4] = (i & 0xff) as u8;
                    let p = dir.join(format!("bench_{i:04}.z64"));
                    std::fs::write(&p, &buf).unwrap();
                    v.push(p);
                }
                println!("[bench] generated {count} synthetic ROMs x {mb} MiB in {dir:?}");
                (v, Some(dir))
            };

        assert!(!paths.is_empty(), "no ROMs to benchmark");
        let total_bytes: u64 = paths
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        let total_mib = total_bytes as f64 / (1024.0 * 1024.0);

        // --- cold-serial: the old path, one ROM at a time ---
        let t = Instant::now();
        for p in &paths {
            if let Some(rom) = crate::device::get_rom_contents(&p.to_path_buf()) {
                std::hint::black_box(crate::device::cart::rom::calculate_hash(&rom));
            }
        }
        let serial = t.elapsed();

        // --- cold-parallel: first scan now, bounded across cores ---
        let workers = std::thread::available_parallelism()
            .map(|n| n.get().min(4))
            .unwrap_or(2);
        let t = Instant::now();
        std::thread::scope(|s| {
            for chunk in paths.chunks(paths.len().div_ceil(workers)) {
                s.spawn(move || {
                    for p in chunk {
                        if let Some(rom) = crate::device::get_rom_contents(&p.to_path_buf()) {
                            std::hint::black_box(crate::device::cart::rom::calculate_hash(&rom));
                        }
                    }
                });
            }
        });
        let parallel = t.elapsed();

        // --- warm-cached: every rescan now, size+mtime only, zero read/hash ---
        let mut cache = LibraryCache::default();
        for p in &paths {
            let key = p.to_string_lossy().to_string();
            if let Some((size, mtime)) = file_signature(p) {
                cache.insert(key, size, mtime, "x".into(), "n".into(), false);
            }
        }
        let t = Instant::now();
        let mut hits = 0usize;
        for p in &paths {
            let key = p.to_string_lossy().to_string();
            if let Some((size, mtime)) = file_signature(p)
                && cache.get_fresh(&key, size, mtime).is_some()
            {
                hits += 1;
            }
        }
        let warm = t.elapsed();

        println!(
            "[bench] {} ROMs, {total_mib:.1} MiB total, {workers} hash workers",
            paths.len()
        );
        println!(
            "[bench] cold-serial   : {serial:?}  ({:.1} MiB/s)",
            total_mib / serial.as_secs_f64()
        );
        println!(
            "[bench] cold-parallel : {parallel:?}  ({:.1} MiB/s)  speedup {:.2}x",
            total_mib / parallel.as_secs_f64(),
            serial.as_secs_f64() / parallel.as_secs_f64()
        );
        println!(
            "[bench] warm-cached   : {warm:?}  ({hits}/{} hits)  speedup {:.0}x",
            paths.len(),
            serial.as_secs_f64() / warm.as_secs_f64().max(1e-9)
        );

        if let Some(dir) = cleanup_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
