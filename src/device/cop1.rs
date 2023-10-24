use crate::device;

//pub const FCR31_FLAG_INEXACT_BIT: u32 = 1 << 2;
//pub const FCR31_FLAG_UNDERFLOW_BIT: u32 = 1 << 3;
//pub const FCR31_FLAG_OVERFLOW_BIT: u32 = 1 << 4;
//pub const FCR31_FLAG_DIVBYZERO_BIT: u32 = 1 << 5;
//pub const FCR31_FLAG_INVALID_BIT: u32 = 1 << 6;
//pub const FCR31_ENABLE_INEXACT_BIT: u32 = 1 << 7;
//pub const FCR31_ENABLE_UNDERFLOW_BIT: u32 = 1 << 8;
//pub const FCR31_ENABLE_OVERFLOW_BIT: u32 = 1 << 9;
//pub const FCR31_ENABLE_DIVBYZERO_BIT: u32 = 1 << 10;
//pub const FCR31_ENABLE_INVALID_BIT: u32 = 1 << 11;
//pub const FCR31_CAUSE_INEXACT_BIT: u32 = 1 << 12;
//pub const FCR31_CAUSE_UNDERFLOW_BIT: u32 = 1 << 13;
//pub const FCR31_CAUSE_OVERFLOW_BIT: u32 = 1 << 14;
//pub const FCR31_CAUSE_DIVBYZERO_BIT: u32 = 1 << 15;
//pub const FCR31_CAUSE_INVALID_BIT: u32 = 1 << 16;
pub const FCR31_CAUSE_UNIMP_BIT: u32 = 1 << 17;
pub const FCR31_CMP_BIT: u32 = 1 << 23;
pub const FCR31_FS_BIT: u32 = 1 << 24;

pub const FCR31_CAUSE_MASK: u32 = 0b00000000000000111111000000000000;
pub const FCR31_ENABLE_MASK: u32 = 0b00000000000000000000111110000000;
pub const FCR31_WRITE_MASK: u32 = 0b00000001100000111111111111111111;

pub struct Cop1 {
    pub fcr0: u32,
    pub fcr31: u32,
    pub flush_mode: u32,
    pub fgr32: [[u8; 4]; 32],
    pub fgr64: [[u8; 8]; 32],
    pub instrs: [fn(&mut device::Device, u32); 32],
    pub b_instrs: [fn(&mut device::Device, u32); 4],
    pub s_instrs: [fn(&mut device::Device, u32); 64],
    pub d_instrs: [fn(&mut device::Device, u32); 64],
    pub w_instrs: [fn(&mut device::Device, u32); 64],
    pub l_instrs: [fn(&mut device::Device, u32); 64],
}

pub fn lwc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[device::cpu_instructions::rs(opcode) as usize].wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16),
        ),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }
    let value = device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Word,
        cached,
    )
    .to_ne_bytes();
    set_fpr_single(
        device,
        device::fpu_instructions::ft(opcode) as usize,
        f32::from_ne_bytes(value),
        false,
    )
}

pub fn ldc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[device::cpu_instructions::rs(opcode) as usize].wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16),
        ),
        device::memory::AccessType::Read,
    );
    if err {
        return;
    }

    let mut w = [2; 32];
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
    let value = ((w[0] as u64) << 32) | (w[1]) as u64;
    set_fpr_double(
        device,
        device::fpu_instructions::ft(opcode) as usize,
        f64::from_ne_bytes(value.to_ne_bytes()),
    )
}

pub fn swc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[device::cpu_instructions::rs(opcode) as usize].wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16),
        ),
        device::memory::AccessType::Write,
    );
    if err {
        return;
    }
    let value = get_fpr_single(device, device::fpu_instructions::ft(opcode) as usize).to_ne_bytes();
    device::memory::data_write(
        device,
        phys_address,
        u32::from_ne_bytes(value),
        0xFFFFFFFF,
        cached,
    );
}

pub fn sdc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let (phys_address, cached, err) = device::memory::translate_address(
        device,
        device.cpu.gpr[device::cpu_instructions::rs(opcode) as usize].wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16),
        ),
        device::memory::AccessType::Write,
    );
    if err {
        return;
    }

    let value = get_fpr_double(device, device::fpu_instructions::ft(opcode) as usize).to_ne_bytes();

    device::memory::data_write(
        device,
        phys_address,
        u32::from_ne_bytes(value[4..8].try_into().unwrap()),
        0xFFFFFFFF,
        cached,
    );
    device::memory::data_write(
        device,
        phys_address + 4,
        u32::from_ne_bytes(value[0..4].try_into().unwrap()),
        0xFFFFFFFF,
        cached,
    )
}

pub fn mfc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let value = get_fpr_single(device, device::fpu_instructions::fs(opcode) as usize);
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        device::cpu_instructions::se32(u32::from_ne_bytes(value.to_ne_bytes()) as i32);
}

pub fn dmfc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let value = get_fpr_double(device, device::fpu_instructions::fs(opcode) as usize);
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        u64::from_ne_bytes(value.to_ne_bytes());
}

pub fn cfc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] = device::cpu_instructions::se32(
        get_control_registers_fpu(device, device::fpu_instructions::fs(opcode)) as i32,
    )
}

pub fn dcfc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop1.fcr31 &= !FCR31_CAUSE_MASK;
    device.cpu.cop1.fcr31 |= FCR31_CAUSE_UNIMP_BIT;
    device::exceptions::floating_point_exception(device)
}

pub fn mtc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let value = f32::from_ne_bytes(
        (device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] as u32).to_ne_bytes(),
    );
    set_fpr_single(
        device,
        device::fpu_instructions::fs(opcode) as usize,
        value,
        false,
    )
}

pub fn dmtc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    let value = f64::from_ne_bytes(
        device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize].to_ne_bytes(),
    );
    set_fpr_double(device, device::fpu_instructions::fs(opcode) as usize, value)
}

pub fn ctc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    set_control_registers_fpu(
        device,
        device::fpu_instructions::fs(opcode),
        device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] as u32,
    )
}

pub fn dctc1(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop1.fcr31 &= !FCR31_CAUSE_MASK;
    device.cpu.cop1.fcr31 |= FCR31_CAUSE_UNIMP_BIT;
    device::exceptions::floating_point_exception(device)
}

pub fn execute_cop1_b(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop1.b_instrs[((opcode >> 16) & 0x3) as usize](device, opcode)
}

pub fn execute_cop1_s(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop1.s_instrs[(opcode & 0x3F) as usize](device, opcode)
}

pub fn execute_cop1_d(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop1.d_instrs[(opcode & 0x3F) as usize](device, opcode)
}

pub fn execute_cop1_l(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop1.l_instrs[(opcode & 0x3F) as usize](device, opcode)
}

pub fn execute_cop1_w(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop1.w_instrs[(opcode & 0x3F) as usize](device, opcode)
}

pub fn unusable(device: &mut device::Device, _opcode: u32) {
    device::exceptions::cop_unusable_exception(device, device::cop0::COP0_CAUSE_CE1)
}

pub fn reserved(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU1
        == 0
    {
        return unusable(device, opcode);
    }

    device::exceptions::reserved_exception(device, device::cop0::COP0_CAUSE_CE1);
}

pub fn get_control_registers_fpu(device: &device::Device, index: u32) -> u32 {
    match index {
        0 => device.cpu.cop1.fcr0,
        31 => device.cpu.cop1.fcr31,
        _ => {
            panic!("unknown FCR register")
        }
    }
}

pub fn set_control_registers_fpu(device: &mut device::Device, index: u32, data: u32) {
    match index {
        0 => { // read only
        }

        31 => {
            device.cpu.cop1.fcr31 = data & FCR31_WRITE_MASK;
            // the Cause bits are ANDed with the Enable bits to check for exceptions
            // "Unimplemented Operation" has no Enable bit and always causes an exception
            if (device.cpu.cop1.fcr31 & FCR31_CAUSE_MASK) >> 5
                & (device.cpu.cop1.fcr31 & FCR31_ENABLE_MASK)
                != 0
                || device.cpu.cop1.fcr31 & FCR31_CAUSE_UNIMP_BIT != 0
            {
                device::exceptions::floating_point_exception(device)
            }

            unsafe {
                let flush_mode;
                if (device.cpu.cop1.fcr31 & 2) != 0 {
                    if (device.cpu.cop1.fcr31 & FCR31_FS_BIT) != 0 {
                        flush_mode = std::arch::x86_64::_MM_FLUSH_ZERO_OFF
                    } else {
                        flush_mode = std::arch::x86_64::_MM_FLUSH_ZERO_ON;
                    }
                } else {
                    flush_mode = std::arch::x86_64::_MM_FLUSH_ZERO_ON;
                }
                if flush_mode != device.cpu.cop1.flush_mode {
                    #[allow(deprecated)]
                    std::arch::x86_64::_MM_SET_FLUSH_ZERO_MODE(flush_mode);
                    device.cpu.cop1.flush_mode = flush_mode;
                }
            }
        }
        _ => {
            panic!("unknown FCR register")
        }
    }
}

pub fn set_fpr_single(device: &mut device::Device, index: usize, value: f32, clear_high: bool) {
    let bytes = value.to_ne_bytes();
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_FR
        == 0
    {
        device.cpu.cop1.fgr32[index] = bytes;
    } else {
        let bytes_lo = bytes;
        let bytes_hi: [u8; 4] = if clear_high {
            [0, 0, 0, 0]
        } else {
            device.cpu.cop1.fgr64[index][4..8].try_into().unwrap()
        };
        device.cpu.cop1.fgr64[index] = [bytes_lo, bytes_hi].concat().try_into().unwrap();
    }
}

pub fn get_fpr_single(device: &device::Device, index: usize) -> f32 {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_FR
        == 0
    {
        f32::from_ne_bytes(device.cpu.cop1.fgr32[index])
    } else {
        f32::from_ne_bytes(device.cpu.cop1.fgr64[index][0..4].try_into().unwrap())
    }
}

pub fn set_fpr_double(device: &mut device::Device, index: usize, value: f64) {
    let bytes = value.to_ne_bytes();
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_FR
        == 0
    {
        device.cpu.cop1.fgr32[index & !1] = bytes[0..4].try_into().unwrap();
        device.cpu.cop1.fgr32[(index & !1) + 1] = bytes[4..8].try_into().unwrap();
    } else {
        device.cpu.cop1.fgr64[index] = bytes;
    }
}

pub fn get_fpr_double(device: &device::Device, index: usize) -> f64 {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_FR
        == 0
    {
        let bytes_lo = device.cpu.cop1.fgr32[index & !1];
        let bytes_hi = device.cpu.cop1.fgr32[(index & !1) + 1];
        f64::from_ne_bytes([bytes_lo, bytes_hi].concat().try_into().unwrap())
    } else {
        f64::from_ne_bytes(device.cpu.cop1.fgr64[index])
    }
}

pub fn init(device: &mut device::Device) {
    set_fgr_registers(device, 0);
    device.cpu.cop1.fcr0 = 0b101000000000;

    device.cpu.cop1.b_instrs = [
        device::fpu_instructions::bc1f,  // 0
        device::fpu_instructions::bc1t,  // 1
        device::fpu_instructions::bc1fl, // 2
        device::fpu_instructions::bc1tl, // 3
    ];

    device.cpu.cop1.s_instrs = [
        device::fpu_instructions::add_s,     // 0
        device::fpu_instructions::sub_s,     // 1
        device::fpu_instructions::mul_s,     // 2
        device::fpu_instructions::div_s,     // 3
        device::fpu_instructions::sqrt_s,    // 4
        device::fpu_instructions::abs_s,     // 5
        device::fpu_instructions::mov_s,     // 6
        device::fpu_instructions::neg_s,     // 7
        device::fpu_instructions::round_l_s, // 8
        device::fpu_instructions::trunc_l_s, // 9
        device::fpu_instructions::ceil_l_s,  // 10
        device::fpu_instructions::floor_l_s, // 11
        device::fpu_instructions::round_w_s, // 12
        device::fpu_instructions::trunc_w_s, // 13
        device::fpu_instructions::ceil_w_s,  // 14
        device::fpu_instructions::floor_w_s, // 15
        device::cop1::reserved,              // 16
        device::cop1::reserved,              // 17
        device::cop1::reserved,              // 18
        device::cop1::reserved,              // 19
        device::cop1::reserved,              // 20
        device::cop1::reserved,              // 21
        device::cop1::reserved,              // 22
        device::cop1::reserved,              // 23
        device::cop1::reserved,              // 24
        device::cop1::reserved,              // 25
        device::cop1::reserved,              // 26
        device::cop1::reserved,              // 27
        device::cop1::reserved,              // 28
        device::cop1::reserved,              // 29
        device::cop1::reserved,              // 30
        device::cop1::reserved,              // 31
        device::cop1::reserved,              // 32
        device::fpu_instructions::cvt_d_s,   // 33
        device::cop1::reserved,              // 34
        device::cop1::reserved,              // 35
        device::fpu_instructions::cvt_w_s,   // 36
        device::fpu_instructions::cvt_l_s,   // 37
        device::cop1::reserved,              // 38
        device::cop1::reserved,              // 38
        device::cop1::reserved,              // 40
        device::cop1::reserved,              // 41
        device::cop1::reserved,              // 42
        device::cop1::reserved,              // 43
        device::cop1::reserved,              // 44
        device::cop1::reserved,              // 45
        device::cop1::reserved,              // 46
        device::cop1::reserved,              // 47
        device::fpu_instructions::c_f_s,     // 48
        device::fpu_instructions::c_un_s,    // 49
        device::fpu_instructions::c_eq_s,    // 50
        device::fpu_instructions::c_ueq_s,   // 51
        device::fpu_instructions::c_olt_s,   // 52
        device::fpu_instructions::c_ult_s,   // 53
        device::fpu_instructions::c_ole_s,   // 54
        device::fpu_instructions::c_ule_s,   // 55
        device::fpu_instructions::c_sf_s,    // 56
        device::fpu_instructions::c_ngle_s,  // 57
        device::fpu_instructions::c_seq_s,   // 58
        device::fpu_instructions::c_ngl_s,   // 59
        device::fpu_instructions::c_lt_s,    // 60
        device::fpu_instructions::c_nge_s,   // 61
        device::fpu_instructions::c_le_s,    // 62
        device::fpu_instructions::c_ngt_s,   // 63
    ];

    device.cpu.cop1.d_instrs = [
        device::fpu_instructions::add_d,     // 0
        device::fpu_instructions::sub_d,     // 1
        device::fpu_instructions::mul_d,     // 2
        device::fpu_instructions::div_d,     // 3
        device::fpu_instructions::sqrt_d,    // 4
        device::fpu_instructions::abs_d,     // 5
        device::fpu_instructions::mov_d,     // 6
        device::fpu_instructions::neg_d,     // 7
        device::fpu_instructions::round_l_d, // 8
        device::fpu_instructions::trunc_l_d, // 9
        device::fpu_instructions::ceil_l_d,  // 10
        device::fpu_instructions::floor_l_d, // 11
        device::fpu_instructions::round_w_d, // 12
        device::fpu_instructions::trunc_w_d, // 13
        device::fpu_instructions::ceil_w_d,  // 14
        device::fpu_instructions::floor_w_d, // 15
        device::cop1::reserved,              // 16
        device::cop1::reserved,              // 17
        device::cop1::reserved,              // 18
        device::cop1::reserved,              // 19
        device::cop1::reserved,              // 20
        device::cop1::reserved,              // 21
        device::cop1::reserved,              // 22
        device::cop1::reserved,              // 23
        device::cop1::reserved,              // 24
        device::cop1::reserved,              // 25
        device::cop1::reserved,              // 26
        device::cop1::reserved,              // 27
        device::cop1::reserved,              // 28
        device::cop1::reserved,              // 29
        device::cop1::reserved,              // 30
        device::cop1::reserved,              // 31
        device::fpu_instructions::cvt_s_d,   // 32
        device::cop1::reserved,              // 33
        device::cop1::reserved,              // 34
        device::cop1::reserved,              // 35
        device::fpu_instructions::cvt_w_d,   // 36
        device::fpu_instructions::cvt_l_d,   // 37
        device::cop1::reserved,              // 38
        device::cop1::reserved,              // 38
        device::cop1::reserved,              // 40
        device::cop1::reserved,              // 41
        device::cop1::reserved,              // 42
        device::cop1::reserved,              // 43
        device::cop1::reserved,              // 44
        device::cop1::reserved,              // 45
        device::cop1::reserved,              // 46
        device::cop1::reserved,              // 47
        device::fpu_instructions::c_f_d,     // 48
        device::fpu_instructions::c_un_d,    // 49
        device::fpu_instructions::c_eq_d,    // 50
        device::fpu_instructions::c_ueq_d,   // 51
        device::fpu_instructions::c_olt_d,   // 52
        device::fpu_instructions::c_ult_d,   // 53
        device::fpu_instructions::c_ole_d,   // 54
        device::fpu_instructions::c_ule_d,   // 55
        device::fpu_instructions::c_sf_d,    // 56
        device::fpu_instructions::c_ngle_d,  // 57
        device::fpu_instructions::c_seq_d,   // 58
        device::fpu_instructions::c_ngl_d,   // 59
        device::fpu_instructions::c_lt_d,    // 60
        device::fpu_instructions::c_nge_d,   // 61
        device::fpu_instructions::c_le_d,    // 62
        device::fpu_instructions::c_ngt_d,   // 63
    ];

    device.cpu.cop1.l_instrs = [
        device::cop1::reserved,            // 0
        device::cop1::reserved,            // 1
        device::cop1::reserved,            // 2
        device::cop1::reserved,            // 3
        device::cop1::reserved,            // 4
        device::cop1::reserved,            // 5
        device::cop1::reserved,            // 6
        device::cop1::reserved,            // 7
        device::cop1::reserved,            // 8
        device::cop1::reserved,            // 9
        device::cop1::reserved,            // 10
        device::cop1::reserved,            // 11
        device::cop1::reserved,            // 12
        device::cop1::reserved,            // 13
        device::cop1::reserved,            // 14
        device::cop1::reserved,            // 15
        device::cop1::reserved,            // 16
        device::cop1::reserved,            // 17
        device::cop1::reserved,            // 18
        device::cop1::reserved,            // 19
        device::cop1::reserved,            // 20
        device::cop1::reserved,            // 21
        device::cop1::reserved,            // 22
        device::cop1::reserved,            // 23
        device::cop1::reserved,            // 24
        device::cop1::reserved,            // 25
        device::cop1::reserved,            // 26
        device::cop1::reserved,            // 27
        device::cop1::reserved,            // 28
        device::cop1::reserved,            // 29
        device::cop1::reserved,            // 30
        device::cop1::reserved,            // 31
        device::fpu_instructions::cvt_s_l, // 32
        device::fpu_instructions::cvt_d_l, // 33
        device::cop1::reserved,            // 34
        device::cop1::reserved,            // 35
        device::cop1::reserved,            // 36
        device::cop1::reserved,            // 37
        device::cop1::reserved,            // 38
        device::cop1::reserved,            // 39
        device::cop1::reserved,            // 40
        device::cop1::reserved,            // 41
        device::cop1::reserved,            // 42
        device::cop1::reserved,            // 43
        device::cop1::reserved,            // 44
        device::cop1::reserved,            // 45
        device::cop1::reserved,            // 46
        device::cop1::reserved,            // 47
        device::cop1::reserved,            // 48
        device::cop1::reserved,            // 49
        device::cop1::reserved,            // 50
        device::cop1::reserved,            // 51
        device::cop1::reserved,            // 52
        device::cop1::reserved,            // 53
        device::cop1::reserved,            // 54
        device::cop1::reserved,            // 55
        device::cop1::reserved,            // 56
        device::cop1::reserved,            // 57
        device::cop1::reserved,            // 58
        device::cop1::reserved,            // 59
        device::cop1::reserved,            // 60
        device::cop1::reserved,            // 61
        device::cop1::reserved,            // 62
        device::cop1::reserved,            // 63
    ];

    device.cpu.cop1.w_instrs = [
        device::cop1::reserved,            // 0
        device::cop1::reserved,            // 1
        device::cop1::reserved,            // 2
        device::cop1::reserved,            // 3
        device::cop1::reserved,            // 4
        device::cop1::reserved,            // 5
        device::cop1::reserved,            // 6
        device::cop1::reserved,            // 7
        device::cop1::reserved,            // 8
        device::cop1::reserved,            // 9
        device::cop1::reserved,            // 10
        device::cop1::reserved,            // 11
        device::cop1::reserved,            // 12
        device::cop1::reserved,            // 13
        device::cop1::reserved,            // 14
        device::cop1::reserved,            // 15
        device::cop1::reserved,            // 16
        device::cop1::reserved,            // 17
        device::cop1::reserved,            // 18
        device::cop1::reserved,            // 19
        device::cop1::reserved,            // 20
        device::cop1::reserved,            // 21
        device::cop1::reserved,            // 22
        device::cop1::reserved,            // 23
        device::cop1::reserved,            // 24
        device::cop1::reserved,            // 25
        device::cop1::reserved,            // 26
        device::cop1::reserved,            // 27
        device::cop1::reserved,            // 28
        device::cop1::reserved,            // 29
        device::cop1::reserved,            // 30
        device::cop1::reserved,            // 31
        device::fpu_instructions::cvt_s_w, // 32
        device::fpu_instructions::cvt_d_w, // 33
        device::cop1::reserved,            // 34
        device::cop1::reserved,            // 35
        device::cop1::reserved,            // 36
        device::cop1::reserved,            // 37
        device::cop1::reserved,            // 38
        device::cop1::reserved,            // 39
        device::cop1::reserved,            // 40
        device::cop1::reserved,            // 41
        device::cop1::reserved,            // 42
        device::cop1::reserved,            // 43
        device::cop1::reserved,            // 44
        device::cop1::reserved,            // 45
        device::cop1::reserved,            // 46
        device::cop1::reserved,            // 47
        device::cop1::reserved,            // 48
        device::cop1::reserved,            // 49
        device::cop1::reserved,            // 50
        device::cop1::reserved,            // 51
        device::cop1::reserved,            // 52
        device::cop1::reserved,            // 53
        device::cop1::reserved,            // 54
        device::cop1::reserved,            // 55
        device::cop1::reserved,            // 56
        device::cop1::reserved,            // 57
        device::cop1::reserved,            // 58
        device::cop1::reserved,            // 59
        device::cop1::reserved,            // 60
        device::cop1::reserved,            // 61
        device::cop1::reserved,            // 62
        device::cop1::reserved,            // 63
    ];

    device.cpu.cop1.instrs = [
        device::cop1::mfc1,           // 0
        device::cop1::dmfc1,          // 1
        device::cop1::cfc1,           // 2
        device::cop1::dcfc1,          // 3
        device::cop1::mtc1,           // 4
        device::cop1::dmtc1,          // 5
        device::cop1::ctc1,           // 6
        device::cop1::dctc1,          // 7
        device::cop1::execute_cop1_b, // 8
        device::cop1::reserved,       // 9
        device::cop1::reserved,       // 10
        device::cop1::reserved,       // 11
        device::cop1::reserved,       // 12
        device::cop1::reserved,       // 13
        device::cop1::reserved,       // 14
        device::cop1::reserved,       // 15
        device::cop1::execute_cop1_s, // 16
        device::cop1::execute_cop1_d, // 17
        device::cop1::reserved,       // 18
        device::cop1::reserved,       // 19
        device::cop1::execute_cop1_w, // 20
        device::cop1::execute_cop1_l, // 21
        device::cop1::reserved,       // 22
        device::cop1::reserved,       // 23
        device::cop1::reserved,       // 24
        device::cop1::reserved,       // 25
        device::cop1::reserved,       // 26
        device::cop1::reserved,       // 27
        device::cop1::reserved,       // 28
        device::cop1::reserved,       // 29
        device::cop1::reserved,       // 30
        device::cop1::reserved,       // 31
    ]
}

pub fn set_fgr_registers(device: &mut device::Device, status_reg: u64) {
    // this method doesn't account for undocumented behaviour (accessing odd numbered registers in half mode)
    if (status_reg & device::cop0::COP0_STATUS_FR) == 0 {
        let mut i = 0;
        while i < 32 {
            let bytes = device.cpu.cop1.fgr64[i];
            device.cpu.cop1.fgr32[i] = bytes[0..4].try_into().unwrap();
            device.cpu.cop1.fgr32[i + 1] = bytes[4..8].try_into().unwrap();
            i += 2;
        }
    } else {
        let mut i = 0;
        while i < 32 {
            let bytes_lo = device.cpu.cop1.fgr32[i];
            let bytes_hi = device.cpu.cop1.fgr32[i + 1];
            device.cpu.cop1.fgr64[i] = [bytes_lo, bytes_hi].concat().try_into().unwrap();
            i += 2;
        }
    }
}
