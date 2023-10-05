#![feature(round_ties_even)]
use std::env;
use std::fs;
mod device;
mod ui;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    let contents = fs::read(file_path).expect("Should have been able to read the file");

    let mut device = device::Device::new();

    device::cart_rom::init(&mut device, contents); // cart needs to come before rdram

    // rdram pointer is shared with parallel-rdp
    let (rdram_ptr, rdram_size) = device::rdram::init(&mut device);

    ui::audio::init(&mut device.ui, 33600);
    ui::video::init(&mut device.ui, rdram_ptr, rdram_size);

    device::mi::init(&mut device);
    device::pif::init(&mut device, false);
    device::memory::init(&mut device);
    device::rsp_interface::init(&mut device);
    device::rdp::init(&mut device);
    device::vi::init(&mut device);
    device::cpu::init(&mut device);

    ui::storage::init(&mut device.ui);
    ui::storage::load_saves(&mut device.ui);

    device::cpu::run(&mut device);
}
