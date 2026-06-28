use crate::retroachievements;
use crate::ui;
#[cfg(target_os = "android")]
use crate::ui::android;
use parking_lot::Mutex;
use slint::Model;
#[cfg(not(target_os = "android"))]
use slint::winit_030::WinitWindowAccessor;
use std::sync::LazyLock;

slint::include_modules!();

#[cfg(not(target_os = "android"))]
pub const N64_EXTENSIONS: [&str; 6] = ["n64", "v64", "z64", "N64", "V64", "Z64"];

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

// Which save-state slots (0-9) already have data on disk for this ROM.
// Mirrors the emulator's path: states/<game-name-or-id>-<sha256(rom)>.state<slot>.
fn savestate_slots(path: &str) -> Vec<bool> {
    // ponytail: reads + hashes the whole ROM on detail-open (one game, lazy); cache if it ever stalls
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

fn set_detail_slots(handle: &AppWindow, path: &str) {
    let slots = savestate_slots(path);
    handle.set_library_detail_slots(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(slots),
    )));
}

fn set_detail_media(handle: &AppWindow, path: &str) {
    let snap = ui::boxart::snap_path(path).and_then(|p| load_art(&p));
    let title = ui::boxart::title_path(path).and_then(|p| load_art(&p));
    // hero banner: prefer the gameplay snap, then the title screen
    let (hero, has_hero) = match snap.clone().or_else(|| title.clone()) {
        Some(img) => (img, true),
        None => (slint::Image::default(), false),
    };
    handle.set_library_detail_hero(hero);
    handle.set_library_detail_has_hero(has_hero);
    let (s, hs) = snap.map_or((slint::Image::default(), false), |i| (i, true));
    handle.set_library_detail_snap(s);
    handle.set_library_detail_has_snap(hs);
    let (t, ht) = title.map_or((slint::Image::default(), false), |i| (i, true));
    handle.set_library_detail_title_art(t);
    handle.set_library_detail_has_title(ht);
}

// Single owner for opening the detail view: reset tab/slot, set slots + cached
// media, then show it; the snap/title fetch runs off-thread and re-applies only
// if the detail still shows this same game.
fn open_detail(handle: &AppWindow, path: &str) {
    handle.set_library_detail_tab(0);
    handle.set_library_detail_slot(-1);
    set_detail_slots(handle, path);
    set_detail_media(handle, path);
    handle.set_library_detail_open(true);
    {
        let p = path.to_string();
        let title = LIBRARY
            .lock()
            .iter()
            .find(|m| m.path == p)
            .map(|m| m.title.clone());
        if let Some(title) = title {
            let weak = handle.as_weak();
            tokio::spawn(async move {
                let index = ui::boxart::load_index().await;
                ui::boxart::resolve_media(&index, &p, &title).await;
                let _ = weak.upgrade_in_event_loop(move |h| {
                    if !h.get_library_detail_open() {
                        return;
                    }
                    let sel = h.get_library_selected().max(0) as usize;
                    if h.get_games()
                        .row_data(sel)
                        .map(|g| g.path.as_str() == p.as_str())
                        .unwrap_or(false)
                    {
                        set_detail_media(&h, &p);
                    }
                });
            });
        }
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

#[derive(Clone)]
struct GameMeta {
    path: String,
    title: String,   // raw filename stem — box-art match key + search
    display: String, // tags stripped, for the card
    subtitle: String,
    recent: bool,
}

// Full library (recent ROMs + scanned ROM folder); shared with async tasks.
static LIBRARY: LazyLock<Mutex<Vec<GameMeta>>> = LazyLock::new(|| Mutex::new(Vec::new()));

// Favorited ROM paths (mirrors config.favorites; UI thread + persisted).
static FAVORITES: LazyLock<Mutex<std::collections::HashSet<String>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashSet::new()));

thread_local! {
    // Decoded box-art images, reused across refreshes (UI thread only).
    static IMG_CACHE: std::cell::RefCell<std::collections::HashMap<String, slint::Image>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn load_art(path: &std::path::Path) -> Option<slint::Image> {
    let key = path.to_string_lossy().to_string();
    IMG_CACHE.with(|c| {
        if let Some(img) = c.borrow().get(&key) {
            return Some(img.clone());
        }
        match slint::Image::load_from_path(path) {
            Ok(img) => {
                c.borrow_mut().insert(key, img.clone());
                Some(img)
            }
            Err(_) => None,
        }
    })
}

// Library filter predicate (0 All · 1 Favorites · 2 Recent · 3 Homebrew + search).
fn game_matches(m: &GameMeta, needle: &str, filter: i32, favorite: bool, homebrew: bool) -> bool {
    (needle.is_empty() || m.title.to_lowercase().contains(needle))
        && match filter {
            1 => favorite,
            2 => m.recent,
            3 => homebrew,
            _ => true,
        }
}

fn apply_library(handle: &AppWindow) {
    let needle = handle.get_library_search().to_lowercase();
    let filter = handle.get_library_filter();
    let favs = FAVORITES.lock();
    let items: Vec<GameEntry> = LIBRARY
        .lock()
        .iter()
        .filter(|m| {
            game_matches(
                m,
                &needle,
                filter,
                favs.contains(&m.path),
                ui::boxart::is_homebrew(&m.path),
            )
        })
        .map(|m| {
            let (art, has_art) = match ui::boxart::art_path(&m.path).and_then(|p| load_art(&p)) {
                Some(img) => (img, true),
                None => (slint::Image::default(), false),
            };
            GameEntry {
                path: m.path.as_str().into(),
                title: m.display.as_str().into(),
                subtitle: m.subtitle.as_str().into(),
                favorite: favs.contains(&m.path),
                art,
                has_art,
            }
        })
        .collect();
    handle.set_games(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(items),
    )));
}

// Display title (trailing (...) / [...] tag groups stripped — same as the box-art
// matcher's normalize) + region, from a ROM filename stem. Build-time, no I/O.
fn title_and_region(stem: &str) -> (String, String) {
    let mut t = stem;
    if let Some(i) = t.find(" (") {
        t = &t[..i];
    }
    if let Some(i) = t.find(" [") {
        t = &t[..i];
    }
    let t = t.trim();
    let lower = stem.to_lowercase();
    let region = if lower.contains("usa") || lower.contains("(u)") {
        "USA"
    } else if lower.contains("world") {
        "World"
    } else if lower.contains("europe") || lower.contains("(e)") {
        "Europe"
    } else if lower.contains("japan") || lower.contains("(j)") {
        "Japan"
    } else {
        ""
    };
    (
        if t.is_empty() {
            stem.to_string()
        } else {
            t.to_string()
        },
        region.to_string(),
    )
}

fn meta_from_path(path: &str) -> GameMeta {
    let decoded = decode_path(path);
    let stem = std::path::Path::new(&decoded)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let (display, subtitle) = title_and_region(stem);
    GameMeta {
        path: path.to_string(),
        title: stem.to_string(),
        display,
        subtitle,
        recent: true,
    }
}

fn recent_meta(config: &ui::config::Config) -> Vec<GameMeta> {
    config
        .recent_roms
        .iter()
        .filter(|x| rom_exists(x))
        .map(|x| meta_from_path(x))
        .collect()
}

#[cfg(not(target_os = "android"))]
fn scan_roms(dir: &std::path::Path) -> Vec<GameMeta> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0u32)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if depth < 4 {
                    stack.push((path, depth + 1));
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && N64_EXTENSIONS.contains(&ext)
                && let Some(p) = path.to_str()
            {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let (display, subtitle) = title_and_region(stem);
                out.push(GameMeta {
                    path: p.to_string(),
                    title: stem.to_string(),
                    display,
                    subtitle,
                    recent: false,
                });
            }
        }
        if out.len() > 5000 {
            break;
        }
    }
    out
}

async fn fetch_all_art(weak: slint::Weak<AppWindow>) {
    let games: Vec<GameMeta> = LIBRARY.lock().clone();
    let index = ui::boxart::load_index().await;
    let mut last = std::time::Instant::now();
    for meta in &games {
        let arted = ui::boxart::resolve_and_cache(&index, &meta.path, &meta.title).await;
        ui::boxart::mark_homebrew(&meta.path).await;
        if arted && last.elapsed() > std::time::Duration::from_millis(400) {
            let _ = weak.upgrade_in_event_loop(|h| apply_library(&h));
            last = std::time::Instant::now();
        }
    }
    let _ = weak.upgrade_in_event_loop(|h| apply_library(&h));
    ui::boxart::persist_crc_cache();
}

#[cfg(not(target_os = "android"))]
async fn rescan_library(weak: slint::Weak<AppWindow>, dir: std::path::PathBuf) {
    let scanned = scan_roms(&dir);
    {
        let mut lib = LIBRARY.lock();
        let mut seen: std::collections::HashSet<String> =
            lib.iter().map(|g| g.path.clone()).collect();
        for g in scanned {
            if seen.insert(g.path.clone()) {
                lib.push(g);
            }
        }
        lib.sort_by_key(|a| a.title.to_lowercase());
    }
    let _ = weak.upgrade_in_event_loop(|h| apply_library(&h));
    fetch_all_art(weak).await;
}

#[cfg(not(target_os = "android"))]
thread_local! {
    static MENU_PADS: std::cell::RefCell<Vec<*mut sdl3_sys::gamepad::SDL_Gamepad>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static MENU_TIMER: std::cell::RefCell<Option<slint::Timer>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(not(target_os = "android"))]
fn menu_gamepad_button(handle: &AppWindow, button: i32) {
    use sdl3_sys::gamepad as gp;
    let count = handle.get_games().row_count() as i32;
    if count == 0 {
        return;
    }
    let detail = handle.get_library_detail_open();
    if button == i32::from(gp::SDL_GAMEPAD_BUTTON_SOUTH) {
        let sel = handle.get_library_selected().clamp(0, count - 1);
        if detail {
            if let Some(game) = handle.get_games().row_data(sel as usize) {
                handle.set_library_detail_open(false);
                handle.invoke_launch_game(game.path, handle.get_library_detail_slot());
            }
        } else if let Some(game) = handle.get_games().row_data(sel as usize) {
            handle.invoke_detail_opened(game.path);
        }
        return;
    }
    if button == i32::from(gp::SDL_GAMEPAD_BUTTON_EAST) {
        handle.set_library_detail_open(false);
        return;
    }
    if detail {
        return; // d-pad does nothing while the detail view is open
    }
    let cols = handle.get_library_columns().max(1);
    let mut sel = handle.get_library_selected();
    if button == i32::from(gp::SDL_GAMEPAD_BUTTON_DPAD_RIGHT) {
        sel += 1;
    } else if button == i32::from(gp::SDL_GAMEPAD_BUTTON_DPAD_LEFT) {
        sel -= 1;
    } else if button == i32::from(gp::SDL_GAMEPAD_BUTTON_DPAD_DOWN) {
        sel += cols;
    } else if button == i32::from(gp::SDL_GAMEPAD_BUTTON_DPAD_UP) {
        sel -= cols;
    } else {
        return;
    }
    handle.set_library_selected(sel.clamp(0, count - 1));
}

// Poll SDL gamepads on the UI thread to drive the library grid with a controller.
#[cfg(not(target_os = "android"))]
fn setup_menu_gamepad(app: &AppWindow) {
    ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);
    MENU_PADS.with(|pads| {
        if !pads.borrow().is_empty() {
            return;
        }
        let mut count = 0;
        let ids = unsafe { sdl3_sys::gamepad::SDL_GetGamepads(&mut count) };
        if !ids.is_null() {
            for i in 0..count as isize {
                let pad = unsafe { sdl3_sys::gamepad::SDL_OpenGamepad(*ids.offset(i)) };
                if !pad.is_null() {
                    pads.borrow_mut().push(pad);
                }
            }
            unsafe { sdl3_sys::stdinc::SDL_free(ids as *mut std::ffi::c_void) };
        }
    });

    let weak = app.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(80),
        move || {
            let Some(handle) = weak.upgrade() else {
                return;
            };
            // only drive the library grid; leave the SDL queue alone elsewhere
            // (so the input-profile wizard's poll isn't starved)
            if handle.get_nav() != 0 || handle.get_game_running() || handle.get_capturing() {
                return;
            }
            unsafe { sdl3_sys::events::SDL_PumpEvents() };
            let mut event: sdl3_sys::events::SDL_Event = Default::default();
            while unsafe { sdl3_sys::events::SDL_PollEvent(&mut event) } {
                if event.event_type() == sdl3_sys::events::SDL_EVENT_GAMEPAD_BUTTON_DOWN {
                    menu_gamepad_button(&handle, i32::from(unsafe { event.gbutton.button }));
                }
            }
        },
    );
    MENU_TIMER.with(|t| *t.borrow_mut() = Some(timer));
}

fn local_game_window(app: &AppWindow, config: &ui::config::Config) {
    app.set_recent_roms(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            config
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

    // ---- Library grid model (new UI) ----
    *FAVORITES.lock() = config.favorites.iter().cloned().collect();
    *LIBRARY.lock() = recent_meta(config);
    apply_library(app);

    let weak = app.as_weak();
    app.on_library_filter_changed(move |_search, _filter| {
        if let Some(handle) = weak.upgrade() {
            apply_library(&handle);
        }
    });

    let weak = app.as_weak();
    app.on_toggle_favorite(move |path| {
        let path = path.to_string();
        {
            let mut favs = FAVORITES.lock();
            if !favs.remove(&path) {
                favs.insert(path.clone());
            }
        }
        let mut config = ui::config::Config::new();
        config.favorites = FAVORITES.lock().iter().cloned().collect();
        drop(config);
        if let Some(handle) = weak.upgrade() {
            apply_library(&handle);
        }
    });

    // scan the ROM folder (if set) + auto-download box art
    #[cfg(not(target_os = "android"))]
    {
        let weak_scan = app.as_weak();
        let rom_dir = config.rom_dir.clone();
        tokio::spawn(async move {
            if !rom_dir.as_os_str().is_empty() && rom_dir.is_dir() {
                rescan_library(weak_scan, rom_dir).await;
            } else {
                fetch_all_art(weak_scan).await;
            }
        });

        let weak_folder = app.as_weak();
        app.on_library_scan_folder(move || {
            let start = weak_folder
                .upgrade()
                .map(|h| h.get_rom_dir())
                .unwrap_or_default();
            let weak_inner = weak_folder.clone();
            tokio::spawn(async move {
                let dialog = if !start.is_empty() && std::fs::exists(&start).unwrap_or(false) {
                    rfd::AsyncFileDialog::new().set_directory(&start)
                } else {
                    rfd::AsyncFileDialog::new()
                };
                if let Some(folder) = dialog.set_title("Select ROM Folder").pick_folder().await {
                    let dir = folder.path().to_path_buf();
                    let dir_str = dir.to_string_lossy().to_string();
                    #[cfg(target_os = "macos")]
                    ui::macos::remember_rom_dir(&dir_str);
                    let _ = weak_inner.upgrade_in_event_loop(move |h| {
                        h.set_rom_dir(dir_str.clone().into());
                        save_settings(&h);
                    });
                    rescan_library(weak_inner, dir).await;
                }
            });
        });
        setup_menu_gamepad(app);
    }

    // Android: no folder to scan; fetch box art for the recent-ROM library.
    #[cfg(target_os = "android")]
    {
        let weak_scan = app.as_weak();
        tokio::spawn(async move {
            fetch_all_art(weak_scan).await;
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
        let load_slot = if slot >= 0 { Some(slot as u32) } else { None };
        weak.upgrade_in_event_loop(move |handle| {
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
        if let Some(handle) = weak.upgrade() {
            open_detail(&handle, path.as_str());
        }
    });

    #[cfg(not(target_os = "android"))]
    {
        let saves_path = ui::get_dirs().data_dir.join("saves");
        app.on_saves_folder_button_clicked(move || {
            open_uri(&saves_path);
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
    app.set_dark_theme(config.video.dark_theme);
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

    if let Some(rom_dir_str) = config.rom_dir.to_str() {
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

#[cfg(not(target_os = "android"))]
fn update_capture_ui(
    weak: &slint::Weak<AppWindow>,
    capture_state: &std::rc::Rc<std::cell::RefCell<Option<ui::input::ProfileCapture>>>,
    capture_timer: &std::rc::Rc<std::cell::RefCell<Option<slint::Timer>>>,
) {
    let done = {
        let mut guard = capture_state.borrow_mut();
        match guard.as_mut() {
            Some(capture) if capture.is_done() => true,
            Some(capture) => {
                if let Some(handle) = weak.upgrade() {
                    handle.set_capture_label(capture.current_label().into());
                    handle.set_capture_index(capture.index() as i32);
                }
                false
            }
            None => false,
        }
    };
    if done {
        if let Some(capture) = capture_state.borrow_mut().take() {
            capture.finish_and_save();
        }
        if let Some(timer) = capture_timer.borrow().as_ref() {
            timer.stop();
        }
        if let Some(handle) = weak.upgrade() {
            handle.set_capturing(false);
            let config = ui::config::Config::new();
            update_input_profiles(&handle.as_weak(), &config);
        }
    }
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
    #[cfg(not(target_os = "android"))]
    let capture_state: std::rc::Rc<std::cell::RefCell<Option<ui::input::ProfileCapture>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    #[cfg(not(target_os = "android"))]
    let capture_timer: std::rc::Rc<std::cell::RefCell<Option<slint::Timer>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    #[cfg(not(target_os = "android"))]
    let cs_create = capture_state.clone();
    #[cfg(not(target_os = "android"))]
    let ct_create = capture_timer.clone();
    app.on_input_profile_creation_button_clicked(move || {
        let handle = weak_app.unwrap();
        let profile_name = handle.get_input_profile_name().to_string();
        let dinput = handle.get_input_dinput();
        let deadzone = handle.get_input_deadzone();
        handle.set_show_input_profile(false);

        #[cfg(target_os = "android")]
        ui::android::spawn_configure_input_profile(profile_name.into(), dinput, deadzone);

        #[cfg(not(target_os = "android"))]
        {
            if profile_name.is_empty() || profile_name == "default" {
                return;
            }
            let capture = ui::input::ProfileCapture::new(profile_name, dinput, deadzone);
            handle.set_capture_label(capture.current_label().into());
            handle.set_capture_index(0);
            handle.set_capturing(true);
            *cs_create.borrow_mut() = Some(capture);

            let weak_poll = handle.as_weak();
            let cs_poll = cs_create.clone();
            let ct_poll = ct_create.clone();
            let timer = slint::Timer::default();
            timer.start(
                slint::TimerMode::Repeated,
                std::time::Duration::from_millis(16),
                move || {
                    if let Some(capture) = cs_poll.borrow_mut().as_mut() {
                        capture.poll_pad();
                    }
                    update_capture_ui(&weak_poll, &cs_poll, &ct_poll);
                },
            );
            *ct_create.borrow_mut() = Some(timer);
        }
    });

    #[cfg(not(target_os = "android"))]
    {
        let weak_key = app.as_weak();
        let cs_key = capture_state.clone();
        let ct_key = capture_timer.clone();
        app.on_input_key_captured(move |text| {
            if let Some(capture) = cs_key.borrow_mut().as_mut()
                && let Some(scancode) = ui::input::slint_text_to_scancode(&text)
            {
                capture.capture_key(scancode);
            }
            update_capture_ui(&weak_key, &cs_key, &ct_key);
        });

        let weak_cancel = app.as_weak();
        let cs_cancel = capture_state.clone();
        let ct_cancel = capture_timer.clone();
        app.on_input_capture_cancel(move || {
            if let Some(capture) = cs_cancel.borrow_mut().take() {
                capture.cancel();
            }
            if let Some(timer) = ct_cancel.borrow().as_ref() {
                timer.stop();
            }
            if let Some(handle) = weak_cancel.upgrade() {
                handle.set_capturing(false);
            }
        });
    }

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
    config.rom_dir = app.get_rom_dir().to_string().into();
    #[cfg(target_os = "macos")]
    {
        config.macos_rom_dir_bookmark = ui::macos::rom_dir_bookmark();
    }
    config.video.integer_scaling = app.get_integer_scaling();
    config.video.ssaa = app.get_ssaa();
    config.video.fullscreen = app.get_fullscreen();
    config.video.widescreen = app.get_widescreen();
    config.video.vsync = app.get_vsync();
    config.video.crt = app.get_apply_crt_shader();
    config.video.dark_theme = app.get_dark_theme();
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

pub fn app_window(app: &AppWindow, is_android: bool) {
    retroachievements::init_client(false, false, false);
    app.set_is_android(is_android);
    about_window(app);
    ui::retroachievements::ra_window(app);
    {
        #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
        let mut config = ui::config::Config::new();
        #[cfg(target_os = "macos")]
        ui::macos::restore(&mut config);
        settings_window(app, &config);
        controller_window(app, &config);
        local_game_window(app, &config);
    }
    ui::netplay::netplay_window(app);
    ui::cheats::cheats_window(app);

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
        weak.upgrade_in_event_loop(move |handle| {
            handle.set_game_running(true);
            handle.window().set_minimized(true);
        })
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
            handle.window().set_minimized(false);
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

    // Android: the library *is* the recent-ROM list (no folder scan), so refresh
    // it and fetch art for any newly added ROM — desktop keeps its scanned library.
    #[cfg(target_os = "android")]
    {
        *LIBRARY.lock() = app
            .get_recent_roms()
            .iter()
            .filter(|r| rom_exists(&r.0))
            .map(|r| meta_from_path(&r.0))
            .collect();
        apply_library(app);
        let weak = app.as_weak();
        tokio::spawn(async move {
            fetch_all_art(weak).await;
        });
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

    // Proves savestate_slots() reproduces the emulator's own on-disk path
    // (storage.rs writes `states/<prefix>-<sha256>.state<slot>`).
    #[test]
    fn savestate_slot_detection_matches_emulator_path() {
        // Minimal valid z64 ROM: big-endian magic + a header name at 0x20.
        let mut rom = vec![0u8; 0x1000];
        rom[0..4].copy_from_slice(&0x8037_1240u32.to_be_bytes());
        rom[0x20..0x2A].copy_from_slice(b"G64TESTROM");
        let rom_path = std::env::temp_dir().join("g64_slot_test.z64");
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

    #[test]
    fn title_and_region_strips_tags() {
        assert_eq!(
            title_and_region("Super Mario 64 (USA)"),
            ("Super Mario 64".to_string(), "USA".to_string())
        );
        assert_eq!(
            title_and_region("Mario Kart 64 (Europe) (Rev A)"),
            ("Mario Kart 64".to_string(), "Europe".to_string())
        );
        assert_eq!(
            title_and_region("1080 Snowboarding (Japan, USA)"),
            ("1080 Snowboarding".to_string(), "USA".to_string())
        );
        // untagged (homebrew): title unchanged, no region
        assert_eq!(
            title_and_region("My Cool Homebrew"),
            ("My Cool Homebrew".to_string(), String::new())
        );
    }

    #[test]
    fn game_filter_arms() {
        let mk = |title: &str, recent: bool| GameMeta {
            path: title.to_string(),
            title: title.to_string(),
            display: title.to_string(),
            subtitle: String::new(),
            recent,
        };
        let mario = mk("Mario", false);
        let zelda = mk("Zelda", true);
        assert!(game_matches(&mario, "", 0, false, false)); // All
        assert!(
            game_matches(&mario, "", 1, true, false) && !game_matches(&mario, "", 1, false, false)
        ); // Favorites
        assert!(
            game_matches(&zelda, "", 2, false, false) && !game_matches(&mario, "", 2, false, false)
        ); // Recent
        assert!(
            game_matches(&mario, "", 3, false, true) && !game_matches(&mario, "", 3, false, false)
        ); // Homebrew
        assert!(
            game_matches(&mario, "mar", 0, false, false)
                && !game_matches(&zelda, "mar", 0, false, false)
        ); // search
    }
}
