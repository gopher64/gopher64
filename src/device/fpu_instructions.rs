use crate::device;

pub fn fs(opcode: u32) -> u32 {
    (opcode >> 11) & 0x1F
}

pub fn ft(opcode: u32) -> u32 {
    (opcode >> 16) & 0x1F
}

pub fn fd(opcode: u32) -> u32 {
    (opcode >> 6) & 0x1F
}

pub fn bc1f(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & device::cop1::FCR31_CMP_BIT == 0 {
        let target = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
        device::cpu_instructions::check_idle_loop(device, target);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = target;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bc1t(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & device::cop1::FCR31_CMP_BIT != 0 {
        let target = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
        device::cpu_instructions::check_idle_loop(device, target);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = target;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bc1fl(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & device::cop1::FCR31_CMP_BIT == 0 {
        let target = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
        device::cpu_instructions::check_idle_loop(device, target);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = target;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn bc1tl(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & device::cop1::FCR31_CMP_BIT != 0 {
        let target = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
        device::cpu_instructions::check_idle_loop(device, target);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = target;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn add_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs + ft, true);
    device::cop0::add_cycles(device, 2)
}

pub fn sub_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs - ft, true);
    device::cop0::add_cycles(device, 2)
}

pub fn mul_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs * ft, true);
    device::cop0::add_cycles(device, 4)
}

pub fn div_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs / ft, true);
    device::cop0::add_cycles(device, 28)
}

pub fn sqrt_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs.sqrt(), true);
    device::cop0::add_cycles(device, 28)
}

pub fn abs_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs.abs(), true);
    device::cop0::add_cycles(device, 2)
}

pub fn mov_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs);
}

pub fn neg_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, -fs, true);
}

pub fn round_l_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.round_ties_even() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn trunc_l_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.trunc() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn ceil_l_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.ceil() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn floor_l_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.floor() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn round_w_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.round_ties_even() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn trunc_w_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.trunc() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn ceil_w_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.ceil() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn floor_w_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.floor() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_d_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs as f64);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_w_s(device: &mut device::Device, opcode: u32) {
    match device.cpu.cop1.fcr31 & 3 {
        0 => round_w_s(device, opcode),
        1 => trunc_w_s(device, opcode),
        2 => ceil_w_s(device, opcode),
        3 => floor_w_s(device, opcode),
        _ => panic!("unknown cvt_w_s"),
    }
}

pub fn cvt_l_s(device: &mut device::Device, opcode: u32) {
    match device.cpu.cop1.fcr31 & 3 {
        0 => round_l_s(device, opcode),
        1 => trunc_l_s(device, opcode),
        2 => ceil_l_s(device, opcode),
        3 => floor_l_s(device, opcode),
        _ => panic!("unknown cvt_l_s"),
    }
}

pub fn c_f_s(device: &mut device::Device, _opcode: u32) {
    device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT
}

pub fn c_un_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_eq_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ueq_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_olt_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ult_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ole_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ule_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_sf_s(device: &mut device::Device, _opcode: u32) {
    device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
}

pub fn c_ngle_s(device: &mut device::Device, _opcode: u32) {
    device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
}

pub fn c_seq_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);

    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ngl_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);

    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_lt_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);

    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_nge_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);

    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_le_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);

    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ngt_s(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_single(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_single(device, ft(opcode) as usize);

    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn add_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs + ft);
    device::cop0::add_cycles(device, 2)
}

pub fn sub_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs - ft);
    device::cop0::add_cycles(device, 2)
}

pub fn mul_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs * ft);
    device::cop0::add_cycles(device, 7)
}

pub fn div_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs / ft);
    device::cop0::add_cycles(device, 57)
}

pub fn sqrt_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs.sqrt());
    device::cop0::add_cycles(device, 57)
}

pub fn abs_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs.abs());
    device::cop0::add_cycles(device, 2)
}

pub fn mov_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs);
}

pub fn neg_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    device::cop1::set_fpr_double(device, fd(opcode) as usize, -fs);
}

pub fn round_l_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.round_ties_even() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn trunc_l_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.trunc() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn ceil_l_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.ceil() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn floor_l_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f64::from_ne_bytes((fs.floor() as i64).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, value);
    device::cop0::add_cycles(device, 4)
}

pub fn round_w_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.round_ties_even() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn trunc_w_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.trunc() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn ceil_w_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.ceil() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn floor_w_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let value = f32::from_ne_bytes((fs.floor() as i32).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, value, true);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_s_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs as f32, true);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_w_d(device: &mut device::Device, opcode: u32) {
    match device.cpu.cop1.fcr31 & 3 {
        0 => round_w_d(device, opcode),
        1 => trunc_w_d(device, opcode),
        2 => ceil_w_d(device, opcode),
        3 => floor_w_d(device, opcode),
        _ => panic!("unknown cvt_w_d"),
    }
}

pub fn cvt_l_d(device: &mut device::Device, opcode: u32) {
    match device.cpu.cop1.fcr31 & 3 {
        0 => round_l_d(device, opcode),
        1 => trunc_l_d(device, opcode),
        2 => ceil_l_d(device, opcode),
        3 => floor_l_d(device, opcode),
        _ => panic!("unknown cvt_l_d"),
    }
}

pub fn c_f_d(device: &mut device::Device, _opcode: u32) {
    device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
}

pub fn c_un_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_eq_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ueq_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_olt_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ult_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ole_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ule_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);
    if fs.is_nan() || ft.is_nan() {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
        return;
    }
    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_sf_d(device: &mut device::Device, _opcode: u32) {
    device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
}

pub fn c_ngle_d(device: &mut device::Device, _opcode: u32) {
    device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
}

pub fn c_seq_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);

    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ngl_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);

    if fs == ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_lt_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);

    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_nge_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);

    if fs < ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_le_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);

    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn c_ngt_d(device: &mut device::Device, opcode: u32) {
    let fs = device::cop1::get_fpr_double(device, fs(opcode) as usize);
    let ft = device::cop1::get_fpr_double(device, ft(opcode) as usize);

    if fs <= ft {
        device.cpu.cop1.fcr31 |= device::cop1::FCR31_CMP_BIT;
    } else {
        device.cpu.cop1.fcr31 &= !device::cop1::FCR31_CMP_BIT;
    }
}

pub fn cvt_s_l(device: &mut device::Device, opcode: u32) {
    let fs =
        i64::from_ne_bytes(device::cop1::get_fpr_double(device, fs(opcode) as usize).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs as f32, true);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_d_l(device: &mut device::Device, opcode: u32) {
    let fs =
        i64::from_ne_bytes(device::cop1::get_fpr_double(device, fs(opcode) as usize).to_ne_bytes());
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs as f64);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_s_w(device: &mut device::Device, opcode: u32) {
    let fs =
        i32::from_ne_bytes(device::cop1::get_fpr_single(device, fs(opcode) as usize).to_ne_bytes());
    device::cop1::set_fpr_single(device, fd(opcode) as usize, fs as f32, true);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_d_w(device: &mut device::Device, opcode: u32) {
    let fs = i32::from_ne_bytes(
        device::cop1::get_fpr_double(device, fs(opcode) as usize).to_ne_bytes()[0..4]
            .try_into()
            .unwrap(),
    );
    device::cop1::set_fpr_double(device, fd(opcode) as usize, fs as f64);
    device::cop0::add_cycles(device, 4)
}
