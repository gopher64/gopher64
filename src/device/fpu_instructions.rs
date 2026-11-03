use crate::device;
use crate::device::cop1::FCR31_CMP_BIT;

pub fn fs(opcode: u32) -> u32 {
    (opcode >> 11) & 0x1F
}

pub fn ft(opcode: u32) -> u32 {
    (opcode >> 16) & 0x1F
}

fn fd(opcode: u32) -> u32 {
    (opcode >> 6) & 0x1F
}

pub trait FpuFloat: Copy + PartialEq + PartialOrd {
    fn is_nan(self) -> bool;
    fn round_ties_even(self) -> Self;
    fn trunc(self) -> Self;
    fn ceil(self) -> Self;
    fn floor(self) -> Self;
    fn to_i64(self) -> i64;
    fn get_fs(device: &device::Device, index: u32) -> Self;
    fn get_ft(device: &device::Device, index: u32) -> Self;
    fn set_fd(device: &mut device::Device, index: u32, value: Self);
}

macro_rules! impl_fpu_float {
    ($t:ty, $bits:ty) => {
        impl FpuFloat for $t {
            fn is_nan(self) -> bool {
                <$t>::is_nan(self)
            }
            // lowered to roundeven/frintn, which don't depend on the host rounding mode
            fn round_ties_even(self) -> Self {
                <$t>::round_ties_even(self)
            }
            fn trunc(self) -> Self {
                <$t>::trunc(self)
            }
            fn ceil(self) -> Self {
                <$t>::ceil(self)
            }
            fn floor(self) -> Self {
                <$t>::floor(self)
            }
            fn to_i64(self) -> i64 {
                self as i64
            }
            fn get_fs(device: &device::Device, index: u32) -> Self {
                <$t>::from_bits(device.cpu.cop1.fgr[device::cop1::fs_index(device, index)] as $bits)
            }
            fn get_ft(device: &device::Device, index: u32) -> Self {
                <$t>::from_bits(device.cpu.cop1.fgr[index as usize] as $bits)
            }
            // 32 bit results clear the upper half of the register
            fn set_fd(device: &mut device::Device, index: u32, value: Self) {
                device.cpu.cop1.fgr[index as usize] = value.to_bits() as u64
            }
        }
    };
}

impl_fpu_float!(f32, u32);
impl_fpu_float!(f64, u64);

fn get_fs_s32(device: &device::Device, index: u32) -> i32 {
    device.cpu.cop1.fgr[device::cop1::fs_index(device, index)] as i32
}

fn get_fs_s64(device: &device::Device, index: u32) -> i64 {
    device.cpu.cop1.fgr[device::cop1::fs_index(device, index)] as i64
}

fn set_fd_s32(device: &mut device::Device, index: u32, value: i32) {
    device.cpu.cop1.fgr[index as usize] = value as u32 as u64
}

fn set_fd_s64(device: &mut device::Device, index: u32, value: i64) {
    device.cpu.cop1.fgr[index as usize] = value as u64
}

#[inline(always)]
fn binary_op<F: FpuFloat>(
    device: &mut device::Device,
    opcode: u32,
    cycles: u64,
    op: impl FnOnce(F, F) -> F,
) {
    let a = F::get_fs(device, fs(opcode));
    let b = F::get_ft(device, ft(opcode));
    F::set_fd(device, fd(opcode), op(a, b));
    device::cop0::add_cycles(device, cycles)
}

#[inline(always)]
fn unary_op<F: FpuFloat, R: FpuFloat>(
    device: &mut device::Device,
    opcode: u32,
    cycles: u64,
    op: impl FnOnce(F) -> R,
) {
    let a = F::get_fs(device, fs(opcode));
    R::set_fd(device, fd(opcode), op(a));
    device::cop0::add_cycles(device, cycles)
}

const ROUND_NEAREST: u32 = 0;
const ROUND_TRUNC: u32 = 1;
const ROUND_CEIL: u32 = 2;
const ROUND_FLOOR: u32 = 3;

// Conversion to a 32 bit (W) or 64 bit (L) integer
#[inline(always)]
fn float_to_int<F: FpuFloat, const LONG: bool>(
    device: &mut device::Device,
    opcode: u32,
    mode: u32,
) {
    let value = F::get_fs(device, fs(opcode));
    let result = match mode {
        ROUND_NEAREST => value.round_ties_even(),
        ROUND_TRUNC => value.trunc(),
        ROUND_CEIL => value.ceil(),
        _ => value.floor(),
    };
    if LONG {
        set_fd_s64(device, fd(opcode), result.to_i64())
    } else {
        set_fd_s32(device, fd(opcode), result.to_i64() as i32)
    }
    device::cop0::add_cycles(device, 4)
}

/// C.cond.fmt, COND is the 4 bit condition from the opcode:
/// bit 0: true if unordered, bit 1: true if equal, bit 2: true if less than
pub fn c_cond<F: FpuFloat, const COND: u32>(device: &mut device::Device, opcode: u32) {
    let a = F::get_fs(device, fs(opcode));
    let b = F::get_ft(device, ft(opcode));
    let condition = if a.is_nan() || b.is_nan() {
        COND & 1 != 0
    } else {
        (COND & 2 != 0 && a == b) || (COND & 4 != 0 && a < b)
    };
    if condition {
        device.cpu.cop1.fcr31 |= FCR31_CMP_BIT
    } else {
        device.cpu.cop1.fcr31 &= !FCR31_CMP_BIT
    }
}

pub fn bc1f(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & FCR31_CMP_BIT == 0 {
        device::cpu_instructions::check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bc1t(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & FCR31_CMP_BIT != 0 {
        device::cpu_instructions::check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::NotTaken;
    }
}

pub fn bc1fl(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & FCR31_CMP_BIT == 0 {
        device::cpu_instructions::check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn bc1tl(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop1.fcr31 & FCR31_CMP_BIT != 0 {
        device::cpu_instructions::check_relative_idle_loop(device, opcode);
        device.cpu.branch_state.state = device::cpu::State::Take;
        device.cpu.branch_state.pc = device.cpu.pc.wrapping_add(
            device::cpu_instructions::se16(device::cpu_instructions::imm(opcode) as i16) << 2,
        ) + 4;
    } else {
        device.cpu.branch_state.state = device::cpu::State::Discard;
    }
}

pub fn add_s(device: &mut device::Device, opcode: u32) {
    binary_op::<f32>(device, opcode, 2, |a, b| a + b)
}

pub fn sub_s(device: &mut device::Device, opcode: u32) {
    binary_op::<f32>(device, opcode, 2, |a, b| a - b)
}

pub fn mul_s(device: &mut device::Device, opcode: u32) {
    binary_op::<f32>(device, opcode, 4, |a, b| a * b)
}

pub fn div_s(device: &mut device::Device, opcode: u32) {
    binary_op::<f32>(device, opcode, 28, |a, b| a / b)
}

pub fn sqrt_s(device: &mut device::Device, opcode: u32) {
    unary_op::<f32, f32>(device, opcode, 28, |a| a.sqrt())
}

pub fn abs_s(device: &mut device::Device, opcode: u32) {
    unary_op::<f32, f32>(device, opcode, 2, |a| a.abs())
}

// MOV copies the whole register and doesn't touch FCR31
pub fn mov_s(device: &mut device::Device, opcode: u32) {
    device.cpu.cop1.fgr[fd(opcode) as usize] =
        device.cpu.cop1.fgr[device::cop1::fs_index(device, fs(opcode))]
}

pub fn neg_s(device: &mut device::Device, opcode: u32) {
    unary_op::<f32, f32>(device, opcode, 0, |a| -a)
}

pub fn round_l_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, true>(device, opcode, ROUND_NEAREST)
}

pub fn trunc_l_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, true>(device, opcode, ROUND_TRUNC)
}

pub fn ceil_l_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, true>(device, opcode, ROUND_CEIL)
}

pub fn floor_l_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, true>(device, opcode, ROUND_FLOOR)
}

pub fn round_w_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, false>(device, opcode, ROUND_NEAREST)
}

pub fn trunc_w_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, false>(device, opcode, ROUND_TRUNC)
}

pub fn ceil_w_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, false>(device, opcode, ROUND_CEIL)
}

pub fn floor_w_s(device: &mut device::Device, opcode: u32) {
    float_to_int::<f32, false>(device, opcode, ROUND_FLOOR)
}

pub fn cvt_d_s(device: &mut device::Device, opcode: u32) {
    unary_op::<f32, f64>(device, opcode, 4, |a| a as f64)
}

pub fn cvt_w_s(device: &mut device::Device, opcode: u32) {
    let mode = device.cpu.cop1.fcr31 & 3;
    float_to_int::<f32, false>(device, opcode, mode)
}

pub fn cvt_l_s(device: &mut device::Device, opcode: u32) {
    let mode = device.cpu.cop1.fcr31 & 3;
    float_to_int::<f32, true>(device, opcode, mode)
}

pub fn add_d(device: &mut device::Device, opcode: u32) {
    binary_op::<f64>(device, opcode, 2, |a, b| a + b)
}

pub fn sub_d(device: &mut device::Device, opcode: u32) {
    binary_op::<f64>(device, opcode, 2, |a, b| a - b)
}

pub fn mul_d(device: &mut device::Device, opcode: u32) {
    binary_op::<f64>(device, opcode, 7, |a, b| a * b)
}

pub fn div_d(device: &mut device::Device, opcode: u32) {
    binary_op::<f64>(device, opcode, 57, |a, b| a / b)
}

pub fn sqrt_d(device: &mut device::Device, opcode: u32) {
    unary_op::<f64, f64>(device, opcode, 57, |a| a.sqrt())
}

pub fn abs_d(device: &mut device::Device, opcode: u32) {
    unary_op::<f64, f64>(device, opcode, 2, |a| a.abs())
}

pub fn mov_d(device: &mut device::Device, opcode: u32) {
    mov_s(device, opcode)
}

pub fn neg_d(device: &mut device::Device, opcode: u32) {
    unary_op::<f64, f64>(device, opcode, 0, |a| -a)
}

pub fn round_l_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, true>(device, opcode, ROUND_NEAREST)
}

pub fn trunc_l_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, true>(device, opcode, ROUND_TRUNC)
}

pub fn ceil_l_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, true>(device, opcode, ROUND_CEIL)
}

pub fn floor_l_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, true>(device, opcode, ROUND_FLOOR)
}

pub fn round_w_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, false>(device, opcode, ROUND_NEAREST)
}

pub fn trunc_w_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, false>(device, opcode, ROUND_TRUNC)
}

pub fn ceil_w_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, false>(device, opcode, ROUND_CEIL)
}

pub fn floor_w_d(device: &mut device::Device, opcode: u32) {
    float_to_int::<f64, false>(device, opcode, ROUND_FLOOR)
}

pub fn cvt_s_d(device: &mut device::Device, opcode: u32) {
    unary_op::<f64, f32>(device, opcode, 4, |a| a as f32)
}

pub fn cvt_w_d(device: &mut device::Device, opcode: u32) {
    let mode = device.cpu.cop1.fcr31 & 3;
    float_to_int::<f64, false>(device, opcode, mode)
}

pub fn cvt_l_d(device: &mut device::Device, opcode: u32) {
    let mode = device.cpu.cop1.fcr31 & 3;
    float_to_int::<f64, true>(device, opcode, mode)
}

pub fn cvt_s_l(device: &mut device::Device, opcode: u32) {
    let value = get_fs_s64(device, fs(opcode));
    f32::set_fd(device, fd(opcode), value as f32);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_d_l(device: &mut device::Device, opcode: u32) {
    let value = get_fs_s64(device, fs(opcode));
    f64::set_fd(device, fd(opcode), value as f64);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_s_w(device: &mut device::Device, opcode: u32) {
    let value = get_fs_s32(device, fs(opcode));
    f32::set_fd(device, fd(opcode), value as f32);
    device::cop0::add_cycles(device, 4)
}

pub fn cvt_d_w(device: &mut device::Device, opcode: u32) {
    let value = get_fs_s32(device, fs(opcode));
    f64::set_fd(device, fd(opcode), value as f64);
    device::cop0::add_cycles(device, 4)
}
