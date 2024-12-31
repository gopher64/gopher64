use crate::device;

#[derive(Copy, Clone)]
pub struct TlbLut {
    pub address: u64,
    pub cached: bool,
}

#[derive(Copy, Clone)]
pub struct TlbEntry {
    pub mask: u64,
    pub vpn2: u64,
    pub region: u8,
    pub g: u8,
    pub asid: u8,
    pub pfn_even: u64,
    pub c_even: u8,
    pub d_even: u8,
    pub v_even: u8,
    pub pfn_odd: u64,
    pub c_odd: u8,
    pub d_odd: u8,
    pub v_odd: u8,

    pub start_even: u64,
    pub end_even: u64,
    pub phys_even: u64,
    pub start_odd: u64,
    pub end_odd: u64,
    pub phys_odd: u64,
}

pub fn read(device: &mut device::Device, index: u64) {
    if index > 31 {
        return;
    }
    device.cpu.cop0.regs[device::cop0::COP0_PAGEMASK_REG as usize] =
        device.cpu.cop0.tlb_entries[index as usize].mask << 13;

    device.cpu.cop0.regs[device::cop0::COP0_ENTRYHI_REG as usize] =
        ((device.cpu.cop0.tlb_entries[index as usize].region as u64) << 62)
            | (device.cpu.cop0.tlb_entries[index as usize].vpn2 << 13)
            | (device.cpu.cop0.tlb_entries[index as usize].asid) as u64;

    device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO0_REG as usize] =
        (device.cpu.cop0.tlb_entries[index as usize].pfn_even << 6)
            | (device.cpu.cop0.tlb_entries[index as usize].c_even << 3) as u64
            | (device.cpu.cop0.tlb_entries[index as usize].d_even << 2) as u64
            | (device.cpu.cop0.tlb_entries[index as usize].v_even << 1) as u64
            | (device.cpu.cop0.tlb_entries[index as usize].g) as u64;

    device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO1_REG as usize] =
        (device.cpu.cop0.tlb_entries[index as usize].pfn_odd << 6)
            | (device.cpu.cop0.tlb_entries[index as usize].c_odd << 3) as u64
            | (device.cpu.cop0.tlb_entries[index as usize].d_odd << 2) as u64
            | (device.cpu.cop0.tlb_entries[index as usize].v_odd << 1) as u64
            | (device.cpu.cop0.tlb_entries[index as usize].g) as u64
}

pub fn write(device: &mut device::Device, index: u64) {
    if index > 31 {
        return;
    }
    tlb_unmap(device, index);

    device.cpu.cop0.tlb_entries[index as usize].g = (device.cpu.cop0.regs
        [device::cop0::COP0_ENTRYLO0_REG as usize]
        & device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO1_REG as usize]
        & 1) as u8;

    device.cpu.cop0.tlb_entries[index as usize].pfn_even =
        (device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO0_REG as usize] >> 6) & 0xFFFFF;
    device.cpu.cop0.tlb_entries[index as usize].pfn_odd =
        (device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO1_REG as usize] >> 6) & 0xFFFFF;
    device.cpu.cop0.tlb_entries[index as usize].c_even =
        ((device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO0_REG as usize] >> 3) & 7) as u8;
    device.cpu.cop0.tlb_entries[index as usize].c_odd =
        ((device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO1_REG as usize] >> 3) & 7) as u8;
    device.cpu.cop0.tlb_entries[index as usize].d_even =
        ((device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO0_REG as usize] >> 2) & 1) as u8;
    device.cpu.cop0.tlb_entries[index as usize].d_odd =
        ((device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO1_REG as usize] >> 2) & 1) as u8;
    device.cpu.cop0.tlb_entries[index as usize].v_even =
        ((device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO0_REG as usize] >> 1) & 1) as u8;
    device.cpu.cop0.tlb_entries[index as usize].v_odd =
        ((device.cpu.cop0.regs[device::cop0::COP0_ENTRYLO1_REG as usize] >> 1) & 1) as u8;
    device.cpu.cop0.tlb_entries[index as usize].asid =
        device.cpu.cop0.regs[device::cop0::COP0_ENTRYHI_REG as usize] as u8;

    device.cpu.cop0.tlb_entries[index as usize].mask =
        (device.cpu.cop0.regs[device::cop0::COP0_PAGEMASK_REG as usize] >> 13) & 0xFFF;

    device.cpu.cop0.tlb_entries[index as usize].vpn2 =
        (device.cpu.cop0.regs[device::cop0::COP0_ENTRYHI_REG as usize] >> 13) & 0x7FFFFFF;

    device.cpu.cop0.tlb_entries[index as usize].region =
        (device.cpu.cop0.regs[device::cop0::COP0_ENTRYHI_REG as usize] >> 62) as u8;

    device.cpu.cop0.tlb_entries[index as usize].mask &= 0b101010101010;
    device.cpu.cop0.tlb_entries[index as usize].mask |=
        device.cpu.cop0.tlb_entries[index as usize].mask >> 1;

    device.cpu.cop0.tlb_entries[index as usize].vpn2 &=
        !device.cpu.cop0.tlb_entries[index as usize].mask;

    device.cpu.cop0.tlb_entries[index as usize].start_even =
        (device.cpu.cop0.tlb_entries[index as usize].vpn2 << 13) & 0xFFFFFFFF;
    device.cpu.cop0.tlb_entries[index as usize].end_even =
        device.cpu.cop0.tlb_entries[index as usize].start_even
            + (device.cpu.cop0.tlb_entries[index as usize].mask << 12)
            + 0xFFF;
    device.cpu.cop0.tlb_entries[index as usize].phys_even =
        device.cpu.cop0.tlb_entries[index as usize].pfn_even << 12;

    device.cpu.cop0.tlb_entries[index as usize].start_odd =
        device.cpu.cop0.tlb_entries[index as usize].end_even + 1;
    device.cpu.cop0.tlb_entries[index as usize].end_odd =
        device.cpu.cop0.tlb_entries[index as usize].start_odd
            + (device.cpu.cop0.tlb_entries[index as usize].mask << 12)
            + 0xFFF;
    device.cpu.cop0.tlb_entries[index as usize].phys_odd =
        device.cpu.cop0.tlb_entries[index as usize].pfn_odd << 12;

    tlb_map(device, index);
}

pub fn probe(device: &mut device::Device) {
    device.cpu.cop0.regs[device::cop0::COP0_INDEX_REG as usize] = 0x80000000; // set probe failure
    for (pos, e) in device.cpu.cop0.tlb_entries.iter().enumerate() {
        if e.vpn2 & !e.mask
            != ((device.cpu.cop0.regs[device::cop0::COP0_ENTRYHI_REG as usize] >> 13) & 0x7FFFFFF)
                & !e.mask
        {
            continue;
        }
        if e.region != (device.cpu.cop0.regs[device::cop0::COP0_ENTRYHI_REG as usize] >> 62) as u8 {
            continue;
        }
        if e.g == 0 && e.asid != device.cpu.cop0.regs[device::cop0::COP0_ENTRYHI_REG as usize] as u8
        {
            continue;
        }
        device.cpu.cop0.regs[device::cop0::COP0_INDEX_REG as usize] = pos as u64;
        break;
    }
}

pub fn tlb_unmap(device: &mut device::Device, index: u64) {
    let e = &mut device.cpu.cop0.tlb_entries[index as usize];

    if e.v_even != 0 {
        let mut i = e.start_even;
        while i < e.end_even {
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].address = 0;
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].cached = false;
            i += 0x1000
        }
        if e.d_even != 0 {
            let mut i = e.start_even;
            while i < e.end_even {
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].address = 0;
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].cached = false;
                i += 0x1000
            }
        }
    }

    if e.v_odd != 0 {
        let mut i = e.start_odd;
        while i < e.end_odd {
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].address = 0;
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].cached = false;
            i += 0x1000
        }
        if e.d_odd != 0 {
            let mut i = e.start_odd;
            while i < e.end_odd {
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].address = 0;
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].cached = false;
                i += 0x1000
            }
        }
    }
}

pub fn tlb_map(device: &mut device::Device, index: u64) {
    let e = &mut device.cpu.cop0.tlb_entries[index as usize];

    if e.v_even != 0
        && e.start_even < e.end_even
        && !(e.start_even >= 0x80000000 && e.end_even < 0xC0000000)
        && e.phys_even < 0x20000000
    {
        let mut i = e.start_even;
        while i < e.end_even {
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].address =
                0x80000000 | (e.phys_even + (i - e.start_even) + 0xFFF);
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].cached = e.c_even != 2;
            i += 0x1000
        }
        if e.d_even != 0 {
            let mut i = e.start_even;
            while i < e.end_even {
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].address =
                    0x80000000 | (e.phys_even + (i - e.start_even) + 0xFFF);
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].cached = e.c_even != 2;
                i += 0x1000
            }
        }
    }

    if e.v_odd != 0
        && e.start_odd < e.end_odd
        && !(e.start_odd >= 0x80000000 && e.end_odd < 0xC0000000)
        && e.phys_odd < 0x20000000
    {
        let mut i = e.start_odd;
        while i < e.end_odd {
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].address =
                0x80000000 | (e.phys_odd + (i - e.start_odd) + 0xFFF);
            device.cpu.cop0.tlb_lut_r[(i >> 12) as usize].cached = e.c_odd != 2;
            i += 0x1000
        }
        if e.d_odd != 0 {
            let mut i = e.start_odd;
            while i < e.end_odd {
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].address =
                    0x80000000 | (e.phys_odd + (i - e.start_odd) + 0xFFF);
                device.cpu.cop0.tlb_lut_w[(i >> 12) as usize].cached = e.c_odd != 2;
                i += 0x1000
            }
        }
    }
}

pub fn get_physical_address(
    device: &mut device::Device,
    mut address: u64,
    access_type: device::memory::AccessType,
) -> (u64, bool, bool) {
    address &= 0xffffffff;

    if access_type == device::memory::AccessType::Write {
        if device.cpu.cop0.tlb_lut_w[(address >> 12) as usize].address != 0 {
            return (
                (device.cpu.cop0.tlb_lut_w[(address >> 12) as usize].address & 0x1FFFF000)
                    | (address & 0xFFF),
                device.cpu.cop0.tlb_lut_w[(address >> 12) as usize].cached,
                false,
            );
        }
    } else if device.cpu.cop0.tlb_lut_r[(address >> 12) as usize].address != 0 {
        return (
            (device.cpu.cop0.tlb_lut_r[(address >> 12) as usize].address & 0x1FFFF000)
                | (address & 0xFFF),
            device.cpu.cop0.tlb_lut_r[(address >> 12) as usize].cached,
            false,
        );
    }

    device::exceptions::tlb_miss_exception(device, address, access_type);

    (0, false, true)
}
