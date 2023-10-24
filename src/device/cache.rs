use crate::device;

#[derive(Copy, Clone)]
pub struct ICache {
    pub valid: bool,
    pub tag: u32,
    pub index: u16,
    pub words: [u32; 8],
    pub instruction: [fn(&mut device::Device, u32); 8],
}

#[derive(Copy, Clone)]
pub struct DCache {
    pub valid: bool,
    pub dirty: bool,
    pub tag: u32,
    pub index: u16,
    pub words: [u32; 4],
}

pub fn icache_hit(device: &mut device::Device, line_index: usize, phys_address: u64) -> bool {
    device.memory.icache[line_index].valid
        && (device.memory.icache[line_index].tag & 0x1ffffffc) == (phys_address & !0xFFF) as u32
}

pub fn icache_writeback(device: &mut device::Device, line_index: usize) {
    device::cop0::add_cycles(device, device::rdram::rdram_calculate_cycles(32));

    let cache_address = ((device.memory.icache[line_index].tag
        | (device.memory.icache[line_index].index) as u32)
        & 0x1ffffffc) as u64;
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address,
        device.memory.icache[line_index].words[0],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x4,
        device.memory.icache[line_index].words[1],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x8,
        device.memory.icache[line_index].words[2],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0xC,
        device.memory.icache[line_index].words[3],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x10,
        device.memory.icache[line_index].words[4],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x14,
        device.memory.icache[line_index].words[5],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x18,
        device.memory.icache[line_index].words[6],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x1C,
        device.memory.icache[line_index].words[7],
        0xFFFFFFFF,
    );
}

pub fn icache_fill(device: &mut device::Device, line_index: usize, phys_address: u64) {
    device::cop0::add_cycles(device, 8);

    device.memory.icache[line_index].valid = true;
    device.memory.icache[line_index].tag = (phys_address & !0xFFF) as u32;
    let cache_address = ((device.memory.icache[line_index].tag
        | (device.memory.icache[line_index].index) as u32)
        & 0x1ffffffc) as u64;
    device.memory.icache[line_index].words[0] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address,
        device::memory::AccessSize::Icache,
    );
    device.memory.icache[line_index].words[1] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x4,
        device::memory::AccessSize::Icache,
    );
    device.memory.icache[line_index].words[2] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x8,
        device::memory::AccessSize::Icache,
    );
    device.memory.icache[line_index].words[3] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0xC,
        device::memory::AccessSize::Icache,
    );
    device.memory.icache[line_index].words[4] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x10,
        device::memory::AccessSize::Icache,
    );
    device.memory.icache[line_index].words[5] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x14,
        device::memory::AccessSize::Icache,
    );
    device.memory.icache[line_index].words[6] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x18,
        device::memory::AccessSize::Icache,
    );
    device.memory.icache[line_index].words[7] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x1C,
        device::memory::AccessSize::Icache,
    );

    device.memory.icache[line_index].instruction[0] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[0]);
    device.memory.icache[line_index].instruction[1] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[1]);
    device.memory.icache[line_index].instruction[2] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[2]);
    device.memory.icache[line_index].instruction[3] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[3]);
    device.memory.icache[line_index].instruction[4] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[4]);
    device.memory.icache[line_index].instruction[5] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[5]);
    device.memory.icache[line_index].instruction[6] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[6]);
    device.memory.icache[line_index].instruction[7] =
        device::cpu::decode_opcode(device, device.memory.icache[line_index].words[7]);
}

pub fn icache_fetch(device: &mut device::Device, phys_address: u64) {
    let line_index = ((phys_address >> 5) & 0x1FF) as usize;
    if !icache_hit(device, line_index, phys_address) {
        icache_fill(device, line_index, phys_address)
    }
    let item = ((phys_address >> 2) & 7) as usize;
    device.memory.icache[line_index].instruction[item](
        device,
        device.memory.icache[line_index].words[item],
    );
}

pub fn dcache_hit(device: &mut device::Device, line_index: usize, phys_address: u64) -> bool {
    device.memory.dcache[line_index].valid
        && (device.memory.dcache[line_index].tag & 0x1ffffffc) == (phys_address & !0xFFF) as u32
}

pub fn dcache_writeback(device: &mut device::Device, line_index: usize) {
    device::cop0::add_cycles(device, device::rdram::rdram_calculate_cycles(16));

    device.memory.dcache[line_index].dirty = false;

    let cache_address = ((device.memory.dcache[line_index].tag
        | (device.memory.dcache[line_index].index) as u32)
        & 0x1ffffffc) as u64;
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address,
        device.memory.dcache[line_index].words[0],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x4,
        device.memory.dcache[line_index].words[1],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0x8,
        device.memory.dcache[line_index].words[2],
        0xFFFFFFFF,
    );
    device.memory.memory_map_write[(cache_address >> 16) as usize](
        device,
        cache_address | 0xC,
        device.memory.dcache[line_index].words[3],
        0xFFFFFFFF,
    );
}

pub fn dcache_fill(device: &mut device::Device, line_index: usize, phys_address: u64) {
    device::cop0::add_cycles(device, 7);

    device.memory.dcache[line_index].valid = true;
    device.memory.dcache[line_index].dirty = false;

    device.memory.dcache[line_index].tag = (phys_address & !0xFFF) as u32;
    let cache_address = ((device.memory.dcache[line_index].tag
        | (device.memory.dcache[line_index].index) as u32)
        & 0x1ffffffc) as u64;
    device.memory.dcache[line_index].words[0] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address,
        device::memory::AccessSize::Dcache,
    );
    device.memory.dcache[line_index].words[1] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x4,
        device::memory::AccessSize::Dcache,
    );
    device.memory.dcache[line_index].words[2] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0x8,
        device::memory::AccessSize::Dcache,
    );
    device.memory.dcache[line_index].words[3] = device.memory.memory_map_read
        [(cache_address >> 16) as usize](
        device,
        cache_address | 0xC,
        device::memory::AccessSize::Dcache,
    );
}

pub fn dcache_read(device: &mut device::Device, phys_address: u64) -> u32 {
    let line_index = ((phys_address >> 4) & 0x1FF) as usize;
    if !dcache_hit(device, line_index, phys_address) {
        if device.memory.dcache[line_index].valid && device.memory.dcache[line_index].dirty {
            dcache_writeback(device, line_index)
        }
        dcache_fill(device, line_index, phys_address)
    } else {
        device::cop0::add_cycles(device, 1)
    }
    device.memory.dcache[line_index].words[((phys_address >> 2) & 3) as usize]
}

pub fn dcache_write(device: &mut device::Device, phys_address: u64, value: u32, mask: u32) {
    let line_index = ((phys_address >> 4) & 0x1FF) as usize;
    if !dcache_hit(device, line_index, phys_address) {
        if device.memory.dcache[line_index].valid && device.memory.dcache[line_index].dirty {
            dcache_writeback(device, line_index)
        }
        dcache_fill(device, line_index, phys_address)
    } else {
        device::cop0::add_cycles(device, 1)
    }
    device::memory::masked_write_32(
        &mut device.memory.dcache[line_index].words[((phys_address >> 2) & 3) as usize],
        value,
        mask,
    );
    device.memory.dcache[line_index].dirty = true;
}

pub fn init(device: &mut device::Device) {
    for (pos, i) in device.memory.icache.iter_mut().enumerate() {
        i.index = (pos << 5) as u16 & 0xFE0
    }
    for (pos, i) in device.memory.dcache.iter_mut().enumerate() {
        i.index = (pos << 4) as u16 & 0xFF0
    }
}
