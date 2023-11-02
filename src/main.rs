#![feature(round_ties_even)]
#![feature(string_remove_matches)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod device;
mod ui;
use clap::Parser;

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
    assign_controller: Option<u32>,
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

fn main() {
    let cache_dir = dirs::cache_dir().unwrap().join("gopher64");

    let _ = std::fs::create_dir_all(cache_dir.clone());
    let _ = std::fs::remove_file(cache_dir.clone().join("game_running"));

    let args = Args::parse();
    let args_as_strings: Vec<String> = std::env::args().collect();
    let args_count = args_as_strings.len();
    if args_count > 1 {
        let mut device = device::Device::new();

        if args.clear_input_bindings {
            ui::input::clear_bindings(&mut device.ui);
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
            ui::input::list_controllers(&device.ui);
            return;
        }
        if args.assign_controller.is_some() {
            if args.port.is_none() {
                println!("Must specify port number");
                return;
            }
            ui::input::assign_controller(
                &mut device.ui,
                args.assign_controller.unwrap(),
                args.port.unwrap(),
            );
            return;
        }
        if args.bind_input_profile.is_some() {
            if args.port.is_none() {
                println!("Must specify port number");
                return;
            }
            ui::input::bind_input_profile(
                &mut device.ui,
                args.bind_input_profile.unwrap(),
                args.port.unwrap(),
            );
            return;
        }
        if args.configure_input_profile.is_some() {
            ui::input::configure_input_profile(
                &mut device.ui,
                args.configure_input_profile.unwrap(),
            );
            return;
        }

        if args.game.is_some() {
            let file_path = std::path::Path::new(args.game.as_ref().unwrap());
            device::run_game(file_path, &mut device, args.fullscreen);
        }
    } else {
        let options = eframe::NativeOptions {
            initial_window_size: Some(eframe::egui::vec2(640.0, 480.0)),
            ..Default::default()
        };
        eframe::run_native(
            "gopher64",
            options,
            Box::new(|_cc| Box::new(ui::gui::GopherEguiApp::new())),
        )
        .unwrap();
    }
}
