#![feature(round_ties_even)]
use std::env;
use std::fs;
use std::io::Read;
mod device;
mod ui;

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

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_path = std::path::Path::new(&args[1]);
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
    } else {
        contents = fs::read(file_path).expect("Should have been able to read the file");
    }

    contents = swap_rom(contents);

    let mut device = device::Device::new();

    device::cart_rom::init(&mut device, contents); // cart needs to come before rdram

    // rdram pointer is shared with parallel-rdp
    let (rdram_ptr, rdram_size) = device::rdram::init(&mut device);

    ui::audio::init(&mut device.ui, 33600);
    ui::video::init(&mut device.ui, rdram_ptr, rdram_size);

    device::mi::init(&mut device);
    device::pif::init(&mut device);
    device::memory::init(&mut device);
    device::rsp_interface::init(&mut device);
    device::rdp::init(&mut device);
    device::vi::init(&mut device);
    device::cpu::init(&mut device);

    ui::storage::init(&mut device.ui);
    ui::storage::load_saves(&mut device.ui);

    device::cpu::run(&mut device);
}
