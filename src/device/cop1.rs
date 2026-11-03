use crate::{device, savestates};

const FCR31_CAUSE_UNIMP_BIT: u32 = 1 << 17;
pub const FCR31_CMP_BIT: u32 = 1 << 23;
const FCR31_FS_BIT: u32 = 1 << 24;

const FCR31_CAUSE_MASK: u32 = 0b00000000000000111111000000000000;
const FCR31_ENABLE_MASK: u32 = 0b00000000000000000000111110000000;
const FCR31_WRITE_MASK: u32 = 0b00000001100000111111111111111111;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Cop1 {
    pub fcr0: u32,
    pub fcr31: u32,
    // The FPU always has 32 64-bit registers, Status.FR only changes how they are indexed
    pub fgr: [u64; 32],
    #[serde(skip, default = "savestates::default_instructions")]
    pub instrs: [fn(&mut device::Device, u32); 32],
    #[serde(skip, default = "savestates::default_instructions")]
    pub b_instrs: [fn(&mut device::Device, u32); 32],
    #[serde(skip, default = "savestates::default_instructions")]
    pub s_instrs: [fn(&mut device::Device, u32); 64],
    #[serde(skip, default = "savestates::default_instructions")]
    pub d_instrs: [fn(&mut device::Device, u32); 64],
    #[serde(skip, default = "savestates::default_instructions")]
    pub w_instrs: [fn(&mut device::Device, u32); 64],
    #[serde(skip, default = "savestates::default_instructions")]
    pub l_instrs: [fn(&mut device::Device, u32); 64],
}

fn cu1_enabled(device: &device::Device) -> bool {
    device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG] & device::cop0::COP0_STATUS_CU1 != 0
}

pub fn lwc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
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
    );
    set_fpr_word(device, device::fpu_instructions::ft(opcode), value)
}

pub fn ldc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
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

    let hi = device::memory::data_read(
        device,
        phys_address,
        device::memory::AccessSize::Dword,
        cached,
    );
    let lo = device::memory::data_read(
        device,
        phys_address + 4,
        device::memory::AccessSize::Dword,
        cached,
    );
    set_fpr_dword(
        device,
        device::fpu_instructions::ft(opcode),
        ((hi as u64) << 32) | lo as u64,
    )
}

pub fn swc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
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
    let value = get_fpr_word(device, device::fpu_instructions::ft(opcode));
    device::memory::data_write(device, phys_address, value, 0xFFFFFFFF, cached);
}

pub fn sdc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
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

    let value = get_fpr_dword(device, device::fpu_instructions::ft(opcode));
    device::memory::data_write(
        device,
        phys_address,
        (value >> 32) as u32,
        0xFFFFFFFF,
        cached,
    );
    device::memory::data_write(device, phys_address + 4, value as u32, 0xFFFFFFFF, cached)
}

fn mfc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    let value = get_fpr_word(device, device::fpu_instructions::fs(opcode));
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        device::cpu_instructions::se32(value as i32);
}

fn dmfc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        get_fpr_dword(device, device::fpu_instructions::fs(opcode));
}

fn cfc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] = device::cpu_instructions::se32(
        get_control_registers_fpu(device, device::fpu_instructions::fs(opcode)) as i32,
    )
}

fn mtc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    let value = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] as u32;
    set_fpr_word(device, device::fpu_instructions::fs(opcode), value)
}

fn dmtc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    let value = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize];
    set_fpr_dword(device, device::fpu_instructions::fs(opcode), value)
}

fn ctc1(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    set_control_registers_fpu(
        device,
        device::fpu_instructions::fs(opcode),
        device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] as u32,
    )
}

fn execute_cop1_b(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.cop1.b_instrs[((opcode >> 16) & 0x1F) as usize](device, opcode)
}

fn execute_cop1_s(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.cop1.s_instrs[(opcode & 0x3F) as usize](device, opcode)
}

fn execute_cop1_d(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.cop1.d_instrs[(opcode & 0x3F) as usize](device, opcode)
}

fn execute_cop1_l(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.cop1.l_instrs[(opcode & 0x3F) as usize](device, opcode)
}

fn execute_cop1_w(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.cop1.w_instrs[(opcode & 0x3F) as usize](device, opcode)
}

fn unusable(device: &mut device::Device, _opcode: u32) {
    device::exceptions::cop_unusable_exception(device, device::cop0::COP0_CAUSE_CE1)
}

// Reserved COP1 encodings (and DCFC1/DCTC1) raise an Unimplemented Operation exception
pub fn unimplemented(device: &mut device::Device, opcode: u32) {
    if !cu1_enabled(device) {
        return unusable(device, opcode);
    }

    device.cpu.cop1.fcr31 &= !FCR31_CAUSE_MASK;
    device.cpu.cop1.fcr31 |= FCR31_CAUSE_UNIMP_BIT;
    device::exceptions::floating_point_exception(device)
}

fn get_control_registers_fpu(device: &device::Device, index: u32) -> u32 {
    match index {
        0 => device.cpu.cop1.fcr0,
        31 => device.cpu.cop1.fcr31,
        _ => {
            panic!("unknown FCR register")
        }
    }
}

fn set_control_registers_fpu(device: &mut device::Device, index: u32, data: u32) {
    match index {
        0 => { // read only
        }

        31 => {
            let old_fcr31 = device.cpu.cop1.fcr31;
            device.cpu.cop1.fcr31 = data & FCR31_WRITE_MASK;
            // the Cause bits are ANDed with the Enable bits to check for exceptions
            // "Unimplemented Operation" has no Enable bit and always causes an exception
            if ((device.cpu.cop1.fcr31 & FCR31_CAUSE_MASK) >> 5)
                & (device.cpu.cop1.fcr31 & FCR31_ENABLE_MASK)
                != 0
                || device.cpu.cop1.fcr31 & FCR31_CAUSE_UNIMP_BIT != 0
            {
                device::exceptions::floating_point_exception(device)
            }
            // only the RM bit 1 and FS bit affect the host flush mode
            const FLUSH_MODE_MASK: u32 = 2 | FCR31_FS_BIT;
            if (old_fcr31 ^ device.cpu.cop1.fcr31) & FLUSH_MODE_MASK != 0 {
                set_host_flush_mode(device);
            }
        }
        _ => {
            panic!("unknown FCR register")
        }
    }
}

pub fn set_host_flush_mode(device: &device::Device) {
    let fcr31 = device.cpu.cop1.fcr31;
    let flush = !((fcr31 & 2) != 0 && (fcr31 & FCR31_FS_BIT) != 0);

    #[cfg(target_arch = "x86_64")]
    unsafe {
        // MXCSR: FTZ (bit 15) flushes denormal results, DAZ (bit 6) flushes denormal
        // inputs. Set both so behaviour matches aarch64's FPCR.FZ.
        const MXCSR_FTZ_DAZ: u32 = (1 << 15) | (1 << 6);
        let mut mxcsr: u32 = 0;
        std::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack, preserves_flags));
        let new = if flush {
            mxcsr | MXCSR_FTZ_DAZ
        } else {
            mxcsr & !MXCSR_FTZ_DAZ
        };
        if new != mxcsr {
            std::arch::asm!("ldmxcsr [{}]", in(reg) &new, options(nostack, preserves_flags, readonly));
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        // FPCR.FZ (bit 24) flushes both denormal inputs and results
        const FPCR_FZ: u64 = 1 << 24;
        let fpcr: u64;
        std::arch::asm!("mrs {}, fpcr", out(reg) fpcr, options(nomem, nostack, preserves_flags));
        let new = if flush {
            fpcr | FPCR_FZ
        } else {
            fpcr & !FPCR_FZ
        };
        if new != fpcr {
            std::arch::asm!("msr fpcr, {}", in(reg) new, options(nomem, nostack, preserves_flags));
        }
    }
}

// In half mode (Status.FR = 0) arithmetic operations ignore the lowest bit of fs, while ft and
// fd use the full index
#[inline(always)]
pub fn fs_index(device: &device::Device, index: u32) -> usize {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG] & device::cop0::COP0_STATUS_FR == 0 {
        (index & !1) as usize
    } else {
        index as usize
    }
}

// 32 bit transfers (MTC1, MFC1, LWC1, SWC1): in half mode odd indices access the upper half of
// the even register
pub fn get_fpr_word(device: &device::Device, index: u32) -> u32 {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG] & device::cop0::COP0_STATUS_FR == 0 {
        (device.cpu.cop1.fgr[(index & !1) as usize] >> ((index & 1) * 32)) as u32
    } else {
        device.cpu.cop1.fgr[index as usize] as u32
    }
}

pub fn set_fpr_word(device: &mut device::Device, index: u32, value: u32) {
    let (register, shift) = if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG]
        & device::cop0::COP0_STATUS_FR
        == 0
    {
        ((index & !1) as usize, (index & 1) * 32)
    } else {
        (index as usize, 0)
    };
    let fgr = &mut device.cpu.cop1.fgr[register];
    *fgr = (*fgr & !(0xFFFFFFFF << shift)) | ((value as u64) << shift);
}

// 64 bit transfers (DMTC1, DMFC1, LDC1, SDC1)
pub fn get_fpr_dword(device: &device::Device, index: u32) -> u64 {
    device.cpu.cop1.fgr[fs_index(device, index)]
}

pub fn set_fpr_dword(device: &mut device::Device, index: u32, value: u64) {
    let index = fs_index(device, index);
    device.cpu.cop1.fgr[index] = value
}

pub fn map_instructions(device: &mut device::Device) {
    device.cpu.cop1.b_instrs = [
        device::fpu_instructions::bc1f,  // 0
        device::fpu_instructions::bc1t,  // 1
        device::fpu_instructions::bc1fl, // 2
        device::fpu_instructions::bc1tl, // 3
        device::cop1::unimplemented,     // 4
        device::cop1::unimplemented,     // 5
        device::cop1::unimplemented,     // 6
        device::cop1::unimplemented,     // 7
        device::cop1::unimplemented,     // 8
        device::cop1::unimplemented,     // 9
        device::cop1::unimplemented,     // 10
        device::cop1::unimplemented,     // 11
        device::cop1::unimplemented,     // 12
        device::cop1::unimplemented,     // 13
        device::cop1::unimplemented,     // 14
        device::cop1::unimplemented,     // 15
        device::cop1::unimplemented,     // 16
        device::cop1::unimplemented,     // 17
        device::cop1::unimplemented,     // 18
        device::cop1::unimplemented,     // 19
        device::cop1::unimplemented,     // 20
        device::cop1::unimplemented,     // 21
        device::cop1::unimplemented,     // 22
        device::cop1::unimplemented,     // 23
        device::cop1::unimplemented,     // 24
        device::cop1::unimplemented,     // 25
        device::cop1::unimplemented,     // 26
        device::cop1::unimplemented,     // 27
        device::cop1::unimplemented,     // 28
        device::cop1::unimplemented,     // 29
        device::cop1::unimplemented,     // 30
        device::cop1::unimplemented,     // 31
    ];

    device.cpu.cop1.s_instrs = [
        device::fpu_instructions::add_s,             // 0
        device::fpu_instructions::sub_s,             // 1
        device::fpu_instructions::mul_s,             // 2
        device::fpu_instructions::div_s,             // 3
        device::fpu_instructions::sqrt_s,            // 4
        device::fpu_instructions::abs_s,             // 5
        device::fpu_instructions::mov_s,             // 6
        device::fpu_instructions::neg_s,             // 7
        device::fpu_instructions::round_l_s,         // 8
        device::fpu_instructions::trunc_l_s,         // 9
        device::fpu_instructions::ceil_l_s,          // 10
        device::fpu_instructions::floor_l_s,         // 11
        device::fpu_instructions::round_w_s,         // 12
        device::fpu_instructions::trunc_w_s,         // 13
        device::fpu_instructions::ceil_w_s,          // 14
        device::fpu_instructions::floor_w_s,         // 15
        device::cop1::unimplemented,                 // 16
        device::cop1::unimplemented,                 // 17
        device::cop1::unimplemented,                 // 18
        device::cop1::unimplemented,                 // 19
        device::cop1::unimplemented,                 // 20
        device::cop1::unimplemented,                 // 21
        device::cop1::unimplemented,                 // 22
        device::cop1::unimplemented,                 // 23
        device::cop1::unimplemented,                 // 24
        device::cop1::unimplemented,                 // 25
        device::cop1::unimplemented,                 // 26
        device::cop1::unimplemented,                 // 27
        device::cop1::unimplemented,                 // 28
        device::cop1::unimplemented,                 // 29
        device::cop1::unimplemented,                 // 30
        device::cop1::unimplemented,                 // 31
        device::cop1::unimplemented,                 // 32
        device::fpu_instructions::cvt_d_s,           // 33
        device::cop1::unimplemented,                 // 34
        device::cop1::unimplemented,                 // 35
        device::fpu_instructions::cvt_w_s,           // 36
        device::fpu_instructions::cvt_l_s,           // 37
        device::cop1::unimplemented,                 // 38
        device::cop1::unimplemented,                 // 38
        device::cop1::unimplemented,                 // 40
        device::cop1::unimplemented,                 // 41
        device::cop1::unimplemented,                 // 42
        device::cop1::unimplemented,                 // 43
        device::cop1::unimplemented,                 // 44
        device::cop1::unimplemented,                 // 45
        device::cop1::unimplemented,                 // 46
        device::cop1::unimplemented,                 // 47
        device::fpu_instructions::c_cond::<f32, 0>,  // 48
        device::fpu_instructions::c_cond::<f32, 1>,  // 49
        device::fpu_instructions::c_cond::<f32, 2>,  // 50
        device::fpu_instructions::c_cond::<f32, 3>,  // 51
        device::fpu_instructions::c_cond::<f32, 4>,  // 52
        device::fpu_instructions::c_cond::<f32, 5>,  // 53
        device::fpu_instructions::c_cond::<f32, 6>,  // 54
        device::fpu_instructions::c_cond::<f32, 7>,  // 55
        device::fpu_instructions::c_cond::<f32, 8>,  // 56
        device::fpu_instructions::c_cond::<f32, 9>,  // 57
        device::fpu_instructions::c_cond::<f32, 10>, // 58
        device::fpu_instructions::c_cond::<f32, 11>, // 59
        device::fpu_instructions::c_cond::<f32, 12>, // 60
        device::fpu_instructions::c_cond::<f32, 13>, // 61
        device::fpu_instructions::c_cond::<f32, 14>, // 62
        device::fpu_instructions::c_cond::<f32, 15>, // 63
    ];

    device.cpu.cop1.d_instrs = [
        device::fpu_instructions::add_d,             // 0
        device::fpu_instructions::sub_d,             // 1
        device::fpu_instructions::mul_d,             // 2
        device::fpu_instructions::div_d,             // 3
        device::fpu_instructions::sqrt_d,            // 4
        device::fpu_instructions::abs_d,             // 5
        device::fpu_instructions::mov_d,             // 6
        device::fpu_instructions::neg_d,             // 7
        device::fpu_instructions::round_l_d,         // 8
        device::fpu_instructions::trunc_l_d,         // 9
        device::fpu_instructions::ceil_l_d,          // 10
        device::fpu_instructions::floor_l_d,         // 11
        device::fpu_instructions::round_w_d,         // 12
        device::fpu_instructions::trunc_w_d,         // 13
        device::fpu_instructions::ceil_w_d,          // 14
        device::fpu_instructions::floor_w_d,         // 15
        device::cop1::unimplemented,                 // 16
        device::cop1::unimplemented,                 // 17
        device::cop1::unimplemented,                 // 18
        device::cop1::unimplemented,                 // 19
        device::cop1::unimplemented,                 // 20
        device::cop1::unimplemented,                 // 21
        device::cop1::unimplemented,                 // 22
        device::cop1::unimplemented,                 // 23
        device::cop1::unimplemented,                 // 24
        device::cop1::unimplemented,                 // 25
        device::cop1::unimplemented,                 // 26
        device::cop1::unimplemented,                 // 27
        device::cop1::unimplemented,                 // 28
        device::cop1::unimplemented,                 // 29
        device::cop1::unimplemented,                 // 30
        device::cop1::unimplemented,                 // 31
        device::fpu_instructions::cvt_s_d,           // 32
        device::cop1::unimplemented,                 // 33
        device::cop1::unimplemented,                 // 34
        device::cop1::unimplemented,                 // 35
        device::fpu_instructions::cvt_w_d,           // 36
        device::fpu_instructions::cvt_l_d,           // 37
        device::cop1::unimplemented,                 // 38
        device::cop1::unimplemented,                 // 38
        device::cop1::unimplemented,                 // 40
        device::cop1::unimplemented,                 // 41
        device::cop1::unimplemented,                 // 42
        device::cop1::unimplemented,                 // 43
        device::cop1::unimplemented,                 // 44
        device::cop1::unimplemented,                 // 45
        device::cop1::unimplemented,                 // 46
        device::cop1::unimplemented,                 // 47
        device::fpu_instructions::c_cond::<f64, 0>,  // 48
        device::fpu_instructions::c_cond::<f64, 1>,  // 49
        device::fpu_instructions::c_cond::<f64, 2>,  // 50
        device::fpu_instructions::c_cond::<f64, 3>,  // 51
        device::fpu_instructions::c_cond::<f64, 4>,  // 52
        device::fpu_instructions::c_cond::<f64, 5>,  // 53
        device::fpu_instructions::c_cond::<f64, 6>,  // 54
        device::fpu_instructions::c_cond::<f64, 7>,  // 55
        device::fpu_instructions::c_cond::<f64, 8>,  // 56
        device::fpu_instructions::c_cond::<f64, 9>,  // 57
        device::fpu_instructions::c_cond::<f64, 10>, // 58
        device::fpu_instructions::c_cond::<f64, 11>, // 59
        device::fpu_instructions::c_cond::<f64, 12>, // 60
        device::fpu_instructions::c_cond::<f64, 13>, // 61
        device::fpu_instructions::c_cond::<f64, 14>, // 62
        device::fpu_instructions::c_cond::<f64, 15>, // 63
    ];

    device.cpu.cop1.l_instrs = [
        device::cop1::unimplemented,       // 0
        device::cop1::unimplemented,       // 1
        device::cop1::unimplemented,       // 2
        device::cop1::unimplemented,       // 3
        device::cop1::unimplemented,       // 4
        device::cop1::unimplemented,       // 5
        device::cop1::unimplemented,       // 6
        device::cop1::unimplemented,       // 7
        device::cop1::unimplemented,       // 8
        device::cop1::unimplemented,       // 9
        device::cop1::unimplemented,       // 10
        device::cop1::unimplemented,       // 11
        device::cop1::unimplemented,       // 12
        device::cop1::unimplemented,       // 13
        device::cop1::unimplemented,       // 14
        device::cop1::unimplemented,       // 15
        device::cop1::unimplemented,       // 16
        device::cop1::unimplemented,       // 17
        device::cop1::unimplemented,       // 18
        device::cop1::unimplemented,       // 19
        device::cop1::unimplemented,       // 20
        device::cop1::unimplemented,       // 21
        device::cop1::unimplemented,       // 22
        device::cop1::unimplemented,       // 23
        device::cop1::unimplemented,       // 24
        device::cop1::unimplemented,       // 25
        device::cop1::unimplemented,       // 26
        device::cop1::unimplemented,       // 27
        device::cop1::unimplemented,       // 28
        device::cop1::unimplemented,       // 29
        device::cop1::unimplemented,       // 30
        device::cop1::unimplemented,       // 31
        device::fpu_instructions::cvt_s_l, // 32
        device::fpu_instructions::cvt_d_l, // 33
        device::cop1::unimplemented,       // 34
        device::cop1::unimplemented,       // 35
        device::cop1::unimplemented,       // 36
        device::cop1::unimplemented,       // 37
        device::cop1::unimplemented,       // 38
        device::cop1::unimplemented,       // 39
        device::cop1::unimplemented,       // 40
        device::cop1::unimplemented,       // 41
        device::cop1::unimplemented,       // 42
        device::cop1::unimplemented,       // 43
        device::cop1::unimplemented,       // 44
        device::cop1::unimplemented,       // 45
        device::cop1::unimplemented,       // 46
        device::cop1::unimplemented,       // 47
        device::cop1::unimplemented,       // 48
        device::cop1::unimplemented,       // 49
        device::cop1::unimplemented,       // 50
        device::cop1::unimplemented,       // 51
        device::cop1::unimplemented,       // 52
        device::cop1::unimplemented,       // 53
        device::cop1::unimplemented,       // 54
        device::cop1::unimplemented,       // 55
        device::cop1::unimplemented,       // 56
        device::cop1::unimplemented,       // 57
        device::cop1::unimplemented,       // 58
        device::cop1::unimplemented,       // 59
        device::cop1::unimplemented,       // 60
        device::cop1::unimplemented,       // 61
        device::cop1::unimplemented,       // 62
        device::cop1::unimplemented,       // 63
    ];

    device.cpu.cop1.w_instrs = [
        device::cop1::unimplemented,       // 0
        device::cop1::unimplemented,       // 1
        device::cop1::unimplemented,       // 2
        device::cop1::unimplemented,       // 3
        device::cop1::unimplemented,       // 4
        device::cop1::unimplemented,       // 5
        device::cop1::unimplemented,       // 6
        device::cop1::unimplemented,       // 7
        device::cop1::unimplemented,       // 8
        device::cop1::unimplemented,       // 9
        device::cop1::unimplemented,       // 10
        device::cop1::unimplemented,       // 11
        device::cop1::unimplemented,       // 12
        device::cop1::unimplemented,       // 13
        device::cop1::unimplemented,       // 14
        device::cop1::unimplemented,       // 15
        device::cop1::unimplemented,       // 16
        device::cop1::unimplemented,       // 17
        device::cop1::unimplemented,       // 18
        device::cop1::unimplemented,       // 19
        device::cop1::unimplemented,       // 20
        device::cop1::unimplemented,       // 21
        device::cop1::unimplemented,       // 22
        device::cop1::unimplemented,       // 23
        device::cop1::unimplemented,       // 24
        device::cop1::unimplemented,       // 25
        device::cop1::unimplemented,       // 26
        device::cop1::unimplemented,       // 27
        device::cop1::unimplemented,       // 28
        device::cop1::unimplemented,       // 29
        device::cop1::unimplemented,       // 30
        device::cop1::unimplemented,       // 31
        device::fpu_instructions::cvt_s_w, // 32
        device::fpu_instructions::cvt_d_w, // 33
        device::cop1::unimplemented,       // 34
        device::cop1::unimplemented,       // 35
        device::cop1::unimplemented,       // 36
        device::cop1::unimplemented,       // 37
        device::cop1::unimplemented,       // 38
        device::cop1::unimplemented,       // 39
        device::cop1::unimplemented,       // 40
        device::cop1::unimplemented,       // 41
        device::cop1::unimplemented,       // 42
        device::cop1::unimplemented,       // 43
        device::cop1::unimplemented,       // 44
        device::cop1::unimplemented,       // 45
        device::cop1::unimplemented,       // 46
        device::cop1::unimplemented,       // 47
        device::cop1::unimplemented,       // 48
        device::cop1::unimplemented,       // 49
        device::cop1::unimplemented,       // 50
        device::cop1::unimplemented,       // 51
        device::cop1::unimplemented,       // 52
        device::cop1::unimplemented,       // 53
        device::cop1::unimplemented,       // 54
        device::cop1::unimplemented,       // 55
        device::cop1::unimplemented,       // 56
        device::cop1::unimplemented,       // 57
        device::cop1::unimplemented,       // 58
        device::cop1::unimplemented,       // 59
        device::cop1::unimplemented,       // 60
        device::cop1::unimplemented,       // 61
        device::cop1::unimplemented,       // 62
        device::cop1::unimplemented,       // 63
    ];

    device.cpu.cop1.instrs = [
        device::cop1::mfc1,           // 0
        device::cop1::dmfc1,          // 1
        device::cop1::cfc1,           // 2
        device::cop1::unimplemented,  // 3
        device::cop1::mtc1,           // 4
        device::cop1::dmtc1,          // 5
        device::cop1::ctc1,           // 6
        device::cop1::unimplemented,  // 7
        device::cop1::execute_cop1_b, // 8
        device::cop1::unimplemented,  // 9
        device::cop1::unimplemented,  // 10
        device::cop1::unimplemented,  // 11
        device::cop1::unimplemented,  // 12
        device::cop1::unimplemented,  // 13
        device::cop1::unimplemented,  // 14
        device::cop1::unimplemented,  // 15
        device::cop1::execute_cop1_s, // 16
        device::cop1::execute_cop1_d, // 17
        device::cop1::unimplemented,  // 18
        device::cop1::unimplemented,  // 19
        device::cop1::execute_cop1_w, // 20
        device::cop1::execute_cop1_l, // 21
        device::cop1::unimplemented,  // 22
        device::cop1::unimplemented,  // 23
        device::cop1::unimplemented,  // 24
        device::cop1::unimplemented,  // 25
        device::cop1::unimplemented,  // 26
        device::cop1::unimplemented,  // 27
        device::cop1::unimplemented,  // 28
        device::cop1::unimplemented,  // 29
        device::cop1::unimplemented,  // 30
        device::cop1::unimplemented,  // 31
    ]
}

pub fn init(device: &mut device::Device) {
    device.cpu.cop1.fcr0 = 0b101000000000;

    map_instructions(device);
    set_host_flush_mode(device);
}
