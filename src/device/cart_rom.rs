use crate::device;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub enum CicType {
    CicNus6101,
    CicNus6102,
    CicNus6103,
    CicNus6105,
    CicNus6106,
}

pub const CART_MASK: usize = 0xFFFFFFF;
pub struct Cart {
    pub rom: Vec<u8>,
    pub is_viewer_buffer: [u8; 0xFFFF],
    pub pal: bool,
    pub latch: u32,
    pub cic_type: CicType,
    pub cic_seed: u8,
    pub rdram_size_offset: usize,
}

pub fn read_mem_fast(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & CART_MASK;
    return u32::from_be_bytes(
        device.cart.rom[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    );
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let cycles = device::pi::calculate_cycles(device, 1, 4);
    device::cop0::add_cycles(device, cycles);

    // well known cart ROM oddity, if a read is perfomed while PI_STATUS_IO_BUSY is set, the latched value is returned rather than the data at the specified address
    if device.pi.regs[device::pi::PI_STATUS_REG as usize] & device::pi::PI_STATUS_IO_BUSY != 0 {
        return device.cart.latch;
    } else {
        let masked_address = address as usize & CART_MASK;
        return u32::from_be_bytes(
            device.cart.rom[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        );
    }
}

pub fn write_mem(device: &mut device::Device, _address: u64, value: u32, mask: u32) {
    device.cart.latch = value & mask;

    device.pi.regs[device::pi::PI_STATUS_REG as usize] |= device::pi::PI_STATUS_IO_BUSY;

    let cycles = device::pi::calculate_cycles(device, 1, 4);
    device::events::create_event(
        device,
        device::events::EventType::PI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + cycles,
        device::pi::dma_event,
    );
}

pub fn dma_read(device: &mut device::Device, _cart_addr: u32, _dram_addr: u32, length: u32) -> u64 {
    return device::pi::calculate_cycles(device, 1, length);
}

// cart is big endian, rdram is native endian
pub fn dma_write(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    let mut i = dram_addr & device::rdram::RDRAM_MASK as u32;
    let mut j = cart_addr & CART_MASK as u32;
    while i < (dram_addr & device::rdram::RDRAM_MASK as u32) + length
        && j < device.cart.rom.len() as u32
    {
        device.rdram.mem[i as usize ^ device.byte_swap] = device.cart.rom[j as usize];
        i += 1;
        j += 1;
    }
    while i < (dram_addr & device::rdram::RDRAM_MASK as u32) + length {
        // DMAs that extend past the end of the ROM return 0's for the portion that extends past the ROM length
        device.rdram.mem[i as usize ^ device.byte_swap] = 0;
        i += 1;
    }
    return device::pi::calculate_cycles(device, 1, length);
}

pub fn init(device: &mut device::Device, rom_file: Vec<u8>) {
    device.cart.rom = rom_file;
    set_system_region(device, device.cart.rom[0x3E]);
    set_cic(device)
}

pub fn set_system_region(device: &mut device::Device, country: u8) {
    let pal_codes: [u8; 8] = [b'D', b'F', b'I', b'P', b'S', b'U', b'X', b'Y'];
    for i in pal_codes {
        if country == i {
            device.cart.pal = true
        }
    }
}

pub fn set_cic(device: &mut device::Device) {
    let hash = calculate_hash(&device.cart.rom[0x40..0x1000]);
    match hash {
        0x83a9a60ad75b3ed9 => {
            device.cart.cic_type = CicType::CicNus6101;
            device.cart.cic_seed = 0x3F;
            device.cart.rdram_size_offset = 0x318;
        }
        0x3c1e2f5171ec2b8 => {
            device.cart.cic_type = CicType::CicNus6102;
            device.cart.cic_seed = 0x3F;
            device.cart.rdram_size_offset = 0x318;
        }
        0xdbdb0cb696006257 => {
            device.cart.cic_type = CicType::CicNus6103;
            device.cart.cic_seed = 0x78;
            device.cart.rdram_size_offset = 0x318;
        }
        0x3d51ac3a48960357 => {
            device.cart.cic_type = CicType::CicNus6105;
            device.cart.cic_seed = 0x91;
            device.cart.rdram_size_offset = 0x3F0;
        }
        0x90135a02ea97ba9f => {
            device.cart.cic_type = CicType::CicNus6106;
            device.cart.cic_seed = 0x85;
            device.cart.rdram_size_offset = 0x318;
        }
        _ => {
            device.cart.cic_type = CicType::CicNus6102;
            device.cart.cic_seed = 0x3F;
            device.cart.rdram_size_offset = 0x318;
            println!("unknown IPL3 {:#01x}", hash)
        }
    }
}

fn calculate_hash<T: Hash>(t: &[T]) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
