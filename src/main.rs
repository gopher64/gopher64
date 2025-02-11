#![deny(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
    result = std::fs::create_dir_all(dirs.cache_dir.clone());
    if result.is_err() {
        panic!("could not create cache dir: {}", result.err().unwrap())
    }
    result = std::fs::create_dir_all(dirs.data_dir.join("saves"));
    if result.is_err() {
        panic!("could not create save dir: {}", result.err().unwrap())
    }
    result = std::fs::create_dir_all(dirs.data_dir.join("states"));
    if result.is_err() {
        panic!("could not create state dir: {}", result.err().unwrap())
    }
    let running_file = dirs.cache_dir.join("game_running");
    if running_file.exists() {
        result = std::fs::remove_file(running_file);
        if result.is_err() {
            panic!("could not remove running file: {}", result.err().unwrap())
        }
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
            let controllers = gui::get_controller_names(&ui);
            for (i, controller) in controllers.iter().enumerate() {
                println!("Controller {}: {}", i, controller);
            }
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
            return;
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
    } else if args.game.is_some() {
        let file_path = std::path::Path::new(args.game.as_ref().unwrap());
        let rom_contents = device::get_rom_contents(file_path);
        if rom_contents.is_empty() {
            println!("Could not read rom file");
            return;
        }
        let handle = std::thread::Builder::new()
            .name("n64".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut device = device::Device::new();
                device::run_game(rom_contents, &mut device, args.fullscreen);
            })
            .unwrap();

        handle.join().unwrap();
    } else {
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([640.0, 480.0])
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!("../data/gopher64.png"))
                        .unwrap(),
                ),
            ..Default::default()
        };

        let controllers_paths;
        let controller_names;
        {
            let game_ui = ui::Ui::new();
            controllers_paths = gui::get_controller_paths(&game_ui);
            controller_names = gui::get_controller_names(&game_ui);
        }
        eframe::run_native(
            "gopher64",
            options,
            Box::new(|cc| {
                Ok(Box::new(ui::gui::GopherEguiApp::new(
                    cc,
                    controllers_paths,
                    controller_names,
                )))
            }),
        )
        .unwrap();
    }
}
