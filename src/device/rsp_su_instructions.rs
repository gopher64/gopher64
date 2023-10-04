use crate::device;
use std::arch::x86_64::*;

pub fn rd(opcode: u32) -> u32 {
    return (opcode >> 11) & 0x1F;
}

pub fn rs(opcode: u32) -> u32 {
    return (opcode >> 21) & 0x1F;
}

pub fn rt(opcode: u32) -> u32 {
    return (opcode >> 16) & 0x1F;
}

pub fn sa(opcode: u32) -> u32 {
    return (opcode >> 6) & 0x1F;
}

pub fn imm(opcode: u32) -> u16 {
    return opcode as u16;
}

pub fn voffset(opcode: u32) -> u8 {
    return (opcode & 0x7F) as u8;
}

pub fn velement(opcode: u32) -> u8 {
    return ((opcode >> 7) & 0xF) as u8;
}

pub fn sign_extend_7bit_offset(offset: u8, shift_amount: u32) -> u32 {
    let soffset = (((offset << 1) & 0x80) | offset) as i8;

    return (((soffset) as i32) as u32) << shift_amount;
}

pub fn modify_vpr_byte(vpr: &mut u128, value: u8, element: u8) {
    let pos = 15 - (element & 15);
    let mask = 0xFF << (pos * 8);
    *vpr &= !mask;
    *vpr |= (value as u128) << (pos * 8);
}

pub fn get_vpr_byte(vpr: u128, element: u8) -> u8 {
    let pos = 15 - (element & 15);
    return (vpr >> (pos * 8)) as u8;
}

pub fn modify_vpr_element(vpr: &mut u128, value: u16, element: u8) {
    let pos = 7 - (element & 7);
    let mask = 0xFFFF << (pos * 16);
    *vpr &= !mask;
    *vpr |= (value as u128) << (pos * 16);
}

pub fn get_vpr_element(vpr: u128, element: u8) -> u16 {
    let pos = 7 - (element & 7);
    return (vpr >> (pos * 16)) as u16;
}

pub fn j(device: &mut device::Device, opcode: u32) {
    if device::rsp_cpu::in_delay_slot_taken(device) {
        return;
    }
    device.rsp.cpu.branch_state.state = device::cpu::State::Take;
    device.rsp.cpu.branch_state.pc =
        (device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] + 4) & 0xF0000000
            | ((opcode & 0x3FFFFFF) << 2) as u32
}

pub fn jal(device: &mut device::Device, opcode: u32) {
    if device::rsp_cpu::in_delay_slot_taken(device) {
        device.rsp.cpu.gpr[31] = (device.rsp.cpu.branch_state.pc + 4) & 0xFFF
    } else {
        device.rsp.cpu.gpr[31] =
            (device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] + 8) & 0xFFF
    }
    if !device::rsp_cpu::in_delay_slot_taken(device) {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc =
            (device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] + 4) & 0xF0000000
                | ((opcode & 0x3FFFFFF) << 2) as u32
    } else if !device::rsp_cpu::in_delay_slot(device) {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn beq(device: &mut device::Device, opcode: u32) {
    if device.rsp.cpu.gpr[rs(opcode) as usize] == device.rsp.cpu.gpr[rt(opcode) as usize] {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bne(device: &mut device::Device, opcode: u32) {
    if device.rsp.cpu.gpr[rs(opcode) as usize] != device.rsp.cpu.gpr[rt(opcode) as usize] {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn blez(device: &mut device::Device, opcode: u32) {
    if device.rsp.cpu.gpr[rs(opcode) as usize] as i32 <= 0 {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bgtz(device: &mut device::Device, opcode: u32) {
    if device.rsp.cpu.gpr[rs(opcode) as usize] as i32 > 0 {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn addi(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32)
}

pub fn addiu(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32)
}

pub fn slti(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] =
        ((device.rsp.cpu.gpr[rs(opcode) as usize] as i32) < (imm(opcode) as i16 as i32)) as u32
}

pub fn sltiu(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] =
        (device.rsp.cpu.gpr[rs(opcode) as usize] < (imm(opcode) as i16 as i32 as u32)) as u32
}

pub fn andi(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize] & (imm(opcode)) as u32
}

pub fn ori(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize] | (imm(opcode)) as u32
}

pub fn xori(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize] ^ (imm(opcode)) as u32
}

pub fn lui(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rt(opcode) as usize] = (imm(opcode) as u32) << 16
}

pub fn lb(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    device.rsp.cpu.gpr[rt(opcode) as usize] =
        device.rsp.mem[address as usize & 0xFFF] as i8 as i32 as u32
}

pub fn lh(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    let mut w = [0; 2];
    w[0] = device.rsp.mem[address as usize & 0xFFF];
    w[1] = device.rsp.mem[(address as usize + 1) & &0xFFF];

    device.rsp.cpu.gpr[rt(opcode) as usize] =
        (((w[0] as u16) << 8) | w[1] as u16) as i16 as i32 as u32
}

pub fn lw(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    let mut w = [0; 4];
    w[0] = device.rsp.mem[address as usize & 0xFFF];
    w[1] = device.rsp.mem[(address as usize + 1) & &0xFFF];
    w[2] = device.rsp.mem[(address as usize + 2) & &0xFFF];
    w[3] = device.rsp.mem[(address as usize + 3) & &0xFFF];

    device.rsp.cpu.gpr[rt(opcode) as usize] =
        (w[0] as u32) << 24 | (w[1] as u32) << 16 | (w[2] as u32) << 8 | (w[3] as u32)
}

pub fn lbu(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    device.rsp.cpu.gpr[rt(opcode) as usize] = device.rsp.mem[address as usize & 0xFFF] as u32
}

pub fn lhu(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    let mut w = [0; 2];
    w[0] = device.rsp.mem[address as usize & 0xFFF];
    w[1] = device.rsp.mem[(address as usize + 1) & &0xFFF];

    device.rsp.cpu.gpr[rt(opcode) as usize] = (((w[0] as u16) << 8) | w[1] as u16) as u32
}

pub fn lwu(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    let mut w = [0; 4];
    w[0] = device.rsp.mem[address as usize & 0xFFF];
    w[1] = device.rsp.mem[(address as usize + 1) & &0xFFF];
    w[2] = device.rsp.mem[(address as usize + 2) & &0xFFF];
    w[3] = device.rsp.mem[(address as usize + 3) & &0xFFF];

    device.rsp.cpu.gpr[rt(opcode) as usize] =
        (w[0] as u32) << 24 | (w[1] as u32) << 16 | (w[2] as u32) << 8 | (w[3] as u32)
}

pub fn sb(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    device.rsp.mem[address as usize & 0xFFF] = (device.rsp.cpu.gpr[rt(opcode) as usize]) as u8;
}

pub fn sh(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    device.rsp.mem[address as usize & 0xFFF] = (device.rsp.cpu.gpr[rt(opcode) as usize] >> 8) as u8;
    device.rsp.mem[(address as usize + 1) & 0xFFF] =
        (device.rsp.cpu.gpr[rt(opcode) as usize]) as u8;
}

pub fn sw(device: &mut device::Device, opcode: u32) {
    let address =
        device.rsp.cpu.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    device.rsp.mem[address as usize & 0xFFF] =
        (device.rsp.cpu.gpr[rt(opcode) as usize] >> 24) as u8;
    device.rsp.mem[(address as usize + 1) & 0xFFF] =
        (device.rsp.cpu.gpr[rt(opcode) as usize] >> 16) as u8;
    device.rsp.mem[(address as usize + 2) & 0xFFF] =
        (device.rsp.cpu.gpr[rt(opcode) as usize] >> 8) as u8;
    device.rsp.mem[(address as usize + 3) & 0xFFF] =
        (device.rsp.cpu.gpr[rt(opcode) as usize]) as u8;
}

pub fn sll(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        (device.rsp.cpu.gpr[rt(opcode) as usize] as u32) << sa(opcode)
}

pub fn srl(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        (device.rsp.cpu.gpr[rt(opcode) as usize] as u32) >> sa(opcode)
}

pub fn sra(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        ((device.rsp.cpu.gpr[rt(opcode) as usize] as i32) >> sa(opcode)) as u32
}

pub fn sllv(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        device.rsp.cpu.gpr[rt(opcode) as usize] << (device.rsp.cpu.gpr[rs(opcode) as usize] & 31)
}

pub fn srlv(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        device.rsp.cpu.gpr[rt(opcode) as usize] >> (device.rsp.cpu.gpr[rs(opcode) as usize] & 31)
}

pub fn srav(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] = ((device.rsp.cpu.gpr[rt(opcode) as usize] as i32)
        >> (device.rsp.cpu.gpr[rs(opcode) as usize] & 31))
        as u32
}

pub fn jr(device: &mut device::Device, opcode: u32) {
    if !device::rsp_cpu::in_delay_slot_taken(device) {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.cpu.gpr[rs(opcode) as usize]
    } else if !device::rsp_cpu::in_delay_slot(device) {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn jalr(device: &mut device::Device, opcode: u32) {
    let in_delay_slot_taken = device::rsp_cpu::in_delay_slot_taken(device);

    if !in_delay_slot_taken {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.cpu.gpr[rs(opcode) as usize]
    } else if !device::rsp_cpu::in_delay_slot(device) {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }

    if in_delay_slot_taken {
        device.rsp.cpu.gpr[rd(opcode) as usize] = (device.rsp.cpu.branch_state.pc + 4) & 0xFFF
    } else {
        device.rsp.cpu.gpr[rd(opcode) as usize] =
            (device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] + 8) & 0xFFF
    }
}

pub fn break_(device: &mut device::Device, _opcode: u32) {
    device.rsp.cpu.broken = true;
}

pub fn add(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(device.rsp.cpu.gpr[rt(opcode) as usize])
}

pub fn addu(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(device.rsp.cpu.gpr[rt(opcode) as usize])
}

pub fn sub(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_sub(device.rsp.cpu.gpr[rt(opcode) as usize])
}

pub fn subu(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_sub(device.rsp.cpu.gpr[rt(opcode) as usize])
}

pub fn and(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize] & device.rsp.cpu.gpr[rt(opcode) as usize]
}

pub fn or(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize] | device.rsp.cpu.gpr[rt(opcode) as usize]
}

pub fn xor(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        device.rsp.cpu.gpr[rs(opcode) as usize] ^ device.rsp.cpu.gpr[rt(opcode) as usize]
}

pub fn nor(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        !(device.rsp.cpu.gpr[rs(opcode) as usize] | device.rsp.cpu.gpr[rt(opcode) as usize])
}

pub fn slt(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] = ((device.rsp.cpu.gpr[rs(opcode) as usize] as i32)
        < (device.rsp.cpu.gpr[rt(opcode) as usize] as i32))
        as u32
}

pub fn sltu(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.gpr[rd(opcode) as usize] =
        (device.rsp.cpu.gpr[rs(opcode) as usize] < device.rsp.cpu.gpr[rt(opcode) as usize]) as u32
}

pub fn bltz(device: &mut device::Device, opcode: u32) {
    if (device.rsp.cpu.gpr[rs(opcode) as usize] as i32) < 0 {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bgez(device: &mut device::Device, opcode: u32) {
    if device.rsp.cpu.gpr[rs(opcode) as usize] as i32 >= 0 {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bltzal(device: &mut device::Device, opcode: u32) {
    if (device.rsp.cpu.gpr[rs(opcode) as usize] as i32) < 0 {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
    device.rsp.cpu.gpr[31] =
        (device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] + 8) & 0xFFF
}

pub fn bgezal(device: &mut device::Device, opcode: u32) {
    if (device.rsp.cpu.gpr[rs(opcode) as usize] as i32) >= 0 {
        device.rsp.cpu.branch_state.state = device::cpu::State::Take;
        device.rsp.cpu.branch_state.pc = device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize]
            .wrapping_add(imm(opcode << 2) as i16 as i32 as u32)
            + 4
    } else {
        device.rsp.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
    device.rsp.cpu.gpr[31] =
        (device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] + 8) & 0xFFF
}

pub fn mfc0(device: &mut device::Device, opcode: u32) {
    if rd(opcode) < device::rsp_interface::SP_REGS_COUNT {
        device.rsp.cpu.gpr[rt(opcode) as usize] = device::rsp_interface::read_regs(
            device,
            (rd(opcode) << 2) as u64,
            device::memory::AccessSize::Word,
        )
    } else {
        device.rsp.cpu.gpr[rt(opcode) as usize] = device::rdp::read_regs_dpc(
            device,
            ((rd(opcode) - device::rsp_interface::SP_REGS_COUNT) << 2) as u64,
            device::memory::AccessSize::Word,
        )
    }
    device.rsp.cpu.sync_point = true;
}

pub fn mtc0(device: &mut device::Device, opcode: u32) {
    if rd(opcode) < device::rsp_interface::SP_REGS_COUNT {
        device::rsp_interface::write_regs(
            device,
            (rd(opcode) << 2) as u64,
            device.rsp.cpu.gpr[rt(opcode) as usize],
            0xFFFFFFFF,
        )
    } else {
        device::rdp::write_regs_dpc(
            device,
            ((rd(opcode) - device::rsp_interface::SP_REGS_COUNT) << 2) as u64,
            device.rsp.cpu.gpr[rt(opcode) as usize],
            0xFFFFFFFF,
        )
    }
    if rd(opcode) == device::rsp_interface::SP_STATUS_REG
        && device.rsp.cpu.gpr[rt(opcode) as usize] & device::rsp_interface::SP_SET_HALT != 0
    {
        device.rsp.regs[device::rsp_interface::SP_STATUS_REG as usize] &=
            !device::rsp_interface::SP_STATUS_HALT; // set halt when event happens
        device.rsp.cpu.halted = true // the RSP can halt itself by setting SP_SET_HALT
    }
    device.rsp.cpu.sync_point = true;
}

pub fn mfc2(device: &mut device::Device, opcode: u32) {
    let hi = get_vpr_byte(device.rsp.cpu.vpr[rd(opcode) as usize], velement(opcode));
    let lo = get_vpr_byte(
        device.rsp.cpu.vpr[rd(opcode) as usize],
        velement(opcode) + 1,
    );
    device.rsp.cpu.gpr[rt(opcode) as usize] = ((hi as u16) << 8 | (lo as u16)) as i16 as i32 as u32
}

pub fn cfc2(device: &mut device::Device, opcode: u32) {
    let hi;
    let lo;
    let mut zero = unsafe { _mm_setzero_si128() };
    match rd(opcode) & 3 {
        0x00 => {
            hi = &mut device.rsp.cpu.vcoh;
            lo = &mut device.rsp.cpu.vcol;
        }
        0x01 => {
            hi = &mut device.rsp.cpu.vcch;
            lo = &mut device.rsp.cpu.vccl;
        }
        0x02 | 0x03 => {
            hi = &mut zero;
            lo = &mut device.rsp.cpu.vce;
        }
        _ => {
            panic!("unknown ctc2")
        }
    }

    unsafe {
        let reverse = _mm_set_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
        device.rsp.cpu.gpr[rt(opcode) as usize] =
            (_mm_movemask_epi8(_mm_shuffle_epi8(_mm_packs_epi16(*hi, *lo), reverse))) as i16 as u32;
    }
}

pub fn mtc2(device: &mut device::Device, opcode: u32) {
    modify_vpr_byte(
        &mut device.rsp.cpu.vpr[rd(opcode) as usize],
        (device.rsp.cpu.gpr[rt(opcode) as usize] >> 8) as u8,
        velement(opcode),
    );
    if velement(opcode) != 15 {
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[rd(opcode) as usize],
            device.rsp.cpu.gpr[rt(opcode) as usize] as u8,
            velement(opcode) + 1,
        );
    }
}

pub fn ctc2(device: &mut device::Device, opcode: u32) {
    let hi;
    let lo;
    let mut zero = unsafe { _mm_setzero_si128() };
    match rd(opcode) & 3 {
        0x00 => {
            hi = &mut device.rsp.cpu.vcoh;
            lo = &mut device.rsp.cpu.vcol;
        }
        0x01 => {
            hi = &mut device.rsp.cpu.vcch;
            lo = &mut device.rsp.cpu.vccl;
        }
        0x02 | 0x03 => {
            hi = &mut zero;
            lo = &mut device.rsp.cpu.vce;
        }
        _ => {
            panic!("unknown ctc2")
        }
    }

    unsafe {
        let mask = _mm_set_epi16(
            0x0101,
            0x0202,
            0x0404,
            0x0808,
            0x1010,
            0x2020,
            0x4040,
            0x8080u16 as i16,
        );
        *lo = _mm_cmpeq_epi8(
            _mm_and_si128(
                _mm_set1_epi8(!device.rsp.cpu.gpr[rt(opcode) as usize] as i8),
                mask,
            ),
            _mm_setzero_si128(),
        );
        *hi = std::arch::x86_64::_mm_cmpeq_epi8(
            _mm_and_si128(
                _mm_set1_epi8(!(device.rsp.cpu.gpr[rt(opcode) as usize] >> 8) as i8),
                mask,
            ),
            _mm_setzero_si128(),
        );
    }
}

pub fn lbv(device: &mut device::Device, opcode: u32) {
    let address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 0));

    let element = velement(opcode);
    modify_vpr_byte(
        &mut device.rsp.cpu.vpr[rt(opcode) as usize],
        device.rsp.mem[(address & 0xFFF) as usize],
        element,
    );
}

pub fn lsv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 1));

    let mut element = velement(opcode);
    let end = std::cmp::min(element + 2, 16);
    while element < end {
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            device.rsp.mem[(address & 0xFFF) as usize],
            element,
        );
        address += 1;
        element += 1;
    }
}

pub fn llv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 2));

    let mut element = velement(opcode);
    let end = std::cmp::min(element + 4, 16);
    while element < end {
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            device.rsp.mem[(address & 0xFFF) as usize],
            element,
        );
        address += 1;
        element += 1;
    }
}

pub fn ldv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 3));

    let mut element = velement(opcode);
    let end = std::cmp::min(element + 8, 16);
    while element < end {
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            device.rsp.mem[(address & 0xFFF) as usize],
            element,
        );
        address += 1;
        element += 1;
    }
}

pub fn lqv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let mut element = velement(opcode);
    let end = std::cmp::min(16 + element - ((address & 15) as u8), 16);
    while element < end {
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            device.rsp.mem[(address & 0xFFF) as usize],
            element,
        );
        address += 1;
        element += 1;
    }
}

pub fn lrv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let mut element = 16u8.wrapping_sub(((address & 15) as u8).wrapping_sub(velement(opcode)));
    address &= !15;
    while element < 16 {
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            device.rsp.mem[(address & 0xFFF) as usize],
            element,
        );
        address += 1;
        element += 1;
    }
}

pub fn lpv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 3));

    let index = ((address & 7) as u8).wrapping_sub(velement(opcode));
    address &= !7;
    let mut offset: u8 = 0;
    while offset < 8 {
        modify_vpr_element(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            (device.rsp.mem[((address.wrapping_add(((index.wrapping_add(offset)) & 15) as u32))
                & 0xFFF) as usize] as u16)
                << 8,
            offset,
        );
        offset += 1;
    }
}

pub fn luv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 3));

    let index = ((address & 7) as u8).wrapping_sub(velement(opcode));
    address &= !7;
    let mut offset: u8 = 0;
    while offset < 8 {
        modify_vpr_element(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            (device.rsp.mem[((address.wrapping_add(((index.wrapping_add(offset)) & 15) as u32))
                & 0xFFF) as usize] as u16)
                << 7,
            offset,
        );
        offset += 1;
    }
}

pub fn lhv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let index = ((address & 7) as u8).wrapping_sub(velement(opcode));
    address &= !7;
    let mut offset: u8 = 0;
    while offset < 8 {
        modify_vpr_element(
            &mut device.rsp.cpu.vpr[rt(opcode) as usize],
            (device.rsp.mem[((address.wrapping_add(((index.wrapping_add(offset * 2)) & 15) as u32))
                & 0xFFF) as usize] as u16)
                << 7,
            offset,
        );
        offset += 1;
    }
}

pub fn lfv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let index = ((address & 7) as u8).wrapping_sub(velement(opcode));
    address &= !7;
    let start = velement(opcode);
    let end = std::cmp::min(start + 8, 16);
    let mut tmp: u128 = 0;
    let mut offset: u8 = 0;
    while offset < 4 {
        modify_vpr_element(
            &mut tmp,
            (device.rsp.mem[((address.wrapping_add(((index.wrapping_add(offset * 4)) & 15) as u32))
                & 0xFFF) as usize] as u16)
                << 7,
            offset,
        );
        modify_vpr_element(
            &mut tmp,
            (device.rsp.mem[((address
                .wrapping_add(((index.wrapping_add(offset * 4).wrapping_add(8)) & 15) as u32))
                & 0xFFF) as usize] as u16)
                << 7,
            offset + 4,
        );
        offset += 1;
    }
    offset = start;
    while offset < end {
        let value = get_vpr_byte(tmp, offset);
        modify_vpr_byte(&mut device.rsp.cpu.vpr[rt(opcode) as usize], value, offset);
        offset += 1;
    }
}

pub fn lwv(_device: &mut device::Device, _opcode: u32) {}

pub fn ltv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let begin = address & !7;
    address = begin + (((velement(opcode)) as u32 + (address & 8)) & 15);
    let vtbase = rt(opcode) & !7;
    let mut vtoff = (velement(opcode)) as u32 >> 1;
    let mut i = 0;
    while i < 8 {
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[(vtbase + vtoff) as usize],
            device.rsp.mem[(address & 0xFFF) as usize],
            i * 2,
        );
        address += 1;
        if address == begin + 16 {
            address = begin
        }
        modify_vpr_byte(
            &mut device.rsp.cpu.vpr[(vtbase + vtoff) as usize],
            device.rsp.mem[(address & 0xFFF) as usize],
            i * 2 + 1,
        );
        address += 1;
        if address == begin + 16 {
            address = begin
        }
        vtoff = (vtoff + 1) & 7;
        i += 1;
    }
}

pub fn sbv(device: &mut device::Device, opcode: u32) {
    let address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 0));

    device.rsp.mem[(address & 0xFFF) as usize] =
        get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], velement(opcode))
}

pub fn ssv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 1));

    let mut element = velement(opcode);
    let end = element + 2;
    while element < end {
        device.rsp.mem[(address & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], element);
        address += 1;
        element += 1;
    }
}

pub fn slv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 2));

    let mut element = velement(opcode);
    let end = element + 4;
    while element < end {
        device.rsp.mem[(address & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], element);
        address += 1;
        element += 1;
    }
}

pub fn sdv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 3));

    let mut element = velement(opcode);
    let end = element + 8;
    while element < end {
        device.rsp.mem[(address & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], element);
        address += 1;
        element += 1;
    }
}

pub fn sqv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let mut element = velement(opcode);
    let end = element + (16 - (address & 15)) as u8;
    while element < end {
        device.rsp.mem[(address & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], element);
        address += 1;
        element += 1;
    }
}

pub fn srv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let mut element = velement(opcode);
    let end = element + (address & 15) as u8;
    let base = (16 - (address & 15)) as u8;
    address &= !15;
    while element < end {
        device.rsp.mem[(address & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], element + base);
        address += 1;
        element += 1;
    }
}

pub fn spv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 3));

    let mut element = velement(opcode);
    let end = element + 8;
    while element < end {
        if (element & 15) < 8 {
            device.rsp.mem[(address & 0xFFF) as usize] =
                get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], (element & 7) << 1);
        } else {
            device.rsp.mem[(address & 0xFFF) as usize] =
                (get_vpr_element(device.rsp.cpu.vpr[rt(opcode) as usize], element) >> 7) as u8;
        }
        address += 1;
        element += 1;
    }
}

pub fn suv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 3));

    let mut element = velement(opcode);
    let end = element + 8;
    while element < end {
        if (element & 15) < 8 {
            device.rsp.mem[(address & 0xFFF) as usize] =
                (get_vpr_element(device.rsp.cpu.vpr[rt(opcode) as usize], element) >> 7) as u8;
        } else {
            device.rsp.mem[(address & 0xFFF) as usize] =
                get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], (element & 7) << 1);
        }
        address += 1;
        element += 1;
    }
}

pub fn shv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let element = velement(opcode);
    let index = (address & 7) as u8;
    address &= !7;
    let mut offset = 0;
    while offset < 8 {
        let byte_val = element + offset * 2;
        let value = get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], byte_val) << 1
            | get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], byte_val + 1) >> 7;
        device.rsp.mem[((address + ((index + offset * 2) & 15) as u32) & 0xFFF) as usize] = value;
        offset += 1;
    }
}

pub fn sfv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));
    let base = address & 7;
    address &= !7;
    let element = velement(opcode);
    let elements;
    match element {
        0 | 15 => {
            elements = [0, 1, 2, 3];
        }
        1 => {
            elements = [6, 7, 4, 5];
        }
        4 => {
            elements = [1, 2, 3, 0];
        }
        5 => {
            elements = [7, 4, 5, 6];
        }
        8 => {
            elements = [4, 5, 6, 7];
        }
        11 => {
            elements = [3, 0, 1, 2];
        }
        12 => {
            elements = [5, 6, 7, 4];
        }
        _ => {
            device.rsp.mem[((address + ((base + 0) & 15)) & 0xFFF) as usize] = 0;
            device.rsp.mem[((address + ((base + 4) & 15)) & 0xFFF) as usize] = 0;
            device.rsp.mem[((address + ((base + 8) & 15)) & 0xFFF) as usize] = 0;
            device.rsp.mem[((address + ((base + 12) & 15)) & 0xFFF) as usize] = 0;
            return;
        }
    }
    let mut offset = 0;
    let mut i = 0;
    while i < 4 {
        device.rsp.mem[((address + ((base + offset) & 15)) & 0xFFF) as usize] =
            (get_vpr_element(device.rsp.cpu.vpr[rt(opcode) as usize], elements[i]) >> 7) as u8;
        offset += 4;
        i += 1;
    }
}

pub fn swv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let mut element = velement(opcode);
    let end = element + 16;
    let mut base = address & 7;
    address &= !7;
    while element < end {
        device.rsp.mem[((address + (base & 15)) & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[rt(opcode) as usize], element);
        base += 1;
        element += 1;
    }
}

pub fn stv(device: &mut device::Device, opcode: u32) {
    let mut address = device.rsp.cpu.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));
    let start = rt(opcode) & !7;
    let end = start + 8;
    let mut element = 16 - (velement(opcode) & !1);
    let mut base = (address & 7).wrapping_sub((velement(opcode) & !1) as u32);
    address &= !7;
    let mut offset = start;
    while offset < end {
        device.rsp.mem[((address + (base & 15)) & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[offset as usize], element);
        base = base.wrapping_add(1);
        element += 1;
        device.rsp.mem[((address + (base & 15)) & 0xFFF) as usize] =
            get_vpr_byte(device.rsp.cpu.vpr[offset as usize], element);
        base = base.wrapping_add(1);
        element += 1;
        offset += 1;
    }
}

pub fn reserved(_device: &mut device::Device, _opcode: u32) {
    panic!("rsp su reserved")
}
