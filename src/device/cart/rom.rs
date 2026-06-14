use crate::device;
use crate::ui;
use sha2::{Digest, Sha256};

pub const CART_MASK: usize = 0xFFFFFFF;

fn read_cart_word(device: &device::Device, address: usize) -> u32 {
    let romsave = &device.ui.storage.saves.romsave.data;
    let rom = &device.cart.rom;
    u32::from_be_bytes(std::array::from_fn(|i| {
        romsave
            .get(&((address + i) as u32))
            .copied()
            .unwrap_or_else(|| *rom.get(address + i).unwrap_or(&0))
    }))
}

pub fn read_mem_fast(
    device: &device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & CART_MASK;
    read_cart_word(device, masked_address)
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let cycles = device::pi::calculate_cycles(device, 1, 4);
    device::cop0::add_cycles(device, cycles);

    // well known cart ROM oddity, if a read is performed while PI_STATUS_IO_BUSY is set, the latched value is returned rather than the data at the specified address
    if device.pi.regs[device::pi::PI_STATUS_REG] & device::pi::PI_STATUS_IO_BUSY != 0 {
        device.cart.latch
    } else {
        let masked_address = address as usize & CART_MASK;
        read_cart_word(device, masked_address)
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if device.cart.sc64.cfg[device::cart::sc64::SC64_ROM_WRITE_ENABLE as usize] != 0 {
        let masked_address = address as usize & CART_MASK;
        let mut data = read_cart_word(device, masked_address);
        device::memory::masked_write_32(&mut data, value, mask);
        for (i, item) in data.to_be_bytes().iter().enumerate() {
            device
                .ui
                .storage
                .saves
                .romsave
                .data
                .insert((masked_address + i) as u32, *item);
        }
        ui::storage::schedule_save(device, ui::storage::SaveTypes::Romsave);
    }

    device.cart.latch = value & mask;

    device.pi.regs[device::pi::PI_STATUS_REG] |= device::pi::PI_STATUS_IO_BUSY;

    let cycles = device::pi::calculate_cycles(device, 1, 4);
    device::events::create_event(device, device::events::EVENT_TYPE_PI, cycles);
}

pub fn dma_read(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    if device.cart.sc64.cfg[device::cart::sc64::SC64_ROM_WRITE_ENABLE as usize] != 0 {
        dram_addr &= device::rdram::RDRAM_MASK as u32;
        cart_addr &= CART_MASK as u32;

        for i in 0..length {
            device.ui.storage.saves.romsave.data.insert(
                cart_addr + i,
                *device
                    .rdram
                    .mem
                    .get((dram_addr + i) as usize ^ device.byte_swap)
                    .unwrap_or(&0),
            );
        }
        ui::storage::schedule_save(device, ui::storage::SaveTypes::Romsave);
    }

    device::pi::calculate_cycles(device, 1, length)
}

// cart is big endian, rdram is native endian
pub fn dma_write(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    cart_addr &= CART_MASK as u32;
    let mut i = dram_addr;
    let mut j = cart_addr;
    while i < dram_addr + length {
        *device
            .rdram
            .mem
            .get_mut(i as usize ^ device.byte_swap)
            .unwrap_or(&mut 0) = if let Some(value) = device.ui.storage.saves.romsave.data.get(&j) {
            *value
        } else {
            *device.cart.rom.get(j as usize).unwrap_or(&0)
        };
        i += 1;
        j += 1;
    }

    device::pi::calculate_cycles(device, 1, length)
}

pub fn init(device: &mut device::Device, rom_file: &[u8]) {
    device.cart.sc64.cfg[device::cart::sc64::SC64_BOOTLOADER_SWITCH as usize] = 1;

    device.cart.rom = rom_file.to_vec();
    device.cart.pal = is_system_pal(&device.cart.rom);
    set_cic(device);

    device.ui.game_hash = calculate_hash(&device.cart.rom);

    device.ui.game_id = String::from_utf8(device.cart.rom[0x3B..0x3E].to_vec()).unwrap();
    if device.ui.game_id.contains('\0') {
        device.ui.game_id = String::from("UNK");
    }
}

pub fn is_system_pal(rom_contents: &[u8]) -> bool {
    let pal_codes: [u8; 8] = *b"DFIPSUXY";
    for i in pal_codes {
        if rom_contents[0x3E] == i {
            return true;
        }
    }
    false
}

fn set_cic(device: &mut device::Device) {
    let hash = calculate_hash(&device.cart.rom[0x40..0x1000]);
    match hash.as_str() {
        "B99F06C4802C2377E31E388435955EF3E99C618A6D55D24699D828EB1075F1EB" => {
            device.cart.cic_seed = 0x3F; // CicNus6101
        }
        "61E88238552C356C23D19409FE5570EE6910419586BC6FC740F638F761ADC46E" => {
            device.cart.cic_seed = 0x3F; // CicNus6102
        }
        "BF3620D30817007091EBE9BDDD1B88C23B8A0052170B3309CDE5B6B4238E45E7" => {
            device.cart.cic_seed = 0x78; // CicNus6103
        }
        "04B7BC6717A9F0EB724CF927E74AD3876C381CBB280D841736FC5E55580B756B" => {
            device.cart.cic_seed = 0x91; // CicNus6105
        }
        "36ADC40148AF56F0D78CD505EB6A90117D1FD6F11C6309E52ED36BC4C6BA340E" => {
            device.cart.cic_seed = 0x85; // CicNus6106
        }
        "53C0088FB777870D0AF32F0251E964030E2E8B72E830C26042FD191169508C05" => {
            device.cart.cic_seed = 0xdd; // CicNus5167
            device.cart.sc64.cfg[device::cart::sc64::SC64_ROM_WRITE_ENABLE as usize] = 1;
        }
        _ => {
            device.cart.cic_seed = 0x3F; // CicNus6102
            //println!("unknown IPL3 {}", hash)
        }
    }
}

pub fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect()
}
