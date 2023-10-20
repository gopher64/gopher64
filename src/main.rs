#![feature(round_ties_even)]
#![feature(string_remove_matches)]
use std::fs;
use std::io::Read;
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
    assign_controller: Option<usize>,
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

fn swap_rom(contents: Vec<u8>) -> Vec<u8> {
    let test = u32::from_be_bytes(contents[0..4].try_into().unwrap());
    if test == 0x80371240 {
        // z64
        return contents;
    } else if test == 0x37804012 {
        // v64
        let mut data: Vec<u8> = vec![0; contents.len()];
        for i in (0..contents.len()).step_by(2) {
            let temp = u16::from_ne_bytes(contents[i..i + 2].try_into().unwrap());
            data[i..i + 2].copy_from_slice(&temp.to_be_bytes());
        }
        return data;
    } else if test == 0x40123780 {
        // n64
        let mut data: Vec<u8> = vec![0; contents.len()];
        for i in (0..contents.len()).step_by(4) {
            let temp = u32::from_ne_bytes(contents[i..i + 4].try_into().unwrap());
            data[i..i + 4].copy_from_slice(&temp.to_be_bytes());
        }
        return data;
    } else {
        panic!("unknown rom format")
    }
}

fn get_rom_contents(file_path: &std::path::Path) -> Vec<u8> {
    let mut contents = vec![];
    if file_path.extension().unwrap().to_ascii_lowercase() == "zip" {
        let zip_file = fs::File::open(file_path).unwrap();
        let mut archive = zip::ZipArchive::new(zip_file).unwrap();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let extension = file
                .enclosed_name()
                .unwrap()
                .extension()
                .unwrap()
                .to_ascii_lowercase();
            if extension == "z64" || extension == "n64" || extension == "v64" {
                file.read_to_end(&mut contents)
                    .expect("could not read zip file");
                break;
            }
        }
    } else if file_path.extension().unwrap().to_ascii_lowercase() == "7z" {
        let mut archive =
            sevenz_rust::SevenZReader::open(file_path, sevenz_rust::Password::empty()).unwrap();

        let mut found = false;
        archive
            .for_each_entries(
                &mut |entry: &sevenz_rust::SevenZArchiveEntry, reader: &mut dyn std::io::Read| {
                    let name = entry.name().to_ascii_lowercase();
                    if !found
                        && (name.ends_with("z64") || name.ends_with("n64") || name.ends_with("v64"))
                    {
                        reader
                            .read_to_end(&mut contents)
                            .expect("could not read zip file");
                        found = true;
                    } else {
                        //skip other files
                        std::io::copy(reader, &mut std::io::sink())?;
                    }
                    Ok(true)
                },
            )
            .expect("ok");
    } else {
        contents = fs::read(file_path).expect("Should have been able to read the file");
    }

    return swap_rom(contents);
}

fn main() {
    let args = Args::parse();
    let mut device = device::Device::new();

    if args.clear_input_bindings {
        ui::input::clear_bindings(&mut device.ui);
        return;
    }
    if args.port.is_some() {
        let port = args.port.unwrap();
        if port < 1 || port > 4 {
            println!("Port must be betwen 1 and 4");
            return;
        }
    }
    if args.list_controllers {
        ui::input::list_controllers(&mut device.ui);
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
        ui::input::configure_input_profile(&mut device.ui, args.configure_input_profile.unwrap());
        return;
    }
    let file_path = std::path::Path::new(args.game.as_ref().unwrap());

    let rom_contents = get_rom_contents(file_path);

    device::cart_rom::init(&mut device, rom_contents); // cart needs to come before rdram

    // rdram pointer is shared with parallel-rdp
    let (rdram_ptr, rdram_size) = device::rdram::init(&mut device);

    ui::audio::init(&mut device.ui, 33600);
    ui::video::init(&mut device.ui, rdram_ptr, rdram_size, args.fullscreen);

    device::mi::init(&mut device);
    device::pif::init(&mut device);
    device::memory::init(&mut device);
    device::rsp_interface::init(&mut device);
    device::rdp::init(&mut device);
    device::vi::init(&mut device);
    device::cpu::init(&mut device);

    ui::storage::init(&mut device.ui);
    ui::storage::load_saves(&mut device.ui);
    device::cart_rom::load_rom_save(&mut device);

    device::cpu::run(&mut device);
}
