#![deny(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod cheats;
mod device;
mod netplay;
mod retroachievements;
mod savestates;
mod ui;
use clap::Parser;
use std::io::Error;
use ui::gui;

/// N64 emulator
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    game: Option<String>,
    #[arg(short, long)]
    fullscreen: bool,
    #[arg(long)]
    overclock: Option<bool>,
    #[arg(long)]
    disable_expansion_pak: Option<bool>,
    #[arg(long, value_name = "CHEATS_FILE", hide = true)]
    cheats: Option<String>,
    #[arg(long, value_name = "NETPLAY_PEER_ADDR", hide = true)]
    netplay_peer_addr: Option<String>,
    #[arg(long, value_name = "NETPLAY_PLAYER_NUMBER", hide = true)]
    netplay_player_number: Option<u8>,
    #[arg(long, value_name = "GB_ROM_PATH", hide = true)]
    gb_rom: Option<Vec<String>>,
    #[arg(long, value_name = "GB_RAM_PATH", hide = true)]
    gb_ram: Option<Vec<String>>,
    #[arg(
        long,
        value_name = "PROFILE_NAME",
        help = "Create a new input profile (keyboard/gamepad mappings)"
    )]
    configure_input_profile: Option<String>,
    #[arg(long, help = "Use DirectInput when configuring a new input profile")]
    use_dinput: bool,
    #[arg(
        long,
        value_name = "DEADZONE_PERCENTAGE",
        help = "Used along with --configure-input-profile to set the deadzone for analog sticks"
    )]
    deadzone: Option<i32>,
    #[arg(
        long,
        value_name = "PROFILE_NAME",
        help = "Must also specify --port. Used to bind a previously created profile to a port"
    )]
    bind_input_profile: Option<String>,
    #[arg(
        long,
        help = "Lists connected controllers which can be used in --assign-controller"
    )]
    list_controllers: bool,
    #[arg(
        long,
        value_name = "CONTROLLER_NUMBER",
        help = "Must also specify --port. Used to assign a controller listed in --list-controllers to a port"
    )]
    assign_controller: Option<i32>,
    #[arg(
        long,
        value_name = "PORT",
        help = "Valid values: 1-4. To be used alongside --bind-input-profile and --assign-controller"
    )]
    port: Option<usize>,
    #[arg(
        long,
        help = "Clear all input profile bindings and controller assignments"
    )]
    clear_input_bindings: bool,
    #[arg(
        long,
        value_name = "SLOT",
        help = "Load savestate from slot 0-9 when starting the game"
    )]
    load_state: Option<u32>,
    #[arg(
        long = "ra-username",
        value_name = "USERNAME",
        help = "Username for RetroAchievements"
    )]
    ra_username: Option<String>,
    #[arg(
        long = "ra-token",
        value_name = "TOKEN",
        help = "Token for RetroAchievements",
        hide = true
    )]
    ra_token: Option<String>,
    #[arg(
        long = "ra-password",
        value_name = "PASSWORD",
        help = "Password for RetroAchievements"
    )]
    ra_password: Option<String>,
    #[arg(
        long = "ra-hardcore",
        help = "Enable Hardcore mode for RetroAchievements"
    )]
    ra_hardcore: bool,
    #[arg(
        long = "ra-challenge",
        help = "Enable Challenge Indicators for RetroAchievements"
    )]
    ra_challenge: bool,
    #[arg(
        long = "ra-leaderboard",
        help = "Enable Leaderboard Trackers for RetroAchievements"
    )]
    ra_leaderboard: bool,
}

#[tokio::main(worker_threads = 4)]
async fn main() -> std::io::Result<()> {
    let dirs = ui::get_dirs();

    std::fs::create_dir_all(dirs.config_dir)?;
    std::fs::create_dir_all(dirs.cache_dir)?;
    std::fs::create_dir_all(dirs.data_dir.join("saves"))?;
    std::fs::create_dir_all(dirs.data_dir.join("states"))?;
    let args = Args::parse();
    if let Some(game) = args.game {
        let file_path = std::path::Path::new(&game);
        let Some(rom_contents) = device::get_rom_contents(file_path) else {
            return Err(Error::other(format!(
                "Could not read ROM file: {}",
                file_path.display()
            )));
        };

        if let Some(slot) = args.load_state
            && slot > 9
        {
            return Err(Error::other("Savestate slot must be between 0 and 9"));
        }

        let mut device = device::Device::new();
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
        let cheats = if let Some(cheats_file) = args.cheats
            && let Ok(data) = std::fs::read(cheats_file)
            && let Ok(cheats) = serde_json::from_slice(&data)
        {
            cheats
        } else {
            let game_crc = ui::storage::get_game_crc(&rom_contents);
            ui::config::Cheats::new()
                .cheats
                .get(&game_crc)
                .cloned()
                .unwrap_or_default()
        };

        if cfg!(ra_hardcore_enabled) {
            retroachievements::init_client(
                args.ra_hardcore,
                args.ra_challenge,
                args.ra_leaderboard,
            );
        } else {
            retroachievements::init_client(false, args.ra_challenge, args.ra_leaderboard);
        }
        let mut shutdown_tx = None;

        if let Some(peer_addr) = args.netplay_peer_addr
            && let Some(player_number) = args.netplay_player_number
        {
            device.netplay = Some(netplay::init(peer_addr.parse().unwrap(), player_number));
        } else {
            if let Some(gb_roms) = &args.gb_rom
                && let Some(gb_rams) = &args.gb_ram
            {
                for i in 0..4 {
                    if let Some(gb_rom) = gb_roms.get(i)
                        && let Some(gb_ram) = gb_rams.get(i)
                    {
                        device.transferpaks[i].cart.rom = std::fs::read(gb_rom).unwrap();
                        device.transferpaks[i].cart.ram = std::fs::read(gb_ram).unwrap();
                    }
                }
            }

            if device.ui.config.emulation.usb {
                (shutdown_tx, device.ui.usb) = ui::usb::init();
            }

            if let Some(username) = args.ra_username {
                let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                if let Some(password) = args.ra_password {
                    retroachievements::login_user(username, password, tx);
                } else if let Some(token) = args.ra_token {
                    retroachievements::login_token_user(username, token, tx);
                } else {
                    tx.send(false).unwrap();
                }

                rx.await.unwrap();
            }
        }

        retroachievements::load_game(&rom_contents, rom_contents.len()).await;
        device::run_game(
            &mut device,
            &rom_contents,
            ui::gui::GameSettings {
                overclock,
                disable_expansion_pak,
                cheats,
                load_savestate_slot: args.load_state,
            },
        );

        if device.netplay.is_some() {
            netplay::close(&mut device);
        } else if let Some(gb_rams) = &args.gb_ram {
            for i in 0..4 {
                if let Some(gb_ram) = gb_rams.get(i) {
                    tokio::fs::write(gb_ram, &device.transferpaks[i].cart.ram)
                        .await
                        .unwrap();
                }
            }
        }
        if let Some(shutdown_tx) = &shutdown_tx {
            ui::usb::close(shutdown_tx);
        }
    } else if std::env::args().count() > 1 {
        let mut ui = ui::Ui::new();

        if args.clear_input_bindings {
            ui::input::clear_bindings(&mut ui);
            return Ok(());
        }
        if let Some(port) = args.port
            && !(1..=4).contains(&port)
        {
            return Err(Error::other("Port must be between 1 and 4"));
        }
        if args.list_controllers {
            let controllers = ui::input::get_controller_names(&ui);
            for (i, controller) in controllers.iter().enumerate() {
                println!("Controller {i}: {controller}");
            }
            return Ok(());
        }
        if let Some(profile) = args.configure_input_profile {
            ui::input::configure_input_profile(
                &mut ui,
                profile,
                args.use_dinput,
                args.deadzone.unwrap_or(ui::input::DEADZONE_DEFAULT),
            );
            return Ok(());
        }
        if let Some(assign_controller) = args.assign_controller {
            let Some(port) = args.port else {
                return Err(Error::other("Must specify port number"));
            };
            ui::input::assign_controller(&mut ui, assign_controller, port);
        }
        if let Some(profile) = args.bind_input_profile {
            let Some(port) = args.port else {
                return Err(Error::other("Must specify port number"));
            };
            ui::input::bind_input_profile(&mut ui, profile, port);
        }
    } else {
        retroachievements::init_client(false, false, false);
        gui::app_window();
        retroachievements::shutdown_client();
    }

    Ok(())
}
