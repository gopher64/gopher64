#![deny(warnings)]

mod cheats;
mod device;
mod netplay;
mod retroachievements;
mod savestates;
mod ui;
use clap::Parser;
#[cfg(target_os = "android")]
use slint::ComponentHandle;
use std::io::Error;

#[cfg(target_os = "android")]
use ui::android;

/// N64 emulator
#[derive(Parser, Debug)]
#[command(author, version=env!("GIT_DESCRIBE"), about, long_about = None, arg_required_else_help = if cfg!(feature = "gui") { false } else { true })]
pub struct Args {
    pub game: Option<String>,
    #[arg(short, long)]
    pub fullscreen: bool,
    #[arg(long)]
    pub overclock: Option<bool>,
    #[arg(long)]
    pub disable_expansion_pak: Option<bool>,
    #[arg(long, value_name = "CHEATS_FILE", hide = true)]
    pub cheats: Option<String>,
    #[arg(long, value_name = "NETPLAY_PEER_ADDR", hide = true)]
    pub netplay_peer_addr: Option<String>,
    #[arg(long, value_name = "NETPLAY_PLAYER_NUMBER", hide = true)]
    pub netplay_player_number: Option<u8>,
    #[arg(
        long,
        value_name = "PROFILE_NAME",
        help = "Create a new input profile (keyboard/gamepad mappings)"
    )]
    pub configure_input_profile: Option<String>,
    #[arg(long, help = "Use DirectInput when configuring a new input profile")]
    pub use_dinput: bool,
    #[arg(
        long,
        value_name = "DEADZONE_PERCENTAGE",
        help = "Used along with --configure-input-profile to set the deadzone for analog sticks"
    )]
    pub deadzone: Option<i32>,
    #[arg(
        long,
        value_name = "PROFILE_NAME",
        help = "Must also specify --port. Used to bind a previously created profile to a port"
    )]
    pub bind_input_profile: Option<String>,
    #[arg(
        long,
        help = "Lists connected controllers which can be used in --assign-controller"
    )]
    pub list_controllers: bool,
    #[arg(
        long,
        value_name = "CONTROLLER_NUMBER",
        help = "Must also specify --port. Used to assign a controller listed in --list-controllers to a port"
    )]
    pub assign_controller: Option<i32>,
    #[arg(
        long,
        value_name = "PORT",
        help = "Valid values: 1-4. To be used alongside --bind-input-profile and --assign-controller"
    )]
    pub port: Option<usize>,
    #[arg(
        long,
        help = "Clear all input profile bindings and controller assignments"
    )]
    pub clear_input_bindings: bool,
    #[arg(
        long,
        value_name = "SLOT",
        help = "Load savestate from slot 0-9 when starting the game"
    )]
    pub load_state: Option<u32>,
    #[arg(
        long = "ra-username",
        value_name = "USERNAME",
        help = "Username for RetroAchievements"
    )]
    pub ra_username: Option<String>,
    #[arg(
        long = "ra-password",
        value_name = "PASSWORD",
        help = "Password for RetroAchievements"
    )]
    pub ra_password: Option<String>,
}

pub async fn run(args: Args, arg_count: usize) -> std::io::Result<()> {
    let dirs = ui::get_dirs();

    std::fs::create_dir_all(&dirs.config_dir)?;
    std::fs::create_dir_all(&dirs.cache_dir)?;
    std::fs::create_dir_all(dirs.data_dir.join("saves"))?;
    std::fs::create_dir_all(dirs.data_dir.join("states"))?;

    ui::sdl_hints();

    if let Some(game) = args.game {
        let file_path = std::path::Path::new(&game).to_path_buf();
        let Some(rom_contents) = device::get_rom_contents(&file_path) else {
            return Err(Error::other(format!(
                "Could not read ROM file: {}",
                file_path.display()
            )));
        };

        let cheats = if let Some(cheats_file) = args.cheats {
            if let Ok(data) = std::fs::read(cheats_file)
                && let Ok(cheats) = serde_json::from_slice(&data)
            {
                cheats
            } else {
                return Err(Error::other("Could not read cheats file"));
            }
        } else {
            let game_crc = ui::storage::get_game_crc(&rom_contents);
            ui::config::Cheats::new()
                .cheats
                .get(&game_crc)
                .cloned()
                .unwrap_or_default()
        };

        if let Some(slot) = args.load_state
            && slot > 9
        {
            return Err(Error::other("Savestate slot must be between 0 and 9"));
        }

        let mut device = device::Device::new();

        device.ui.config.recent_roms.retain(|x| *x != game);
        device.ui.config.recent_roms.insert(0, game);
        device.ui.config.recent_roms.truncate(5);

        if args.fullscreen {
            device.ui.video.fullscreen = true;
        } else {
            device.ui.video.fullscreen = device.ui.config.video.fullscreen;
        }
        let overclock = args
            .overclock
            .unwrap_or(device.ui.config.emulation.overclock);
        let disable_expansion_pak = args
            .disable_expansion_pak
            .unwrap_or(device.ui.config.emulation.disable_expansion_pak);

        let mut shutdown_tx = None;
        let mut usb_handle = None;

        if let Some(peer_addr) = args.netplay_peer_addr
            && let Some(player_number) = args.netplay_player_number
        {
            device.netplay = Some(netplay::init(peer_addr.parse().unwrap(), player_number));
        } else {
            for i in 0..4 {
                if device.ui.config.input.transfer_pak[i]
                    && !device.ui.config.input.gb_rom_path[i].is_empty()
                    && !device.ui.config.input.gb_ram_path[i].is_empty()
                    && let Ok(rom) = std::fs::read(&device.ui.config.input.gb_rom_path[i])
                    && let Ok(ram) = std::fs::read(&device.ui.config.input.gb_ram_path[i])
                {
                    device::controller::gbcart::init(&mut device.transferpaks[i].cart, &rom, &ram);
                }
            }

            if device.ui.config.emulation.usb {
                (shutdown_tx, usb_handle, device.ui.usb) = ui::usb::init();
            }
        };

        let file_path = dirs.config_dir.join("retroachievements.json");
        let ra_config = if let ra_config = std::fs::read(&file_path).unwrap_or_default()
            && let ra_config =
                serde_json::from_slice::<retroachievements::RAConfig>(ra_config.as_ref())
                    .unwrap_or_default()
            && (ra_config.enabled || args.ra_username.is_some())
        {
            let username = args.ra_username.unwrap_or(ra_config.username.clone());
            retroachievements::init_client(
                if cfg!(ra_hardcore_enabled) {
                    ra_config.hardcore
                } else {
                    false
                },
                ra_config.challenge,
                ra_config.leaderboard,
            );

            let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
            if let Some(password) = args.ra_password {
                retroachievements::login_user(username, password, tx);
            } else if !ra_config.token.is_empty() {
                retroachievements::login_token_user(username, ra_config.token.clone(), tx);
            } else {
                tx.send(false).unwrap();
            }

            rx.await.unwrap();
            ra_config
        } else {
            retroachievements::RAConfig::default()
        };

        device::run_game(
            &mut device,
            &rom_contents,
            ui::GameSettings {
                overclock,
                disable_expansion_pak,
                cheats,
                load_savestate_slot: args.load_state,
            },
            ra_config,
        )
        .await;

        // on Android, the client is shut down in the app_window function
        #[cfg(not(target_os = "android"))]
        retroachievements::shutdown_client();

        if device.netplay.is_some() {
            netplay::close(&mut device);
        } else {
            for i in 0..4 {
                if device.ui.config.input.transfer_pak[i]
                    && !device.ui.config.input.gb_ram_path[i].is_empty()
                    && !device.transferpaks[i].cart.ram.is_empty()
                {
                    device::controller::gbcart::save(
                        &mut device.transferpaks[i].cart,
                        device.vi.elapsed_time as i64,
                        &device.ui.config.input.gb_ram_path[i],
                    );
                }
            }
        }
        if let Some(shutdown_tx) = &shutdown_tx {
            ui::usb::close(shutdown_tx, usb_handle).await;
        }
    } else if arg_count > 1 {
        let mut config = ui::config::Config::new();

        if let Some(profile) = args.configure_input_profile {
            ui::input::configure_input_profile(
                &mut config,
                profile,
                args.use_dinput,
                args.deadzone.unwrap_or(ui::input::DEADZONE_DEFAULT),
            );

            ui::sdl_close();
            return Ok(());
        } else {
            if args.clear_input_bindings {
                ui::input::clear_bindings(&mut config);
                return Ok(());
            }
            if let Some(port) = args.port
                && !(1..=4).contains(&port)
            {
                return Err(Error::other("Port must be between 1 and 4"));
            }

            ui::sdl_init(sdl3_sys::init::SDL_INIT_GAMEPAD);

            if args.list_controllers {
                let controllers = ui::input::get_controller_names();
                for (i, controller) in controllers.iter().enumerate() {
                    println!("Controller {i}: {controller}");
                }
            } else {
                if let Some(assign_controller) = args.assign_controller {
                    let Some(port) = args.port else {
                        ui::sdl_close();
                        return Err(Error::other("Must specify port number"));
                    };
                    ui::input::assign_controller(&mut config, assign_controller - 1, port);
                }
                if let Some(profile) = args.bind_input_profile {
                    let Some(port) = args.port else {
                        ui::sdl_close();
                        return Err(Error::other("Must specify port number"));
                    };
                    ui::input::bind_input_profile(&mut config, profile, port);
                }
            }
        }
    } else {
        #[cfg(feature = "gui")]
        {
            let app = ui::gui::AppWindow::new().unwrap();
            ui::gui::app_window(&app, false);
        }
    }

    ui::sdl_close();
    Ok(())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
#[tokio::main(worker_threads = 4)]
async fn android_main(app: slint::android::AndroidApp) {
    slint::android::init_with_event_listener(app.clone(), move |event| match event {
        slint::android::android_activity::PollEvent::Main(main_event) => match main_event {
            slint::android::android_activity::MainEvent::TerminateWindow { .. } => {
                if let Ok(weak_app_window) = android::WEAK_SLINT_WINDOW.lock()
                    && let Some(weak_app_window) = weak_app_window.as_ref()
                {
                    weak_app_window
                        .upgrade_in_event_loop(move |handle| ui::gui::save_settings(&handle))
                        .unwrap();
                }
            }
            _ => {}
        },
        _ => {}
    })
    .unwrap();
    let app_window = ui::gui::AppWindow::new().unwrap();
    *android::WEAK_SLINT_WINDOW.lock().unwrap() = Some(app_window.as_weak());

    *android::ANDROID_APP.lock().unwrap() = Some(app);

    let dirs = ui::get_dirs();

    std::fs::create_dir_all(&dirs.config_dir).unwrap();
    std::fs::create_dir_all(&dirs.cache_dir).unwrap();
    std::fs::create_dir_all(dirs.data_dir.join("saves")).unwrap();
    std::fs::create_dir_all(dirs.data_dir.join("states")).unwrap();

    ui::gui::app_window(&app_window, true);
    *android::WEAK_SLINT_WINDOW.lock().unwrap() = None;
    *android::ANDROID_APP.lock().unwrap() = None;
}
