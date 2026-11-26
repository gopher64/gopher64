use crate::device;
use crate::ui;
use sha2::{Digest, Sha256};

pub const CART_MASK: usize = 0xFFFFFFF;
const CIC_SEED_5167: u8 = 0xdd;

/// Whether cart ROM writes are saved to disk. Real flashcarts (e.g. SummerCart64) only keep
/// ROM writes until power-off, so only CIC 5167 ROMs (which expect persistence) get a romsave.
pub fn romsave_persists(cic_seed: u8) -> bool {
    cic_seed == CIC_SEED_5167
}

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

fn apply_patches(device: &mut device::Device) {
    let (instructions, checksum): (&[(usize, u32)], u64) = match device.ui.game_hash.as_str() {
        // https://github.com/gopher64/gopher64/issues/1436
        // WCW Nitro (USA): call osWritebackDCacheAll instead of writing back only the task.
        "4ADE28775C54157FCA8763DE388442B5CA149111B13CD7F1D2E6429C90EAF065" => {
            (&[(0xd9f0, 0x0c01_7a60)], 0xd4c4_5a5a_3138_2c95)
        }
        // https://github.com/gopher64/gopher64/issues/1427
        // Wonder Project J2: update the shop only with zero graphics tasks outstanding.
        // Otherwise switching items can reuse memory still referenced by the previous task.
        "8BBDA06B90CA6CBB897286B05D6BBC3D87464AF6A14C3641EC84D2B25610A3AA" => {
            (&[(0x67a4, 0x2b21_0001)], 0xe43c_9767_d6f9_4f42) // Japanese retail
        }
        "CF0E08C8349291EE5561BA03FA82287A505ACECE1644588001937B0ABC732D31" => {
            (&[(0x67a4, 0x2b21_0001)], 0x4f1e_88f5_8c52_7523) // English fan translation
        }
        // https://github.com/gopher64/gopher64/issues/1446
        // John Romero's Daikatana (Japan): the fade flag clears before the splash callback
        // returns. Wait for the callback to unregister itself before loading
        // the title overlay over its code.
        "D733DEBF3A38C643127D58E84B086F44948BD630BF6449030DB8CEA1CA51ABF8" => (
            &[
                (0x38ce8, 0x3c01_801b), // lui at, 0x801b
                (0x38cf0, 0x8c23_f4a0), // lw v1, -0xb60(at), callback pointer tested by beqz
            ],
            0x8c7c_ff4d_b5f2_fa26,
        ),
        // Knife Edge - Nose Gunner: replacement words derived from MiSTer's XOR patches.
        // https://github.com/MiSTer-devel/N64_ROM_Database/blob/main/N64-database.txt
        "07578B1E624FCE4AD366F1FC5C446E4516592DA3A1F072730656AE0C5CDFD103" => (
            // Europe
            &[
                (0x19338, 0x1000_0006),
                (0x19354, 0x3c04_8012),
                (0x19358, 0x8c84_d408),
                (0x1935c, 0x3c05_8019),
                (0x19360, 0x34a5_7f63),
                (0x19364, 0x1480_0005),
                (0x19368, 0x2404_002a),
                (0x1936c, 0x2404_001a),
                (0x19370, 0xa0a4_0000),
                (0x19374, 0x1000_0003),
                (0x19378, 0x0000_0000),
                (0x1937c, 0xa0a4_0000),
                (0x19380, 0x0000_0000),
                (0x19384, 0x1000_ffbb),
                (0x19388, 0x0000_0000),
                (0x17b0c4, 0x11c0_0093),
            ],
            0x8fd6_0561_899a_0f19,
        ),
        "AB0DDAC8FCE7BA6DFD4F3CF83EB801A5BF64181515DE9FB4FE42F91E0BCB8143" => (
            // Japan
            &[
                (0x191c8, 0x1000_0006),
                (0x191e4, 0x3c04_8012),
                (0x191e8, 0x8c84_d288),
                (0x191ec, 0x3c05_8019),
                (0x191f0, 0x34a5_7dc3),
                (0x191f4, 0x1480_0005),
                (0x191f8, 0x2404_002a),
                (0x191fc, 0x2404_001a),
                (0x19200, 0xa0a4_0000),
                (0x19204, 0x1000_0003),
                (0x19208, 0x0000_0000),
                (0x1920c, 0xa0a4_0000),
                (0x19210, 0x0000_0000),
                (0x19214, 0x1000_ffbb),
                (0x19218, 0x0000_0000),
                (0x180614, 0x11c0_008b),
            ],
            0x564c_916e_086f_0fb0,
        ),
        "467FCB503F384D2479FB3A5FE22405089F56ECAB38A3ACD1A6AAF0D8B73372B5" => (
            // USA
            &[
                (0x19228, 0x1000_0006),
                (0x19244, 0x3c04_8012),
                (0x19248, 0x8c84_d298),
                (0x1924c, 0x3c05_8019),
                (0x19250, 0x34a5_7b0f),
                (0x19254, 0x1480_0005),
                (0x19258, 0x2404_002a),
                (0x1925c, 0x2404_001a),
                (0x19260, 0xa0a4_0000),
                (0x19264, 0x1000_0003),
                (0x19268, 0x0000_0000),
                (0x1926c, 0xa0a4_0000),
                (0x19270, 0x0000_0000),
                (0x19274, 0x1000_ffbb),
                (0x19278, 0x0000_0000),
                (0x17a380, 0x11c0_0083),
            ],
            0xb9d6_ca85_cc96_5fde,
        ),
        _ => return,
    };
    for &(offset, instruction) in instructions {
        device.cart.rom[offset..offset + 4].copy_from_slice(&instruction.to_be_bytes());
    }
    // Updated CIC-NUS-6102 header checksums for the patched ROM.
    device.cart.rom[0x10..0x18].copy_from_slice(&checksum.to_be_bytes());
    println!("Applied compatibility patch");
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

    apply_patches(device);
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
            device.cart.cic_seed = CIC_SEED_5167;
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
