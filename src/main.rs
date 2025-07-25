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
        help = "Create a new input profile (keyboard/gamepad mappings)."
    )]
    configure_input_profile: Option<String>,
    #[arg(
        short,
        long,
        help = "Use DirectInput when configuring a new input profile."
    )]
    use_dinput: bool,
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

#[tokio::main]
async fn main() {
    let dirs = ui::get_dirs();

    let mut result = std::fs::create_dir_all(dirs.config_dir);
    if result.is_err() {
        panic!("could not create config dir: {}", result.err().unwrap())
    }
    result = std::fs::create_dir_all(dirs.data_dir.join("saves"));
    if result.is_err() {
        panic!("could not create save dir: {}", result.err().unwrap())
    }
    result = std::fs::create_dir_all(dirs.data_dir.join("states"));
    if result.is_err() {
        panic!("could not create state dir: {}", result.err().unwrap())
    }

    let args = Args::parse();
    let args_as_strings: Vec<String> = std::env::args().collect();
    let args_count = args_as_strings.len();
    if args_count > 1 && args.game.is_none() {
        let mut ui = ui::Ui::new();

        if args.clear_input_bindings {
            ui::input::clear_bindings(&mut ui);
            return;
        }
        if args.port.is_some() {
            let port = args.port.unwrap();
            if !(1..=4).contains(&port) {
                println!("Port must be betwen 1 and 4");
                return;
            }
        }
        if args.list_controllers {
            let controllers = ui::input::get_controller_names(&ui);
            for (i, controller) in controllers.iter().enumerate() {
                println!("Controller {i}: {controller}");
            }
            return;
        }
        if args.configure_input_profile.is_some() {
            ui::input::configure_input_profile(
                &mut ui,
                args.configure_input_profile.unwrap(),
                args.use_dinput,
            );
            return;
        }
        if args.assign_controller.is_some() {
            if args.port.is_none() {
                println!("Must specify port number");
                return;
            }
            ui::input::assign_controller(
                &mut ui,
                args.assign_controller.unwrap(),
                args.port.unwrap(),
            );
        }
        if args.bind_input_profile.is_some() {
            if args.port.is_none() {
                println!("Must specify port number");
                return;
            }
            ui::input::bind_input_profile(
                &mut ui,
                args.bind_input_profile.unwrap(),
                args.port.unwrap(),
            );
        }
    } else if args.game.is_some() {
        let file_path = std::path::Path::new(args.game.as_ref().unwrap());
        if let Some(rom_contents) = device::get_rom_contents(file_path) {
            let handle = std::thread::Builder::new()
                .name("n64".to_string())
                .stack_size(env!("N64_STACK_SIZE").parse().unwrap())
                .spawn(move || {
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
                })
                .unwrap();

            handle.join().unwrap();
        } else {
            println!("Could not read rom file");
            return;
        }
    } else {
        gui::app_window();
    }
}
