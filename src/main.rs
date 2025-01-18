#![deny(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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

#[tokio::main]
async fn main() {
    let exe_path = std::env::current_exe().unwrap();
    let portable_dir = exe_path.parent();
    let portable = portable_dir.unwrap().join("portable.txt").exists();
    let config_dir;
    let cache_dir;
    let data_dir;
    if portable {
        config_dir = portable_dir.unwrap().join("portable_data").join("config");
        cache_dir = portable_dir.unwrap().join("portable_data").join("cache");
        data_dir = portable_dir.unwrap().join("portable_data").join("data");
    } else {
        config_dir = dirs::config_dir().unwrap().join("gopher64");
        cache_dir = dirs::cache_dir().unwrap().join("gopher64");
        data_dir = dirs::data_dir().unwrap().join("gopher64");
    };

    let mut result = std::fs::create_dir_all(config_dir.clone());
    if result.is_err() {
        panic!("could not create config dir: {}", result.err().unwrap())
    }
    result = std::fs::create_dir_all(cache_dir.clone());
    if result.is_err() {
        panic!("could not create cache dir: {}", result.err().unwrap())
    }
    result = std::fs::create_dir_all(data_dir.clone().join("saves"));
    if result.is_err() {
        panic!("could not create data dir: {}", result.err().unwrap())
    }
    let running_file = cache_dir.join("game_running");
    if running_file.exists() {
        result = std::fs::remove_file(running_file);
        if result.is_err() {
            panic!("could not remove running file: {}", result.err().unwrap())
        }
    }

    let args = Args::parse();
    let args_as_strings: Vec<String> = std::env::args().collect();
    let args_count = args_as_strings.len();
    if args_count > 1 {
        let mut device = device::Device::new(config_dir);

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
                args.use_dinput,
            );
            return;
        }

        if args.game.is_some() {
            let file_path = std::path::Path::new(args.game.as_ref().unwrap());
            device::run_game(file_path, data_dir, &mut device, args.fullscreen);
        }
    } else {
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
            ..Default::default()
        };
        eframe::run_native(
            "gopher64",
            options,
            Box::new(|cc| {
                Ok(Box::new(ui::gui::GopherEguiApp::new(
                    cc, config_dir, cache_dir, data_dir,
                )))
            }),
        )
        .unwrap();
    }
}
