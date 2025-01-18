use crate::device;

pub fn rd(opcode: u32) -> u32 {
    (opcode >> 11) & 0x1F
}

pub fn rs(opcode: u32) -> u32 {
    (opcode >> 21) & 0x1F
}

pub fn rt(opcode: u32) -> u32 {
    (opcode >> 16) & 0x1F
}

pub fn sa(opcode: u32) -> u32 {
    (opcode >> 6) & 0x1F
}

pub fn imm(opcode: u32) -> u16 {
    opcode as u16
}

pub fn se32(value: i32) -> u64 {
    value as i64 as u64
}

pub fn se16(value: i16) -> u64 {
    value as i64 as u64
}

pub fn se8(value: i8) -> u64 {
    value as i64 as u64
}

pub fn bshift<T: Into<u64>>(address: T) -> u64 {
    ((address.into() & 3) ^ 3) << 3
}

pub fn hshift<T: Into<u64>>(address: T) -> u64 {
    ((address.into() & 2) ^ 2) << 3
}

pub fn bits_below_mask<T: Into<u64>>(x: T) -> u64 {
    (1 << x.into()) - 1
}

pub fn bits_above_mask<T: Into<u64>>(x: T) -> u64 {
    !bits_below_mask(x)
}

pub fn check_relative_idle_loop(device: &mut device::Device, opcode: u32) {
    if imm(opcode) as i16 == -1
        && device.memory.fast_read[(device.cpu.pc_phys >> 16) as usize](
            device,
            device.cpu.pc_phys + 4,
            device::memory::AccessSize::Word,
        ) == 0
    {
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] = device.cpu.next_event_count
    }
}

pub fn check_absolute_idle_loop(device: &mut device::Device, opcode: u32) {
    if (opcode & 0x3FFFFFF) as u64 == (device.cpu.pc_phys & 0x0FFFFFFF) >> 2
        && device.memory.fast_read[(device.cpu.pc_phys >> 16) as usize](
            device,
            device.cpu.pc_phys + 4,
            device::memory::AccessSize::Word,
        ) == 0
    {
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] = device.cpu.next_event_count
    }
}

pub fn j(device: &mut device::Device, opcode: u32) {
    if device::cpu::in_delay_slot_taken(device) {
        return;
    }
    check_absolute_idle_loop(device, opcode);
    device.cpu.branch_state.state = device::cpu::State::Take;
    device.cpu.branch_state.pc =
        (device.cpu.pc + 4) & 0xFFFFFFFFF0000000 | ((opcode & 0x3FFFFFF) << 2) as u64
}

pub fn jal(device: &mut device::Device, opcode: u32) {
    if device::cpu::in_delay_slot_taken(device) {
        device.cpu.gpr[31] = device.cpu.branch_state.pc + 4
    } else {
        device.cpu.gpr[31] = device.cpu.pc + 8
    }
    if !device::cpu::in_delay_slot_taken(device) {
        check_absolute_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc =
            (device.cpu.pc + 4) & 0xFFFFFFFFF0000000 | ((opcode & 0x3FFFFFF) << 2) as u64
    } else if !device::cpu::in_delay_slot(device) {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn beq(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] == device.cpu.gpr[rt(opcode) as usize] {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bne(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] != device.cpu.gpr[rt(opcode) as usize] {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn blez(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] as i64 <= 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bgtz(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] as i64 > 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn addi(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] =
        se32(device.cpu.gpr[rs(opcode) as usize] as i32).wrapping_add(se16(imm(opcode) as i16))
}

pub fn addiu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] =
        se32(device.cpu.gpr[rs(opcode) as usize] as i32).wrapping_add(se16(imm(opcode) as i16))
}

pub fn slti(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] =
        ((device.cpu.gpr[rs(opcode) as usize] as i64) < (imm(opcode) as i16 as i64)) as u64
}

pub fn sltiu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] =
        (device.cpu.gpr[rs(opcode) as usize] < se16(imm(opcode) as i16)) as u64
}

pub fn andi(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] = device.cpu.gpr[rs(opcode) as usize] & imm(opcode) as u64
}

pub fn ori(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] = device.cpu.gpr[rs(opcode) as usize] | imm(opcode) as u64
}

pub fn xori(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] = device.cpu.gpr[rs(opcode) as usize] ^ imm(opcode) as u64
}

pub fn lui(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] = se32(((imm(opcode) as u32) << 16) as i32)
}

pub fn beql(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] == device.cpu.gpr[rt(opcode) as usize] {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn bnel(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] != device.cpu.gpr[rt(opcode) as usize] {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn blezl(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] as i64 <= 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn bgtzl(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] as i64 > 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn daddi(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16))
}

pub fn daddiu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rt(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16))
}

pub fn ldl(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * n;

    let mask = bits_below_mask(8 * n);

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !7;

    let mut w = [0; 2];
    w[0] = device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Dword,
        cached,
    );
    w[1] = device::memory::data_read(
        device,
        phys_address + 4,
        device::memory::AccessSize::Dword,
        cached,
    );
    device.cpu.gpr[rt(opcode) as usize] = (device.cpu.gpr[rt(opcode) as usize] & mask)
        | (((((w[0]) as u64) << 32) | (w[1]) as u64) << shift)
}

pub fn ldr(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * (7 - n);

    let mask = if n == 7 {
        0
    } else {
        bits_above_mask(8 * (n + 1))
    };

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !7;

    let mut w = [0; 2];
    w[0] = device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Dword,
        cached,
    );
    w[1] = device::memory::data_read(
        device,
        phys_address + 4,
        device::memory::AccessSize::Dword,
        cached,
    );
    device.cpu.gpr[rt(opcode) as usize] = (device.cpu.gpr[rt(opcode) as usize] & mask)
        | (((((w[0]) as u64) << 32) | (w[1]) as u64) >> shift)
}

pub fn lb(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = bshift(phys_address);
    device.cpu.gpr[rt(opcode) as usize] = se8((device::memory::data_read(
        device,
        phys_address & !0x3,
        device::memory::AccessSize::Word,
        cached,
    ) >> shift) as u8 as i8)
}

pub fn lh(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = hshift(phys_address);
    device.cpu.gpr[rt(opcode) as usize] = se16(
        (device::memory::data_read(
            device,
            phys_address & !0x3,
            device::memory::AccessSize::Word,
            cached,
        ) >> shift) as u16 as i16,
    )
}

pub fn lwl(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * n;

    let mask = bits_below_mask(8 * n) as u32;

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !3;

    device.cpu.gpr[rt(opcode) as usize] = se32(
        ((device.cpu.gpr[rt(opcode) as usize] as u32) & mask
            | (device::memory::data_read(
                device,
                phys_address,
                device::memory::AccessSize::Word,
                cached,
            ) << shift)) as i32,
    )
}

pub fn lw(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    device.cpu.gpr[rt(opcode) as usize] = se32(device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Word,
        cached,
    ) as i32)
}

pub fn lbu(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = bshift(phys_address);
    device.cpu.gpr[rt(opcode) as usize] = (device::memory::data_read(
        device,
        phys_address & !0x3,
        device::memory::AccessSize::Word,
        cached,
    ) >> shift) as u8 as u64
}

pub fn lhu(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = hshift(phys_address);
    device.cpu.gpr[rt(opcode) as usize] = (device::memory::data_read(
        device,
        phys_address & !0x3,
        device::memory::AccessSize::Word,
        cached,
    ) >> shift) as u16 as u64
}

pub fn lwr(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * (3 - n);

    let mask = if n == 3 {
        0
    } else {
        bits_above_mask(8 * (n + 1))
    };

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !3;

    device.cpu.gpr[rt(opcode) as usize] = se32(
        ((device.cpu.gpr[rt(opcode) as usize] as u32) & (mask as u32)
            | (device::memory::data_read(
                device,
                phys_address,
                device::memory::AccessSize::Word,
                cached,
            ) >> shift)) as i32,
    )
}

pub fn lwu(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    device.cpu.gpr[rt(opcode) as usize] = device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Word,
        cached,
    ) as u64
}

pub fn sb(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Write,
    );
    if err {
        return;
    }

    let shift = bshift(phys_address);

    device::memory::data_write(
        device,
        phys_address & !0x3,
        (device.cpu.gpr[rt(opcode) as usize] as u32) << shift,
        0xFF << shift,
        cached,
    )
}

pub fn sh(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Write,
    );
    if err {
        return;
    }

    let shift = hshift(phys_address);

    device::memory::data_write(
        device,
        phys_address & !0x3,
        (device.cpu.gpr[rt(opcode) as usize] as u32) << shift,
        0xFFFF << shift,
        cached,
    )
}

pub fn swl(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * n;

    let mask = if n == 0 {
        u32::MAX
    } else {
        bits_below_mask(8 * (4 - n)) as u32
    };

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !3;

    device::memory::data_write(
        device,
        phys_address,
        (device.cpu.gpr[rt(opcode) as usize] >> shift) as u32,
        mask,
        cached,
    )
}

pub fn sw(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Write,
    );
    if err {
        return;
    }

    device::memory::data_write(
        device,
        phys_address,
        device.cpu.gpr[rt(opcode) as usize] as u32,
        0xFFFFFFFF,
        cached,
    )
}

pub fn sdl(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * n;

    let mask = if n == 0 {
        u64::MAX
    } else {
        bits_below_mask(8 * (8 - n))
    };

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !7;

    let value = device.cpu.gpr[rt(opcode) as usize] >> shift;
    device::memory::data_write(
        device,
        phys_address,
        (value >> 32) as u32,
        (mask >> 32) as u32,
        cached,
    );
    device::memory::data_write(device, phys_address + 4, value as u32, mask as u32, cached)
}

pub fn sdr(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * (7 - n);

    let mask = bits_above_mask(8 * (7 - n));

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !7;

    let value = device.cpu.gpr[rt(opcode) as usize] << shift;
    device::memory::data_write(
        device,
        phys_address,
        (value >> 32) as u32,
        (mask >> 32) as u32,
        cached,
    );
    device::memory::data_write(device, phys_address + 4, value as u32, mask as u32, cached)
}

pub fn swr(device: &mut device::Device, opcode: u32) {
    let addr = device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * (3 - n);

    let mask = bits_above_mask(8 * (3 - n)) as u32;

    let (mut phys_address, cached, err) =
        device::memory::translate_address(device, addr, device::memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !3;

    device::memory::data_write(
        device,
        phys_address,
        (device.cpu.gpr[rt(opcode) as usize] << shift) as u32,
        mask,
        cached,
    )
}

pub fn cache(device: &mut device::Device, opcode: u32) {
    let (phys_address, _, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let dcache_line = ((phys_address >> 4) & 0x1FF) as usize;
    let icache_line = ((phys_address >> 5) & 0x1FF) as usize;
    match rt(opcode) {
        0x00 => {
            //icache index invalidate
            device.memory.icache[icache_line].valid = false
        }
        0x04 => {
            //icache load tag
            let mut valid = 0;
            if device.memory.icache[icache_line].valid {
                valid = 1
            }
            device::memory::masked_write_64(
                &mut device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize],
                valid << 7,
                0x80,
            );
            device::memory::masked_write_64(
                &mut device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize],
                0,
                0x40,
            );
            device::memory::masked_write_64(
                &mut device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize],
                (device.memory.icache[icache_line].tag >> 4) as u64,
                0xFFFFF00,
            )
        }
        0x08 => {
            //icache store tag
            device.memory.icache[icache_line].valid =
                (device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize] & 0x80) >> 7 != 0;
            device.memory.icache[icache_line].tag =
                ((device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize] & 0xFFFFF00) << 4)
                    as u32
        }
        0x10 => {
            //icache hit invalidate
            if device::cache::icache_hit(device, icache_line, phys_address) {
                device.memory.icache[icache_line].valid = false
            }
        }
        0x14 => {
            //icache fill
            device::cache::icache_fill(device, icache_line, phys_address)
        }
        0x18 => {
            //icache hit write back
            if device::cache::icache_hit(device, icache_line, phys_address) {
                device::cache::icache_writeback(device, icache_line)
            }
        }
        0x01 => {
            //dcache index write back invalidate
            if device.memory.dcache[dcache_line].dirty && device.memory.dcache[dcache_line].valid {
                device::cache::dcache_writeback(device, dcache_line)
            }
            device.memory.dcache[dcache_line].valid = false
        }
        0x05 => {
            //dcache index load tag
            let mut valid = 0;
            let mut dirty = 0;
            if device.memory.dcache[dcache_line].valid {
                valid = 1
            }
            if device.memory.dcache[dcache_line].dirty {
                dirty = 1
            }
            device::memory::masked_write_64(
                &mut device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize],
                valid << 7,
                0x80,
            );
            device::memory::masked_write_64(
                &mut device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize],
                dirty << 6,
                0x40,
            );
            device::memory::masked_write_64(
                &mut device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize],
                (device.memory.dcache[dcache_line].tag >> 4) as u64,
                0xFFFFF00,
            )
        }
        0x09 => {
            //dcache index store tag
            device.memory.dcache[dcache_line].valid =
                (device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize] & 0x80) >> 7 != 0;
            device.memory.dcache[dcache_line].dirty =
                (device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize] & 0x40) >> 6 != 0;
            device.memory.dcache[dcache_line].tag =
                ((device.cpu.cop0.regs[device::cop0::COP0_TAGLO_REG as usize] & 0xFFFFF00) << 4)
                    as u32
        }
        0x0D => {
            //dcache create dirty exclusive
            if !device::cache::dcache_hit(device, dcache_line, phys_address)
                && device.memory.dcache[dcache_line].dirty
            {
                device::cache::dcache_writeback(device, dcache_line)
            }
            device.memory.dcache[dcache_line].tag = (phys_address & !0xFFF) as u32;
            device.memory.dcache[dcache_line].valid = true;
            device.memory.dcache[dcache_line].dirty = true
        }
        0x11 => {
            //dcache hit invalidate
            if device::cache::dcache_hit(device, dcache_line, phys_address) {
                device.memory.dcache[dcache_line].valid = false;
                device.memory.dcache[dcache_line].dirty = false
            }
        }
        0x15 => {
            //dcache hit write back invalidate
            if device::cache::dcache_hit(device, dcache_line, phys_address) {
                if device.memory.dcache[dcache_line].dirty {
                    device::cache::dcache_writeback(device, dcache_line)
                }
                device.memory.dcache[dcache_line].valid = false
            }
        }
        0x19 => {
            //dcache hit write back
            if device::cache::dcache_hit(device, dcache_line, phys_address)
                && device.memory.dcache[dcache_line].dirty
            {
                device::cache::dcache_writeback(device, dcache_line)
            }
        }
        _ => {
            panic!("unknown cache code {:#01x}", rt(opcode))
        }
    }
}

pub fn ll(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    device.cpu.gpr[rt(opcode) as usize] = se32(device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Word,
        cached,
    ) as i32);
    device.cpu.llbit = true;
    device.cpu.cop0.regs[device::cop0::COP0_LLADDR_REG as usize] = phys_address >> 4
}

pub fn lld(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let mut w = [0; 2];
    w[0] = device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Dword,
        cached,
    );
    w[1] = device::memory::data_read(
        device,
        phys_address + 4,
        device::memory::AccessSize::Dword,
        cached,
    );
    device.cpu.gpr[rt(opcode) as usize] = ((w[0] as u64) << 32) | (w[1]) as u64;

    device.cpu.llbit = true;
    device.cpu.cop0.regs[device::cop0::COP0_LLADDR_REG as usize] = phys_address >> 4
}

pub fn ld(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let mut w = [0; 2];
    w[0] = device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Dword,
        cached,
    );
    w[1] = device::memory::data_read(
        device,
        phys_address + 4,
        device::memory::AccessSize::Dword,
        cached,
    );
    device.cpu.gpr[rt(opcode) as usize] = ((w[0] as u64) << 32) | (w[1]) as u64
}

pub fn sc(device: &mut device::Device, opcode: u32) {
    if device.cpu.llbit {
        let (phys_address, cached, err) = device::memory::translate_address(
            device,
            device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
            device::memory::AccessType::Write,
        );
        if err {
            return;
        }

        device.cpu.llbit = false;
        device::memory::data_write(
            device,
            phys_address,
            device.cpu.gpr[rt(opcode) as usize] as u32,
            0xFFFFFFFF,
            cached,
        );
        device.cpu.gpr[rt(opcode) as usize] = 1
    } else {
        device.cpu.gpr[rt(opcode) as usize] = 0
    }
}

pub fn scd(device: &mut device::Device, opcode: u32) {
    if device.cpu.llbit {
        let (phys_address, cached, err) = device::memory::translate_address(
            device,
            device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
            device::memory::AccessType::Write,
        );
        if err {
            return;
        }

        device.cpu.llbit = false;
        device::memory::data_write(
            device,
            phys_address,
            (device.cpu.gpr[rt(opcode) as usize] >> 32) as u32,
            0xFFFFFFFF,
            cached,
        );
        device::memory::data_write(
            device,
            phys_address + 4,
            device.cpu.gpr[rt(opcode) as usize] as u32,
            0xFFFFFFFF,
            cached,
        );
        device.cpu.gpr[rt(opcode) as usize] = 1
    } else {
        device.cpu.gpr[rt(opcode) as usize] = 0
    }
}

pub fn sd(device: &mut device::Device, opcode: u32) {
    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        device::memory::AccessType::Write,
    );
    if err {
        return;
    }

    device::memory::data_write(
        device,
        phys_address,
        (device.cpu.gpr[rt(opcode) as usize] >> 32) as u32,
        0xFFFFFFFF,
        cached,
    );
    device::memory::data_write(
        device,
        phys_address + 4,
        device.cpu.gpr[rt(opcode) as usize] as u32,
        0xFFFFFFFF,
        cached,
    )
}

pub fn sll(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        se32(((device.cpu.gpr[rt(opcode) as usize] as u32) << sa(opcode)) as i32)
}

pub fn srl(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        se32(((device.cpu.gpr[rt(opcode) as usize] as u32) >> sa(opcode)) as i32)
}

pub fn sra(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        se32(((device.cpu.gpr[rt(opcode) as usize] as i64) >> sa(opcode)) as i32)
}

pub fn sllv(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = se32(
        ((device.cpu.gpr[rt(opcode) as usize] as u32)
            << (device.cpu.gpr[rs(opcode) as usize] as u32 & 31)) as i32,
    )
}

pub fn srlv(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = se32(
        ((device.cpu.gpr[rt(opcode) as usize] as u32)
            >> (device.cpu.gpr[rs(opcode) as usize] as u32 & 31)) as i32,
    )
}

pub fn srav(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = se32(
        ((device.cpu.gpr[rt(opcode) as usize] as i64) >> (device.cpu.gpr[rs(opcode) as usize] & 31))
            as i32,
    )
}

pub fn jr(device: &mut device::Device, opcode: u32) {
    if !device::cpu::in_delay_slot_taken(device) {
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.gpr[rs(opcode) as usize];
    } else if !device::cpu::in_delay_slot(device) {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn jalr(device: &mut device::Device, opcode: u32) {
    let in_delay_slot_taken = device::cpu::in_delay_slot_taken(device);

    if !in_delay_slot_taken {
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.gpr[rs(opcode) as usize];
    } else if !device::cpu::in_delay_slot(device) {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }

    if in_delay_slot_taken {
        device.cpu.gpr[rd(opcode) as usize] = device.cpu.branch_state.pc + 4
    } else {
        device.cpu.gpr[rd(opcode) as usize] = device.cpu.pc + 8
    }
}

pub fn sync(_device: &mut device::Device, _opcode: u32) {}

pub fn mfhi(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = device.cpu.hi
}

pub fn mthi(device: &mut device::Device, opcode: u32) {
    device.cpu.hi = device.cpu.gpr[rs(opcode) as usize]
}

pub fn mflo(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = device.cpu.lo
}

pub fn mtlo(device: &mut device::Device, opcode: u32) {
    device.cpu.lo = device.cpu.gpr[rs(opcode) as usize]
}

pub fn dsllv(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rt(opcode) as usize] << (device.cpu.gpr[rs(opcode) as usize] & 63)
}

pub fn dsrlv(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rt(opcode) as usize] >> (device.cpu.gpr[rs(opcode) as usize] & 63)
}

pub fn dsrav(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = ((device.cpu.gpr[rt(opcode) as usize] as i64)
        >> (device.cpu.gpr[rs(opcode) as usize] & 63))
        as u64
}

pub fn mult(device: &mut device::Device, opcode: u32) {
    let result = ((device.cpu.gpr[rs(opcode) as usize]) as i32 as i64)
        .wrapping_mul(device.cpu.gpr[rt(opcode) as usize] as i32 as i64);

    device.cpu.lo = se32((result) as i32);
    device.cpu.hi = se32((result >> 32) as i32);

    device::cop0::add_cycles(device, 4);
}

pub fn multu(device: &mut device::Device, opcode: u32) {
    let result = (device.cpu.gpr[rs(opcode) as usize] as u32 as u64)
        .wrapping_mul(device.cpu.gpr[rt(opcode) as usize] as u32 as u64);

    device.cpu.lo = se32((result) as u32 as i32);
    device.cpu.hi = se32((result >> 32) as u32 as i32);

    device::cop0::add_cycles(device, 4);
}

pub fn div(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rt(opcode) as usize]) as i32 != 0 {
        device.cpu.lo = se32(
            (device.cpu.gpr[rs(opcode) as usize] as i32)
                .wrapping_div(device.cpu.gpr[rt(opcode) as usize] as i32),
        );
        device.cpu.hi = se32(
            (device.cpu.gpr[rs(opcode) as usize] as i32)
                .wrapping_rem(device.cpu.gpr[rt(opcode) as usize] as i32),
        );
    } else {
        if (device.cpu.gpr[rs(opcode) as usize] as i32) < 0 {
            device.cpu.lo = 1
        } else {
            device.cpu.lo = u64::MAX
        }
        device.cpu.hi = se32(device.cpu.gpr[rs(opcode) as usize] as i32)
    }

    device::cop0::add_cycles(device, 36)
}

pub fn divu(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rt(opcode) as usize]) as u32 != 0 {
        device.cpu.lo = se32(
            ((device.cpu.gpr[rs(opcode) as usize] as u32)
                / (device.cpu.gpr[rt(opcode) as usize] as u32)) as i32,
        );
        device.cpu.hi = se32(
            ((device.cpu.gpr[rs(opcode) as usize] as u32)
                % (device.cpu.gpr[rt(opcode) as usize] as u32)) as i32,
        );
    } else {
        device.cpu.lo = u64::MAX;
        device.cpu.hi = se32(device.cpu.gpr[rs(opcode) as usize] as u32 as i32)
    }

    device::cop0::add_cycles(device, 36)
}

pub fn dmult(device: &mut device::Device, opcode: u32) {
    let result = ((device.cpu.gpr[rs(opcode) as usize] as i64 as i128)
        * (device.cpu.gpr[rt(opcode) as usize] as i64 as i128)) as u128;
    device.cpu.lo = result as u64;
    device.cpu.hi = (result >> 64) as u64;
    device::cop0::add_cycles(device, 7)
}

pub fn dmultu(device: &mut device::Device, opcode: u32) {
    let result = (device.cpu.gpr[rs(opcode) as usize] as u128)
        * (device.cpu.gpr[rt(opcode) as usize] as u128);
    device.cpu.lo = result as u64;
    device.cpu.hi = (result >> 64) as u64;
    device::cop0::add_cycles(device, 7)
}

pub fn ddiv(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rt(opcode) as usize] as i64 != 0 {
        device.cpu.lo = (((device.cpu.gpr[rs(opcode) as usize] as i64) as i128)
            / (device.cpu.gpr[rt(opcode) as usize] as i64) as i128) as u64;
        device.cpu.hi = (((device.cpu.gpr[rs(opcode) as usize] as i64) as i128)
            % (device.cpu.gpr[rt(opcode) as usize] as i64) as i128) as u64;
    } else {
        if ((device.cpu.gpr[rs(opcode) as usize]) as i64) < 0 {
            device.cpu.lo = 1
        } else {
            device.cpu.lo = u64::MAX
        }
        device.cpu.hi = device.cpu.gpr[rs(opcode) as usize]
    }
    device::cop0::add_cycles(device, 68);
}

pub fn ddivu(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rt(opcode) as usize] != 0 {
        device.cpu.lo = device.cpu.gpr[rs(opcode) as usize] / device.cpu.gpr[rt(opcode) as usize];
        device.cpu.hi = device.cpu.gpr[rs(opcode) as usize] % device.cpu.gpr[rt(opcode) as usize]
    } else {
        device.cpu.lo = u64::MAX;
        device.cpu.hi = device.cpu.gpr[rs(opcode) as usize]
    }
    device::cop0::add_cycles(device, 68);
}

pub fn add(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = se32(
        (device.cpu.gpr[rs(opcode) as usize] as u32)
            .wrapping_add(device.cpu.gpr[rt(opcode) as usize] as u32) as i32,
    )
}

pub fn addu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = se32(
        (device.cpu.gpr[rs(opcode) as usize] as u32)
            .wrapping_add(device.cpu.gpr[rt(opcode) as usize] as u32) as i32,
    )
}

pub fn sub(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = se32(
        (device.cpu.gpr[rs(opcode) as usize] as u32)
            .wrapping_sub(device.cpu.gpr[rt(opcode) as usize] as u32) as i32,
    )
}

pub fn subu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = se32(
        (device.cpu.gpr[rs(opcode) as usize] as u32)
            .wrapping_sub(device.cpu.gpr[rt(opcode) as usize] as u32) as i32,
    )
}

pub fn and(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize] & device.cpu.gpr[rt(opcode) as usize]
}

pub fn or(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize] | device.cpu.gpr[rt(opcode) as usize]
}

pub fn xor(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize] ^ device.cpu.gpr[rt(opcode) as usize]
}

pub fn nor(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        !(device.cpu.gpr[rs(opcode) as usize] | device.cpu.gpr[rt(opcode) as usize])
}

pub fn slt(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = ((device.cpu.gpr[rs(opcode) as usize] as i64)
        < (device.cpu.gpr[rt(opcode) as usize] as i64))
        as u64
}

pub fn sltu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        (device.cpu.gpr[rs(opcode) as usize] < device.cpu.gpr[rt(opcode) as usize]) as u64
}

pub fn dadd(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(device.cpu.gpr[rt(opcode) as usize])
}

pub fn daddu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize].wrapping_add(device.cpu.gpr[rt(opcode) as usize])
}

pub fn dsub(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize].wrapping_sub(device.cpu.gpr[rt(opcode) as usize])
}

pub fn dsubu(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        device.cpu.gpr[rs(opcode) as usize].wrapping_sub(device.cpu.gpr[rt(opcode) as usize])
}

pub fn tge(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) >= (device.cpu.gpr[rt(opcode) as usize] as i64)
    {
        device::exceptions::trap_exception(device)
    }
}

pub fn tgeu(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] >= device.cpu.gpr[rt(opcode) as usize] {
        device::exceptions::trap_exception(device)
    }
}

pub fn tlt(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) < (device.cpu.gpr[rt(opcode) as usize] as i64) {
        device::exceptions::trap_exception(device)
    }
}

pub fn tltu(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] < device.cpu.gpr[rt(opcode) as usize] {
        device::exceptions::trap_exception(device)
    }
}

pub fn teq(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] == device.cpu.gpr[rt(opcode) as usize] {
        device::exceptions::trap_exception(device)
    }
}

pub fn tne(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] != device.cpu.gpr[rt(opcode) as usize] {
        device::exceptions::trap_exception(device)
    }
}

pub fn dsll(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = device.cpu.gpr[rt(opcode) as usize] << sa(opcode)
}

pub fn dsrl(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = device.cpu.gpr[rt(opcode) as usize] >> sa(opcode)
}

pub fn dsra(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        ((device.cpu.gpr[rt(opcode) as usize] as i64) >> sa(opcode)) as u64
}

pub fn dsll32(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = device.cpu.gpr[rt(opcode) as usize] << (32 + sa(opcode))
}

pub fn dsrl32(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] = device.cpu.gpr[rt(opcode) as usize] >> (32 + sa(opcode))
}

pub fn dsra32(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[rd(opcode) as usize] =
        ((device.cpu.gpr[rt(opcode) as usize] as i64) >> (32 + sa(opcode))) as u64
}

pub fn bltz(device: &mut device::Device, opcode: u32) {
    if ((device.cpu.gpr[rs(opcode) as usize]) as i64) < 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bgez(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) >= 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bltzl(device: &mut device::Device, opcode: u32) {
    if ((device.cpu.gpr[rs(opcode) as usize]) as i64) < 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn bgezl(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) >= 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn tgei(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) >= (imm(opcode) as i16 as i64) {
        device::exceptions::trap_exception(device)
    }
}

pub fn tgeiu(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] >= se16(imm(opcode) as i16) {
        device::exceptions::trap_exception(device)
    }
}

pub fn tlti(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) < (imm(opcode) as i16 as i64) {
        device::exceptions::trap_exception(device)
    }
}

pub fn tltiu(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] < se16(imm(opcode) as i16) {
        device::exceptions::trap_exception(device)
    }
}

pub fn teqi(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) == (imm(opcode) as i16 as i64) {
        device::exceptions::trap_exception(device)
    }
}

pub fn tnei(device: &mut device::Device, opcode: u32) {
    if (device.cpu.gpr[rs(opcode) as usize] as i64) != (imm(opcode) as i16 as i64) {
        device::exceptions::trap_exception(device)
    }
}

pub fn bltzal(device: &mut device::Device, opcode: u32) {
    if ((device.cpu.gpr[rs(opcode) as usize]) as i64) < 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
    device.cpu.gpr[31] = device.cpu.pc + 8
}

pub fn bgezal(device: &mut device::Device, opcode: u32) {
    let in_delay_slot = device::cpu::in_delay_slot(device);

    if device.cpu.gpr[rs(opcode) as usize] as i64 >= 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }

    if in_delay_slot {
        device.cpu.gpr[31] = device.cpu.branch_state.pc + 4
    } else {
        device.cpu.gpr[31] = device.cpu.pc + 8
    }
}

pub fn bltzall(device: &mut device::Device, opcode: u32) {
    if ((device.cpu.gpr[rs(opcode) as usize]) as i64) < 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
    device.cpu.gpr[31] = device.cpu.pc + 8
}

pub fn bgezall(device: &mut device::Device, opcode: u32) {
    if device.cpu.gpr[rs(opcode) as usize] as i64 >= 0 {
        check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
    device.cpu.gpr[31] = device.cpu.pc + 8
}
