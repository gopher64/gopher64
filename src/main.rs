#![deny(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod cheats;
mod device;
mod netplay;
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
    #[arg(
        short,
        long,
        value_name = "PROFILE_NAME",
        help = "Create a new input profile (keyboard/gamepad mappings)"
    )]
    configure_input_profile: Option<String>,
    #[arg(
        short,
        long,
        help = "Use DirectInput when configuring a new input profile"
    )]
    use_dinput: bool,
    #[arg(
        short,
        long,
        value_name = "DEADZONE_PERCENTAGE",
        help = "Used along with --configure-input-profile to set the deadzone for analog sticks"
    )]
    deadzone: Option<i32>,
    #[arg(
        short,
        long,
        value_name = "PROFILE_NAME",
        help = "Must also specify --port. Used to bind a previously created profile to a port"
    )]
    bind_input_profile: Option<String>,
    #[arg(
        short,
        long,
        help = "Lists connected controllers which can be used in --assign-controller"
    )]
    list_controllers: bool,
    #[arg(
        short,
        long,
        value_name = "CONTROLLER_NUMBER",
        help = "Must also specify --port. Used to assign a controller listed in --list-controllers to a port"
    )]
    assign_controller: Option<i32>,
    #[arg(
        short,
        long,
        value_name = "PORT",
        help = "Valid values: 1-4. To be used alongside --bind-input-profile and --assign-controller"
    )]
    port: Option<usize>,
    #[arg(
        short = 'z',
        long,
        help = "Clear all input profile bindings and controller assignments"
    )]
    clear_input_bindings: bool,
}

fn main() -> std::io::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .thread_name("n64")
        .thread_stack_size(env!("N64_STACK_SIZE").parse().unwrap())
        .build()
        .unwrap();

    let dirs = ui::get_dirs();

    std::fs::create_dir_all(dirs.config_dir)?;
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

        let handle = runtime.spawn(async move {
            let mut device = device::Device::new();
            let overclock = device.ui.config.emulation.overclock;
            let disable_expansion_pak = device.ui.config.emulation.disable_expansion_pak;

            let game_cheats = {
                let game_crc = ui::storage::get_game_crc(&rom_contents);
                let cheats = ui::config::Cheats::new();
                cheats.cheats.get(&game_crc).cloned().unwrap_or_default()
            };
            device::run_game(
                &mut device,
                rom_contents,
                ui::gui::GameSettings {
                    fullscreen: args.fullscreen,
                    overclock,
                    disable_expansion_pak,
                    cheats: game_cheats,
                },
            );
        });
        runtime.block_on(handle).unwrap()
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
        runtime.block_on(async {
            gui::app_window();
        });
    }
    Ok(())
}
