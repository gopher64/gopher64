use crate::device;
use crate::retroachievements;
use crate::ui;
#[cfg(target_os = "android")]
use crate::ui::android;
use slint::Model;
#[cfg(not(target_os = "android"))]
use slint::winit_030::WinitWindowAccessor;

slint::include_modules!();

#[cfg(not(target_os = "android"))]
pub const N64_EXTENSIONS: [&str; 12] = [
    "n64", "v64", "z64", "7z", "zip", "bin", "N64", "V64", "Z64", "7Z", "ZIP", "BIN",
];

#[derive(serde::Deserialize)]
struct GithubData {
    tag_name: String,
}

pub struct NetplayDevice {
    pub server_addr: String,
    pub player_number: usize,
    pub number_of_players: usize,
    pub input_delay: usize,
}

fn check_latest_version(weak: slint::Weak<AppWindow>) {
    let task = ui::WEB_CLIENT
        .get("https://api.github.com/repos/gopher64/gopher64/releases/latest")
        .send();
    tokio::spawn(async move {
        let response = task.await;
        if let Ok(response) = response {
            let data: Result<GithubData, reqwest::Error> = response.json().await;

            let latest_version = match data {
                Ok(data) => match semver::Version::parse(&data.tag_name[1..]) {
                    Ok(github_version) => github_version,
                    Err(e) => {
                        eprintln!("Error parsing latest version from GitHub: {}", e);
                        semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
                    }
                },
                Err(e) => {
                    eprintln!("Error getting latest version from GitHub: {}", e);
                    semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
                }
            };
            let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
            if current_version < latest_version {
                weak.upgrade_in_event_loop(move |handle| handle.set_has_update(true))
                    .unwrap();
            }
        }
    });
}

pub fn open_uri(path: impl AsRef<std::ffi::OsStr>) {
    #[cfg(target_os = "android")]
    return ui::android::open_uri(path.as_ref().to_str().unwrap());

    #[cfg(not(target_os = "android"))]
    if let Err(e) = open::that_detached(path) {
        eprintln!("Error opening path: {}", e);
    }
}

fn run_with_path(weak: slint::Weak<AppWindow>, path: std::path::PathBuf, load_slot: Option<u32>) {
    let weak2 = weak.clone();
    weak.upgrade_in_event_loop(move |handle| {
        if handle.get_game_running() {
            return;
        }
        save_settings(&handle);

        run_rom(
            path,
            ui::GameSettings {
                overclock: handle.get_overclock_n64_cpu(),
                disable_expansion_pak: handle.get_disable_expansion_pak(),
                cheats: rustc_hash::FxHashMap::default(), // will be filled in later
                load_savestate_slot: load_slot,
            },
            None,
            weak2,
        );
    })
    .unwrap();
}

// Which save-state slots (0-9) already have data on disk for this ROM. Mirrors the
// emulator's path: states/<game-name-or-id>-<sha256(rom)>.state<slot>. Reads +
// hashes the whole ROM, so it MUST be called inside spawn_blocking.
fn savestate_slots(path: &str) -> Vec<bool> {
    let Some(rom) = crate::device::get_rom_contents(&std::path::PathBuf::from(path)) else {
        return vec![false; 10];
    };
    let hash = crate::device::cart::rom::calculate_hash(&rom);
    let name = ui::storage::get_game_name(&rom);
    let prefix = if name.is_empty() {
        let id = String::from_utf8_lossy(&rom[0x3B..0x3E]);
        if id.contains('\0') {
            "UNK".to_string()
        } else {
            id.into_owned()
        }
    } else {
        name
    };
    let states = ui::get_dirs().data_dir.join("states");
    (0..10)
        .map(|slot| states.join(format!("{prefix}-{hash}.state{slot}")).exists())
        .collect()
}

// Open the game detail view for `path`: title/box-art come from the already-loaded
// library row; save-state slots and snap/title media are filled in off-thread.
fn open_detail(handle: &AppWindow, path: &str) {
    handle.set_detail_tab(0);
    handle.set_detail_slot(-1);
    handle.set_detail_slots(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(vec![false; 10]),
    )));
    // Clear stale media; the box art (carried by the row) is the hero fallback.
    handle.set_detail_hero(slint::Image::default());
    handle.set_detail_has_hero(false);
    handle.set_detail_snap(slint::Image::default());
    handle.set_detail_has_snap(false);
    handle.set_detail_title_art(slint::Image::default());
    handle.set_detail_has_title(false);

    let all = handle.get_all_games();
    let row = (0..all.row_count())
        .filter_map(|i| all.row_data(i))
        .find(|g| g.path.as_str() == path);
    let art_name = if let Some(g) = row {
        handle.set_detail_title(g.title.clone());
        handle.set_detail_art(g.art.clone());
        handle.set_detail_has_art(g.has_art);
        handle.set_detail_favorite(g.favorite);
        g.art_name.to_string()
    } else {
        handle.set_detail_title(rom_title(path).into());
        handle.set_detail_art(slint::Image::default());
        handle.set_detail_has_art(false);
        handle.set_detail_favorite(false);
        String::new()
    };
    handle.set_detail_path(path.into());
    handle.set_detail_open(true);

    let weak = handle.as_weak();
    let path = path.to_string();
    tokio::spawn(async move {
        // Save-state availability: whole-ROM read + hash must stay off the UI thread.
        let p = path.clone();
        if let Ok(slots) = tokio::task::spawn_blocking(move || savestate_slots(&p)).await {
            let p = path.clone();
            let _ = weak.upgrade_in_event_loop(move |h| {
                if h.get_detail_open() && h.get_detail_path().as_str() == p {
                    h.set_detail_slots(slint::ModelRc::from(std::rc::Rc::new(
                        slint::VecModel::from(slots),
                    )));
                }
            });
        }
        // Gameplay snap + title screen (async network I/O; fine on the runtime).
        if !art_name.is_empty() {
            let (snap, title) = ui::boxart::download_media(&art_name).await;
            let _ = weak.upgrade_in_event_loop(move |h| {
                if !h.get_detail_open() || h.get_detail_path().as_str() != path {
                    return;
                }
                let snap = snap
                    .as_deref()
                    .and_then(|p| slint::Image::load_from_path(p).ok());
                let title = title
                    .as_deref()
                    .and_then(|p| slint::Image::load_from_path(p).ok());
                if let Some(img) = &snap {
                    h.set_detail_snap(img.clone());
                    h.set_detail_has_snap(true);
                }
                if let Some(img) = &title {
                    h.set_detail_title_art(img.clone());
                    h.set_detail_has_title(true);
                }
                if let Some(img) = snap.or(title) {
                    h.set_detail_hero(img);
                    h.set_detail_has_hero(true);
                }
            });
        }
    });
}

#[cfg(not(target_os = "android"))]
fn file_dropped(app: &AppWindow) {
    let weak = app.as_weak();
    app.window()
        .on_winit_window_event(move |_winit_window, event| {
            if let slint::winit_030::winit::event::WindowEvent::DroppedFile(path) = event {
                run_with_path(weak.clone(), path.to_path_buf(), None);
            }
            slint::winit_030::EventResult::Propagate
        });
}

fn rom_exists(path: &str) -> bool {
    #[cfg(not(target_os = "android"))]
    return std::fs::exists(path).unwrap_or(false);
    #[cfg(target_os = "android")]
    return android::rom_exists(path);
}

// Box-art library: filename stem doubles as the display title + box-art match key.
fn rom_title(path: &str) -> String {
    std::path::Path::new(&decode_path(path))
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

fn game_entries(paths: &[String], favorites: &std::collections::HashSet<String>) -> Vec<GameEntry> {
    paths
        .iter()
        .map(|p| GameEntry {
            path: p.as_str().into(),
            title: rom_title(p).into(),
            art: slint::Image::default(),
            has_art: false,
            favorite: favorites.contains(p),
            homebrew: false,
            art_name: slint::SharedString::new(),
        })
        .collect()
}

// Set the full (unfiltered) library, then apply the active search/filter to it.
fn set_all_games(app: &AppWindow, paths: &[String]) {
    let favorites: std::collections::HashSet<String> =
        app.get_favorites().iter().map(|s| s.to_string()).collect();
    app.set_all_games(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(game_entries(paths, &favorites)),
    )));
    apply_filter(app);
}

// Shared No-Intro map for the Android SAF folder-scan JNI callback, which can't be
// handed the Arc as a parameter. OnceLock (not LazyLock): the value is the specific
// runtime Arc created in lib.rs and shared with load_no_intro/netplay/cheats — a
// fresh LazyLock-built map would never be populated.
#[cfg(target_os = "android")]
static NO_INTRO_MAP: std::sync::OnceLock<
    std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
> = std::sync::OnceLock::new();

// Android SAF folder-scan result sink: Kotlin walks the picked document tree and
// hands back ROM content-URIs (via nativeOnFolderScanned); show them and fetch box
// art off the UI thread. `fetch_art`'s cache degrades to a no-op on Android
// (content-URIs have no stat), so every launch re-identifies — acceptable given the
// user re-picks the folder; a persisted-tree startup re-walk is a follow-up.
#[cfg(target_os = "android")]
pub fn apply_scanned_folder(weak: &slint::Weak<AppWindow>, paths: Vec<String>) {
    let Some(map) = NO_INTRO_MAP.get().cloned() else {
        return;
    };
    let shown = paths.clone();
    let _ = weak.upgrade_in_event_loop(move |h| set_all_games(&h, &shown));
    tokio::spawn(fetch_art(weak.clone(), paths, map));
}

// Library filter predicate (0 All · 1 Favorites · 2 Homebrew) + substring search.
fn game_matches(title_lc: &str, needle: &str, filter: i32, favorite: bool, homebrew: bool) -> bool {
    (needle.is_empty() || title_lc.contains(needle))
        && match filter {
            1 => favorite,
            2 => homebrew,
            _ => true,
        }
}

// Rebuild the displayed `games` model from `all_games` using the current search +
// filter. Cheap (images are ref-counted); runs on every keystroke / filter click.
fn apply_filter(app: &AppWindow) {
    let needle = app.get_library_search().to_lowercase();
    let filter = app.get_library_filter();
    let all = app.get_all_games();
    let mut shown = Vec::new();
    for i in 0..all.row_count() {
        if let Some(g) = all.row_data(i)
            && game_matches(
                &g.title.to_lowercase(),
                &needle,
                filter,
                g.favorite,
                g.homebrew,
            )
        {
            shown.push(g);
        }
    }
    app.set_games(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(shown),
    )));
}

// Walk a folder (recursively) for ROMs, including zip/7z archives.
#[cfg(not(target_os = "android"))]
fn scan_roms(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            // file_type() does not follow symlinks, so a symlink cycle can't loop us.
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            let p = entry.path();
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file()
                && let Some(ext) = p.extension().and_then(|e| e.to_str())
                && N64_EXTENSIONS.contains(&ext)
                && crate::device::is_n64_rom(&p)
            {
                out.push(p.to_string_lossy().to_string());
            }
        }
    }
    out.sort();
    out
}

// Resolve box art + homebrew flag for each game off-thread, updating its row as
// results arrive. The whole-ROM read runs in spawn_blocking and the hash runs in
// this async task (never the UI thread) — the maintainer's startup-freeze fix.
async fn fetch_art(
    weak: slint::Weak<AppWindow>,
    paths: Vec<String>,
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    if paths.is_empty() {
        return;
    }
    let mut cache = ui::library_cache::LibraryCache::load();
    let mut last = std::time::Instant::now();
    for (i, path) in paths.iter().enumerate() {
        // Fast path: unchanged file (size+mtime match) reuses the cached name +
        // homebrew flag, skipping the whole ROM read + SHA-256. This is the win for
        // rescans of a large folder.
        let sig = ui::library_cache::file_signature(std::path::Path::new(path));
        let (name, homebrew) = if let Some((size, mtime_ns)) = sig
            && let Some(entry) = cache.get_fresh(path, size, mtime_ns)
        {
            (entry.name.clone(), entry.homebrew)
        } else {
            let p = path.clone();
            let Ok(Some(rom)) = tokio::task::spawn_blocking(move || {
                device::get_rom_contents(&std::path::PathBuf::from(p))
            })
            .await
            else {
                continue;
            };
            // One hash, one map lock -> both name and homebrew (was two hashes).
            let hash = device::cart::rom::calculate_hash(&rom).to_lowercase();
            let (name, homebrew) = match no_intro_map.lock().await.get(&hash) {
                Some(n) => (n.clone(), false),
                None => (ui::storage::get_game_name(&rom), true),
            };
            if let Some((size, mtime_ns)) = sig {
                cache.insert(path.clone(), size, mtime_ns, hash, name.clone(), homebrew);
            }
            (name, homebrew)
        };
        let (png, art_name): (Option<std::path::PathBuf>, slint::SharedString) = if name.is_empty()
        {
            (None, slint::SharedString::new())
        } else {
            let filename = ui::boxart::escaped_filename(&name);
            let png = ui::boxart::download_art(&filename).await;
            (png, filename.into())
        };
        let row_path = path.clone();
        let _ = weak.upgrade_in_event_loop(move |h| {
            let all = h.get_all_games();
            if let Some(mut g) = all.row_data(i)
                && g.path.as_str() == row_path
            {
                if let Some(img) = png
                    .as_deref()
                    .and_then(|p| slint::Image::load_from_path(p).ok())
                {
                    g.art = img;
                    g.has_art = true;
                }
                g.homebrew = homebrew;
                g.art_name = art_name;
                all.set_row_data(i, g);
            }
        });
        if last.elapsed() > std::time::Duration::from_millis(400) {
            let _ = weak.upgrade_in_event_loop(|h| apply_filter(&h));
            last = std::time::Instant::now();
        }
    }
    // Drop entries for files no longer scanned so the cache can't grow unbounded.
    let live: std::collections::HashSet<String> = paths.iter().cloned().collect();
    cache.retain_paths(&live);
    cache.save();
    let _ = weak.upgrade_in_event_loop(|h| apply_filter(&h));
}

// Scan `dir` off the UI thread, show the rows, then resolve art (cache-backed).
// Shared by startup, manual Scan Folder, and the live watcher.
#[cfg(not(target_os = "android"))]
async fn rescan_and_fetch(
    weak: slint::Weak<AppWindow>,
    dir: std::path::PathBuf,
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    let paths = tokio::task::spawn_blocking(move || scan_roms(&dir))
        .await
        .unwrap_or_default();
    let shown = paths.clone();
    let _ = weak.upgrade_in_event_loop(move |h| set_all_games(&h, &shown));
    fetch_art(weak, paths, no_intro_map).await;
}

// Live folder watcher: keeps the library in sync while the app is open (desktop
// only — Android SAF content-URIs aren't visible to inotify/FSEvents). Kept alive
// in a static; replacing it drops the previous watcher and closes its channel, so
// the old debounce thread exits. A change coalesces a 1s burst into one rescan;
// rescans never write the watched folder, so they can't self-trigger.
#[cfg(not(target_os = "android"))]
static FOLDER_WATCHER: std::sync::LazyLock<std::sync::Mutex<Option<notify::RecommendedWatcher>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

#[cfg(not(target_os = "android"))]
fn watch_folder(
    weak: slint::Weak<AppWindow>,
    dir: std::path::PathBuf,
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    use notify::Watcher;
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut watcher =
        match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                let _ = tx.send(());
            }
        }) {
            Ok(w) => w,
            Err(_) => return,
        };
    if watcher
        .watch(&dir, notify::RecursiveMode::Recursive)
        .is_err()
    {
        return;
    }
    if let Ok(mut slot) = FOLDER_WATCHER.lock() {
        *slot = Some(watcher);
    }
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            while rx
                .recv_timeout(std::time::Duration::from_millis(1000))
                .is_ok()
            {}
            let weak = weak.clone();
            let dir = dir.clone();
            let map = no_intro_map.clone();
            handle.spawn(rescan_and_fetch(weak, dir, map));
        }
    });
}

fn local_game_window(
    app: &AppWindow,
    config: &ui::config::Config,
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    app.set_recent_roms(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
                .ui
                .recent_roms
                .iter()
                .filter(|x| rom_exists(x))
                .map(|x| {
                    (
                        x.into(),
                        std::path::Path::new(&decode_path(x))
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .into(),
                    )
                })
                .collect::<Vec<(slint::SharedString, slint::SharedString)>>(),
        ),
    )));
    app.set_favorites(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
                .ui
                .favorites
                .iter()
                .map(|s| s.as_str().into())
                .collect::<Vec<slint::SharedString>>(),
        ),
    )));

    // Box-art library. Desktop scans the ROM folder off the UI thread (a recursive
    // std::fs walk must never block GUI startup); Android lists recent ROMs.
    #[cfg(not(target_os = "android"))]
    {
        set_all_games(app, &[]);
        let rom_dir = config.ui.rom_dir.clone();
        let weak = app.as_weak();
        let map = no_intro_map.clone();
        tokio::spawn(async move {
            if !rom_dir.as_os_str().is_empty() && rom_dir.is_dir() {
                rescan_and_fetch(weak.clone(), rom_dir.clone(), map.clone()).await;
                // Keep the library live-synced while the app is open.
                watch_folder(weak, rom_dir, map);
            }
        });
    }
    #[cfg(target_os = "android")]
    {
        let game_paths: Vec<String> = config
            .ui
            .recent_roms
            .iter()
            .filter(|x| rom_exists(x))
            .cloned()
            .collect();
        set_all_games(app, &game_paths);
        tokio::spawn(fetch_art(app.as_weak(), game_paths, no_intro_map.clone()));

        // SAF folder-tree picker: Kotlin walks the tree -> nativeOnFolderScanned ->
        // apply_scanned_folder (which reads NO_INTRO_MAP set in app_window).
        let weak_scan = app.as_weak();
        app.on_scan_folder_clicked(move || {
            let start = weak_scan
                .upgrade()
                .map(|h| h.get_rom_dir().to_string())
                .unwrap_or_default();
            ui::android::select_rom_folder(start);
        });
    }

    let weak = app.as_weak();
    app.on_open_rom_button_clicked(move || {
        weak.upgrade_in_event_loop(move |handle| {
            save_settings(&handle);
            open_rom(&handle)
        })
        .unwrap();
    });

    let weak = app.as_weak();
    app.on_recent_rom_button_clicked(move |rom| {
        weak.upgrade_in_event_loop(move |handle| {
            run_with_path(
                handle.as_weak(),
                std::path::PathBuf::from(rom.to_string()),
                None,
            );
        })
        .unwrap();
    });

    let weak = app.as_weak();
    app.on_launch_game(move |path, slot| {
        weak.upgrade_in_event_loop(move |handle| {
            let load_slot = if slot >= 0 { Some(slot as u32) } else { None };
            run_with_path(
                handle.as_weak(),
                std::path::PathBuf::from(path.to_string()),
                load_slot,
            );
        })
        .unwrap();
    });

    let weak = app.as_weak();
    app.on_detail_opened(move |path| {
        weak.upgrade_in_event_loop(move |handle| {
            open_detail(&handle, path.as_str());
        })
        .unwrap();
    });

    let weak = app.as_weak();
    app.on_library_filter_changed(move |_, _| {
        weak.upgrade_in_event_loop(|handle| apply_filter(&handle))
            .unwrap();
    });

    let weak = app.as_weak();
    app.on_toggle_favorite(move |path| {
        weak.upgrade_in_event_loop(move |handle| {
            let mut favorites: Vec<slint::SharedString> = handle.get_favorites().iter().collect();
            let fav_now = match favorites.iter().position(|f| f.as_str() == path.as_str()) {
                Some(i) => {
                    favorites.remove(i);
                    false
                }
                None => {
                    favorites.push(path.clone());
                    true
                }
            };
            handle.set_favorites(slint::ModelRc::from(std::rc::Rc::new(
                slint::VecModel::from(favorites),
            )));
            let all = handle.get_all_games();
            for i in 0..all.row_count() {
                if let Some(mut g) = all.row_data(i)
                    && g.path.as_str() == path.as_str()
                {
                    g.favorite = fav_now;
                    all.set_row_data(i, g);
                    break;
                }
            }
            if handle.get_detail_path().as_str() == path.as_str() {
                handle.set_detail_favorite(fav_now);
            }
            save_settings(&handle);
            apply_filter(&handle);
        })
        .unwrap();
    });

    #[cfg(not(target_os = "android"))]
    {
        let saves_path = ui::get_dirs().data_dir.join("saves");
        app.on_saves_folder_button_clicked(move || {
            open_uri(&saves_path);
        });

        let weak_scan = app.as_weak();
        app.on_scan_folder_clicked(move || {
            let start = weak_scan
                .upgrade()
                .map(|h| h.get_rom_dir())
                .unwrap_or_default();
            let weak_inner = weak_scan.clone();
            let dialog = if !start.is_empty() && std::fs::exists(&start).unwrap_or(false) {
                rfd::AsyncFileDialog::new().set_directory(&start)
            } else {
                rfd::AsyncFileDialog::new()
            }
            .set_title("Select ROM Folder")
            .pick_folder();
            let map = no_intro_map.clone();
            tokio::spawn(async move {
                if let Some(folder) = dialog.await {
                    let dir = folder.path().to_path_buf();
                    let dir_str = dir.to_string_lossy().to_string();
                    let _ = weak_inner.upgrade_in_event_loop(move |h| {
                        h.set_rom_dir(dir_str.into());
                        save_settings(&h);
                    });
                    rescan_and_fetch(weak_inner.clone(), dir.clone(), map.clone()).await;
                    // Live-sync the newly picked folder while the app is open.
                    watch_folder(weak_inner, dir, map);
                }
            });
        });

        file_dropped(app);
    }
}

fn input_profiles(config: &ui::config::Config) -> Vec<String> {
    let mut profiles = vec![];
    for key in config.input.input_profiles.keys() {
        profiles.push(key.clone())
    }

    // make sure default profile is always first
    if let Some(pos) = profiles.iter().position(|x| x == "default") {
        let default_profile = profiles.remove(pos);
        profiles.insert(0, default_profile);
    }
    profiles
}

fn settings_window(app: &AppWindow, config: &ui::config::Config) {
    app.set_integer_scaling(config.video.integer_scaling);
    app.set_ssaa(config.video.ssaa);
    app.set_fullscreen(config.video.fullscreen);
    app.set_widescreen(config.video.widescreen);
    app.set_vsync(config.video.vsync);
    app.set_apply_crt_shader(config.video.crt);
    app.set_theme(config.ui.theme);
    app.set_overclock_n64_cpu(config.emulation.overclock);
    app.set_disable_expansion_pak(config.emulation.disable_expansion_pak);
    app.set_emulate_usb(config.emulation.usb);
    app.set_rewind(config.emulation.rewind);
    let combobox_value = match config.video.upscale {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        _ => 0,
    };
    app.set_resolution(combobox_value);

    if let Some(rom_dir_str) = config.ui.rom_dir.to_str() {
        app.set_rom_dir(rom_dir_str.into());
    }
}

pub fn update_input_profiles(weak: &slint::Weak<AppWindow>, config: &ui::config::Config) {
    let profiles = input_profiles(config);
    let config_bindings = config.input.input_profile_binding.clone();
    weak.upgrade_in_event_loop(move |handle| {
        let profile_bindings = slint::VecModel::default();
        for (i, input_profile_binding) in handle.get_selected_profile_binding().iter().enumerate() {
            let currently_selected = handle
                .get_input_profiles()
                .row_data(input_profile_binding as usize)
                .unwrap_or(config_bindings[i].clone().into())
                .to_string();
            let position = profiles
                .iter()
                .position(|profile| *profile == currently_selected);
            profile_bindings.push(position.unwrap_or(0) as i32);
        }

        handle.set_input_profiles(slint::ModelRc::from(std::rc::Rc::new(
            slint::VecModel::from(
                profiles
                    .iter()
                    .map(|x| x.into())
                    .collect::<Vec<slint::SharedString>>(),
            ),
        )));

        handle
            .set_selected_profile_binding(slint::ModelRc::from(std::rc::Rc::new(profile_bindings)));
    })
    .unwrap();
}

fn clear_gb_paths(weak: &slint::Weak<AppWindow>, player: i32) {
    weak.upgrade_in_event_loop(move |handle| {
        let rom_paths = handle.get_gb_rom_paths();
        let ram_paths = handle.get_gb_ram_paths();
        rom_paths.set_row_data(player as usize, String::new().into());
        ram_paths.set_row_data(player as usize, String::new().into());
        handle.set_gb_rom_paths(rom_paths);
        handle.set_gb_ram_paths(ram_paths);
    })
    .unwrap();
}

fn controller_window(app: &AppWindow, config: &ui::config::Config) {
    #[cfg(not(target_os = "android"))]
    ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);

    app.set_emulate_vru(config.input.emulate_vru);

    app.set_controller_enabled(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(config.input.controller_enabled.to_vec()),
    )));

    app.set_transferpak(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(config.input.transfer_pak.to_vec()),
    )));

    app.set_gb_rom_paths(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
                .input
                .gb_rom_path
                .iter()
                .map(|x| x.into())
                .collect::<Vec<slint::SharedString>>(),
        ),
    )));

    app.set_gb_ram_paths(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
                .input
                .gb_ram_path
                .iter()
                .map(|x| x.into())
                .collect::<Vec<slint::SharedString>>(),
        ),
    )));

    update_input_profiles(&app.as_weak(), config);

    app.set_controller_changed(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(vec![false, false, false, false]),
    )));

    let config_controller_assignment = config.input.controller_assignment.clone();
    let weak_app = app.as_weak();
    app.on_controller_window_created(move || {
        let controller_assignment = config_controller_assignment.clone();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                let mut current_selected_paths = vec![None; 4];
                for (i, selected_controller) in handle.get_selected_controller().iter().enumerate()
                {
                    current_selected_paths[i] = handle
                        .get_controller_paths()
                        .row_data(selected_controller as usize);
                }

                let controller_names = ui::input::get_controller_names();
                handle.set_controller_names(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(
                        controller_names
                            .iter()
                            .map(|x| x.into())
                            .collect::<Vec<slint::SharedString>>(),
                    ),
                )));

                let controller_paths = ui::input::get_controller_paths();
                handle.set_controller_paths(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(
                        controller_paths
                            .iter()
                            .map(|x| x.into())
                            .collect::<Vec<slint::SharedString>>(),
                    ),
                )));

                let selected_controllers = slint::VecModel::default();
                for i in 0..4 {
                    let assigned_path =
                        if let Some(current_selected_path) = &current_selected_paths[i] {
                            current_selected_path.to_string()
                        } else if let Some(config_assigned_path) = &controller_assignment[i] {
                            config_assigned_path.to_string()
                        } else {
                            String::new()
                        };
                    let selected_index = controller_paths
                        .iter()
                        .position(|controller_path| assigned_path == *controller_path)
                        .unwrap_or(0) as i32;
                    selected_controllers.push(selected_index);
                }
                handle.set_selected_controller(slint::ModelRc::from(std::rc::Rc::new(
                    selected_controllers,
                )));
            })
            .unwrap();
    });

    let weak_app = app.as_weak();
    app.on_input_profile_button_clicked(move || {
        weak_app
            .upgrade_in_event_loop(move |handle| {
                handle.set_input_deadzone(ui::input::DEADZONE_DEFAULT);
                handle.set_input_profile_name(String::new().into());
                handle.set_input_dinput(false);
                handle.set_show_input_profile(true);
            })
            .unwrap();
    });
    let weak_app = app.as_weak();
    app.on_input_profile_creation_button_clicked(move || {
        #[cfg(not(target_os = "android"))]
        let weak_app2 = weak_app.clone();
        weak_app
            .upgrade_in_event_loop(move |handle| {
                let profile_name = handle.get_input_profile_name();
                let dinput = handle.get_input_dinput();
                let deadzone = handle.get_input_deadzone();
                handle.set_show_input_profile(false);

                #[cfg(target_os = "android")]
                ui::android::spawn_configure_input_profile(profile_name, dinput, deadzone);

                #[cfg(not(target_os = "android"))]
                tokio::spawn(async move {
                    let cli_path = std::env::current_exe()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .join(format!("{}-cli", env!("CARGO_PKG_NAME")));
                    let cmd_path = if cfg!(target_os = "macos") && cli_path.exists() {
                        cli_path
                    } else {
                        std::env::current_exe().unwrap()
                    };
                    let mut command = tokio::process::Command::new(cmd_path);
                    command.args([
                        "--configure-input-profile",
                        &profile_name,
                        "--deadzone",
                        &deadzone.to_string(),
                    ]);
                    if dinput {
                        command.arg("--use-dinput");
                    }
                    if !command.status().await.unwrap().success() {
                        eprintln!("Failed to configure input profile");
                    }
                    let config = ui::config::Config::new();
                    update_input_profiles(&weak_app2, &config);
                });
            })
            .unwrap();
    });

    let weak_app2 = app.as_weak();
    app.on_transferpak_toggled(move |player, enabled| {
        if enabled {
            let select_gb_rom = select_gb_rom(player);

            let weak_app3 = weak_app2.clone();
            tokio::spawn(async move {
                if let Some(gb_rom) = select_gb_rom.await {
                    let weak_app4 = weak_app3.clone();
                    weak_app3
                        .upgrade_in_event_loop(move |_handle| {
                            let select_gb_ram = select_gb_ram(player);

                            tokio::spawn(async move {
                                if let Some(gb_ram) = select_gb_ram.await {
                                    weak_app4
                                        .upgrade_in_event_loop(move |handle| {
                                            let rom_paths = handle.get_gb_rom_paths();
                                            let ram_paths = handle.get_gb_ram_paths();
                                            rom_paths.set_row_data(
                                                player as usize,
                                                gb_rom.to_str().unwrap().into(),
                                            );
                                            ram_paths.set_row_data(
                                                player as usize,
                                                gb_ram.to_str().unwrap().into(),
                                            );
                                            handle.set_gb_rom_paths(rom_paths);
                                            handle.set_gb_ram_paths(ram_paths);
                                        })
                                        .unwrap();
                                } else {
                                    clear_gb_paths(&weak_app4, player);
                                }
                            });
                        })
                        .unwrap();
                } else {
                    clear_gb_paths(&weak_app3, player);
                }
            });
        }
    });
}

pub fn save_settings(app: &AppWindow) {
    let mut config = ui::config::Config::new();
    config.ui.rom_dir = app.get_rom_dir().to_string().into();
    config.ui.favorites = app.get_favorites().iter().map(|s| s.to_string()).collect();
    config.video.integer_scaling = app.get_integer_scaling();
    config.video.ssaa = app.get_ssaa();
    config.video.fullscreen = app.get_fullscreen();
    config.video.widescreen = app.get_widescreen();
    config.video.vsync = app.get_vsync();
    config.video.crt = app.get_apply_crt_shader();
    config.ui.theme = app.get_theme();
    config.emulation.overclock = app.get_overclock_n64_cpu();
    config.emulation.disable_expansion_pak = app.get_disable_expansion_pak();
    config.emulation.usb = app.get_emulate_usb();
    config.emulation.rewind = app.get_rewind();
    let upscale_values = [1, 2, 4, 8];
    config.video.upscale = upscale_values[app.get_resolution() as usize];

    config.input.emulate_vru = app.get_emulate_vru();
    for (i, controller_enabled) in app.get_controller_enabled().iter().enumerate() {
        config.input.controller_enabled[i] = controller_enabled;
    }
    for (i, transferpak_enabled) in app.get_transferpak().iter().enumerate() {
        config.input.transfer_pak[i] = transferpak_enabled;
        config.input.gb_rom_path[i] = app.get_gb_rom_paths().row_data(i).unwrap().to_string();
        config.input.gb_ram_path[i] = app.get_gb_ram_paths().row_data(i).unwrap().to_string();
    }
    for (i, input_profile_binding) in app.get_selected_profile_binding().iter().enumerate() {
        config.input.input_profile_binding[i] = app
            .get_input_profiles()
            .row_data(input_profile_binding as usize)
            .unwrap()
            .to_string();
    }

    for (i, selected_controller) in app.get_selected_controller().iter().enumerate() {
        if app.get_controller_changed().row_data(i).unwrap_or(false) {
            let controller_path = app
                .get_controller_paths()
                .row_data(selected_controller as usize)
                .unwrap()
                .to_string();
            if controller_path.is_empty() {
                config.input.controller_assignment[i] = None;
            } else {
                config.input.controller_assignment[i] = Some(controller_path);
            }
        }
    }
}

fn about_window(app: &AppWindow) {
    app.on_wiki_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64/wiki");
    });
    app.on_discord_button_clicked(move || {
        open_uri("https://discord.gg/9RGXq8W8JQ");
    });
    app.on_patreon_button_clicked(move || {
        open_uri("https://patreon.com/loganmc10");
    });
    app.on_github_sponsors_button_clicked(move || {
        open_uri("https://github.com/sponsors/loganmc10");
    });
    app.on_source_code_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64");
    });
    app.on_newversion_button_clicked(move || {
        open_uri("https://github.com/gopher64/gopher64/releases/latest");
    });
    app.set_version(format!("Version: {}", env!("GIT_DESCRIBE")).into());

    //flatpak, itch.io, and android have their own update checking mechanism
    if std::env::var("FLATPAK_ID").is_err()
        && std::env::var("ITCHIO_APP").is_err()
        && cfg!(not(target_os = "android"))
    {
        check_latest_version(app.as_weak());
    }
}

pub fn app_window(
    app: &AppWindow,
    is_android: bool,
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    let no_intro_map_clone = no_intro_map.clone();
    tokio::spawn(async move {
        load_no_intro(no_intro_map_clone).await;
    });

    // The Android SAF folder-scan JNI callback (nativeOnFolderScanned) needs the
    // No-Intro map but can't be handed it as a param, so publish it here.
    #[cfg(target_os = "android")]
    let _ = NO_INTRO_MAP.set(no_intro_map.clone());

    retroachievements::init_client(false, false, false);
    app.set_is_android(is_android);
    about_window(app);
    ui::retroachievements::ra_window(app);
    {
        let config = ui::config::Config::new();
        settings_window(app, &config);
        controller_window(app, &config);
        local_game_window(app, &config, no_intro_map.clone());
    }
    ui::netplay::netplay_window(app, no_intro_map.clone());
    ui::cheats::cheats_window(app, no_intro_map);

    #[cfg(not(target_os = "android"))]
    {
        let weak_app = app.as_weak();
        app.window().on_close_requested(move || {
            weak_app
                .upgrade_in_event_loop(move |handle| {
                    save_settings(&handle);
                    handle.invoke_netplay_close();
                })
                .unwrap();
            slint::CloseRequestResponse::HideWindow
        });
    }

    app.run().unwrap();
    retroachievements::shutdown_client();
}

pub fn run_rom(
    file_path: std::path::PathBuf,
    game_settings: ui::GameSettings,
    netplay: Option<NetplayDevice>,
    weak: slint::Weak<AppWindow>,
) {
    #[cfg(target_os = "android")]
    ui::android::run_rom(file_path, game_settings, netplay, weak);

    #[cfg(not(target_os = "android"))]
    tokio::spawn(async move {
        weak.upgrade_in_event_loop(move |handle| handle.set_game_running(true))
            .unwrap();

        let cli_path = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(format!("{}-cli", env!("CARGO_PKG_NAME")));
        let cmd_path = if cfg!(target_os = "macos") && cli_path.exists() {
            cli_path
        } else {
            std::env::current_exe().unwrap()
        };
        let mut command = tokio::process::Command::new(cmd_path);
        command.args([
            "--overclock",
            &game_settings.overclock.to_string(),
            "--disable-expansion-pak",
            &game_settings.disable_expansion_pak.to_string(),
        ]);
        if let Some(slot) = game_settings.load_savestate_slot {
            command.args(["--load-state", &slot.to_string()]);
        }
        let cheats_path = ui::get_dirs().cache_dir.join("cheats.json");
        if let Some(netplay_device) = netplay {
            let f = std::fs::File::create(&cheats_path).unwrap();
            serde_json::to_writer_pretty(f, &game_settings.cheats).unwrap();

            command.args([
                "--netplay-server-addr",
                &netplay_device.server_addr,
                "--netplay-player-number",
                &netplay_device.player_number.to_string(),
                "--netplay-number-of-players",
                &netplay_device.number_of_players.to_string(),
                "--netplay-input-delay",
                &netplay_device.input_delay.to_string(),
                "--cheats",
                cheats_path.to_str().unwrap(),
            ]);
        }

        let success = command
            .arg(file_path.to_str().unwrap())
            .status()
            .await
            .unwrap()
            .success();

        if !success {
            eprintln!("Failed to run game");
        }

        let _ = std::fs::remove_file(cheats_path);

        weak.upgrade_in_event_loop(move |handle| {
            if let Some(rom_dir) = file_path.parent().unwrap().to_str() {
                handle.set_rom_dir(rom_dir.into());
            }
            if success {
                update_recent_roms(&handle, file_path);
            }
            handle.set_game_running(false);
        })
        .unwrap();
    });
}

fn decode_path(path: &str) -> String {
    #[cfg(target_os = "android")]
    return ui::android::decode_path(path);
    #[cfg(not(target_os = "android"))]
    return path.to_string();
}

pub fn update_recent_roms(app: &AppWindow, file_path: std::path::PathBuf) {
    let recent_roms = slint::VecModel::default();
    recent_roms.push((
        file_path.to_str().unwrap().into(),
        std::path::Path::new(&decode_path(file_path.to_str().unwrap()))
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .into(),
    ));

    for rom in app.get_recent_roms().iter() {
        if rom.0 != file_path.to_str().unwrap() && recent_roms.row_count() < 5 && rom_exists(&rom.0)
        {
            recent_roms.push(rom);
        }
    }
    app.set_recent_roms(slint::ModelRc::from(std::rc::Rc::new(recent_roms)));
}

pub async fn get_nointro_name(
    rom: &[u8],
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) -> String {
    let hash = device::cart::rom::calculate_hash(rom).to_lowercase();
    if let Some(name) = no_intro_map.lock().await.get(&hash) {
        name.clone()
    } else {
        ui::storage::get_game_name(rom)
    }
}

async fn load_no_intro(
    no_intro_map: std::sync::Arc<tokio::sync::Mutex<rustc_hash::FxHashMap<String, String>>>,
) {
    let mut reader = quick_xml::Reader::from_str(include_str!(
        "../../data/ui/Nintendo - Nintendo 64 (DB Export) (20260609-194259).xml"
    ));
    let mut current_game = String::new();
    let mut map = no_intro_map.lock().await;
    let mut xml_version = quick_xml::XmlVersion::Implicit1_0;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Decl(e)) => {
                if let Ok(version) = e.xml_version() {
                    xml_version = version;
                }
            }
            Ok(quick_xml::events::Event::Start(e)) => {
                if e.name().as_ref() == b"game"
                    && let Ok(Some(name_attribute)) = e.try_get_attribute("name")
                    && let Ok(normalized_value) = name_attribute.normalized_value(xml_version)
                {
                    current_game = normalized_value.into_owned();
                }
            }
            Ok(quick_xml::events::Event::End(e)) => {
                if e.name().as_ref() == b"game" {
                    current_game.clear();
                }
            }
            Ok(quick_xml::events::Event::Empty(e)) => {
                if e.name().as_ref() == b"file"
                    && let Ok(Some(format_attribute)) = e.try_get_attribute("format")
                    && let Ok(normalized_value) = format_attribute.normalized_value(xml_version)
                    && normalized_value.as_ref() == "BigEndian"
                    && let Ok(Some(sha256_attribute)) = e.try_get_attribute("sha256")
                    && let Ok(normalized_sha256) = sha256_attribute.normalized_value(xml_version)
                    && !current_game.is_empty()
                {
                    map.insert(normalized_sha256.to_lowercase(), current_game.clone());
                }
            }
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(quick_xml::events::Event::Eof) => break,
            _ => (),
        }
    }
}

pub async fn select_rom(rom_dir: slint::SharedString) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    return ui::android::select_rom(rom_dir).await;

    #[cfg(not(target_os = "android"))]
    {
        if !rom_dir.is_empty() && std::fs::exists(&rom_dir).unwrap_or(false) {
            rfd::AsyncFileDialog::new().set_directory(rom_dir)
        } else {
            rfd::AsyncFileDialog::new()
        }
        .set_title("Select ROM")
        .add_filter("ROM files", &N64_EXTENSIONS)
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
    }
}

pub async fn select_gb_rom(player: i32) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    return ui::android::select_gb_rom(player).await;

    #[cfg(not(target_os = "android"))]
    {
        rfd::AsyncFileDialog::new()
            .set_title(format!("GB ROM P{}", player + 1))
            .add_filter("GB ROM files", &["gb", "gbc", "GB", "GBC"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }
}

pub async fn select_gb_ram(player: i32) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    return ui::android::select_gb_ram(player).await;

    #[cfg(not(target_os = "android"))]
    {
        rfd::AsyncFileDialog::new()
            .set_title(format!("GB RAM P{}", player + 1))
            .add_filter("GB RAM files", &["sav", "ram", "srm", "SAV", "RAM", "SRM"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }
}

fn open_rom(app: &AppWindow) {
    let select_rom = select_rom(app.get_rom_dir());

    let overclock = app.get_overclock_n64_cpu();
    let disable_expansion_pak = app.get_disable_expansion_pak();

    let weak = app.as_weak();
    tokio::spawn(async move {
        if let Some(file) = select_rom.await {
            run_rom(
                file,
                ui::GameSettings {
                    overclock,
                    disable_expansion_pak,
                    cheats: rustc_hash::FxHashMap::default(), // will be filled in later
                    load_savestate_slot: None,
                },
                None,
                weak,
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_filter_arms() {
        // (title_lc, needle, filter, favorite, homebrew)
        assert!(game_matches("mario", "", 0, false, false)); // All shows everything
        assert!(game_matches("mario", "", 1, true, false));
        assert!(!game_matches("mario", "", 1, false, false)); // Favorites
        assert!(game_matches("mario", "", 2, false, true));
        assert!(!game_matches("mario", "", 2, false, false)); // Homebrew
        assert!(game_matches("mario kart", "kart", 0, false, false));
        assert!(!game_matches("zelda", "kart", 0, false, false)); // search excludes non-matches
    }

    // Proves savestate_slots() reproduces the emulator's own on-disk path
    // (storage.rs writes `states/<prefix>-<sha256>.state<slot>`).
    #[test]
    fn savestate_slot_detection_matches_emulator_path() {
        // Minimal valid z64 ROM: big-endian magic + a header name at 0x20.
        let mut rom = vec![0u8; 0x1000];
        rom[0..4].copy_from_slice(&0x8037_1240u32.to_be_bytes());
        rom[0x20..0x2A].copy_from_slice(b"G64TESTROM");
        let rom_path = std::env::temp_dir().join("g64_parity_slot_test.z64");
        std::fs::write(&rom_path, &rom).unwrap();

        // Independently build the slot-3 path the way storage.rs does, and create it.
        let contents = crate::device::get_rom_contents(&rom_path).unwrap();
        let hash = crate::device::cart::rom::calculate_hash(&contents);
        let name = ui::storage::get_game_name(&contents);
        let states = ui::get_dirs().data_dir.join("states");
        std::fs::create_dir_all(&states).unwrap();
        let slot3 = states.join(format!("{name}-{hash}.state3"));
        std::fs::write(&slot3, b"x").unwrap();

        let slots = savestate_slots(rom_path.to_str().unwrap());
        let _ = std::fs::remove_file(&slot3);
        let _ = std::fs::remove_file(&rom_path);

        assert_eq!(slots.len(), 10);
        assert!(slots[3], "slot 3 was written and must be detected");
        assert!(!slots[0] && !slots[5], "unwritten slots stay empty");
    }
}
