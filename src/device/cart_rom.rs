use crate::device;
use sha2::{Digest, Sha256};

pub enum CicType {
    CicNus6101,
    CicNus6102,
    CicNus6103,
    CicNus6105,
    CicNus6106,
    CicNus5167,
}

const CART_MASK: usize = 0xFFFFFFF;
pub struct Cart {
    pub rom: Vec<u8>,
    pub is_viewer_buffer: [u8; 0xFFFF],
    pub pal: bool,
    pub latch: u32,
    pub cic_type: CicType,
    pub cic_seed: u8,
    pub rdram_size_offset: usize,
    pub rtc: device::cart::AfRtc,
}

pub fn read_mem_fast(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & CART_MASK;
    u32::from_be_bytes(
        device.cart.rom[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    )
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
        device.cart.latch
    } else {
        let masked_address = address as usize & CART_MASK;
        u32::from_be_bytes(
            device.cart.rom[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        )
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

pub fn dma_read(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    cart_addr &= CART_MASK as u32;

    for i in 0..length {
        if cart_addr + i < device.cart.rom.len() as u32 {
            device.cart.rom[(cart_addr + i) as usize] =
                device.rdram.mem[(dram_addr + i) as usize ^ device.byte_swap];

            device.ui.saves.romsave.0.insert(
                (cart_addr + i).to_string(),
                serde_json::to_value(device.rdram.mem[(dram_addr + i) as usize ^ device.byte_swap])
                    .unwrap(),
            );
        }
    }

    device.ui.saves.romsave.1 = true;

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
    while i < dram_addr + length && j < device.cart.rom.len() as u32 {
        device.rdram.mem[i as usize ^ device.byte_swap] = device.cart.rom[j as usize];
        i += 1;
        j += 1;
    }
    while i < dram_addr + length {
        // DMAs that extend past the end of the ROM return 0's for the portion that extends past the ROM length
        device.rdram.mem[i as usize ^ device.byte_swap] = 0;
        i += 1;
    }
    device::pi::calculate_cycles(device, 1, length)
}

pub fn init(device: &mut device::Device, rom_file: Vec<u8>) {
    device.cart.rom = rom_file;
    set_system_region(device, device.cart.rom[0x3E]);
    set_cic(device);

    device.ui.game_hash = calculate_hash(&device.cart.rom);

    device.ui.game_id = String::from_utf8(device.cart.rom[0x3B..0x3E].to_vec()).unwrap();
    if device.ui.game_id.contains('\0') {
        device.ui.game_id = String::from("UNK");
    }
}

pub fn load_rom_save(device: &mut device::Device) {
    if device.ui.saves.romsave.0.is_empty() {
        return;
    }
    for (key, value) in device.ui.saves.romsave.0.iter() {
        device.cart.rom[key.parse::<usize>().unwrap()] =
            serde_json::from_value(value.clone()).unwrap()
    }
}

fn set_system_region(device: &mut device::Device, country: u8) {
    let pal_codes: [u8; 8] = [b'D', b'F', b'I', b'P', b'S', b'U', b'X', b'Y'];
    for i in pal_codes {
        if country == i {
            device.cart.pal = true
        }
    }
}

fn set_cic(device: &mut device::Device) {
    let hash = calculate_hash(&device.cart.rom[0x40..0x1000]);
    match hash.as_str() {
        "B99F06C4802C2377E31E388435955EF3E99C618A6D55D24699D828EB1075F1EB" => {
            device.cart.cic_type = CicType::CicNus6101;
            device.cart.cic_seed = 0x3F;
            device.cart.rdram_size_offset = 0x318;
        }
        "61E88238552C356C23D19409FE5570EE6910419586BC6FC740F638F761ADC46E" => {
            device.cart.cic_type = CicType::CicNus6102;
            device.cart.cic_seed = 0x3F;
            device.cart.rdram_size_offset = 0x318;
        }
        "BF3620D30817007091EBE9BDDD1B88C23B8A0052170B3309CDE5B6B4238E45E7" => {
            device.cart.cic_type = CicType::CicNus6103;
            device.cart.cic_seed = 0x78;
            device.cart.rdram_size_offset = 0x318;
        }
        "04B7BC6717A9F0EB724CF927E74AD3876C381CBB280D841736FC5E55580B756B" => {
            device.cart.cic_type = CicType::CicNus6105;
            device.cart.cic_seed = 0x91;
            device.cart.rdram_size_offset = 0x3F0;
        }
        "36ADC40148AF56F0D78CD505EB6A90117D1FD6F11C6309E52ED36BC4C6BA340E" => {
            device.cart.cic_type = CicType::CicNus6106;
            device.cart.cic_seed = 0x85;
            device.cart.rdram_size_offset = 0x318;
        }
        "53C0088FB777870D0AF32F0251E964030E2E8B72E830C26042FD191169508C05" => {
            device.cart.cic_type = CicType::CicNus5167;
            device.cart.cic_seed = 0xdd;
            device.cart.rdram_size_offset = 0x318;
        }
        _ => {
            device.cart.cic_type = CicType::CicNus6102;
            device.cart.cic_seed = 0x3F;
            device.cart.rdram_size_offset = 0x318;
            println!("unknown IPL3 {}", hash)
        }
    }
}

fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:X}", hasher.finalize())
}
