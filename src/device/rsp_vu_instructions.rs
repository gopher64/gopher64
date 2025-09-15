#[cfg(target_arch = "aarch64")]
use device::__m128i;
#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));
#[cfg(target_arch = "aarch64")]
include!("../compat/aarch64.rs");
use crate::device;
use crate::device::rsp_su_instructions::{get_vpr16, modify_vpr16};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

fn vt(opcode: u32) -> u32 {
    (opcode >> 16) & 0x1F
}

fn ve(opcode: u32) -> u32 {
    (opcode >> 21) & 0xF
}

fn vs(opcode: u32) -> u32 {
    (opcode >> 11) & 0x1F
}

fn vd(opcode: u32) -> u32 {
    (opcode >> 6) & 0x1F
}

fn de(opcode: u32) -> u32 {
    (opcode >> 11) & 0x7
}

fn clamp_signed_32(value: i32) -> i16 {
    value.clamp(-32768, 32767) as i16
}

fn clamp_signed_64(value: i64) -> i16 {
    value.clamp(-32768, 32767) as i16
}

fn s_clip(x: i64, bits: u32) -> i64 {
    let mask = (1i64 << bits) - 1;
    let value = x & mask;
    (value << (64 - bits)) >> (64 - bits)
}

fn compute_reciprocal(input: i32, reciprocals: &[u16]) -> u32 {
    let mask = input >> 31;
    let mut data = input ^ mask;
    if input > -32768 {
        data -= mask
    }
    if data == 0 {
        0x7fffffff
    } else if input == -32768 {
        0xffff0000
    } else {
        let shift = (data as u32).leading_zeros();
        let index = (((data as u64) << shift) & 0x7fc00000) >> 22;
        let mut result = reciprocals[index as usize] as u32;
        result = (0x10000 | result) << 14;
        (result >> (31 - shift)) ^ mask as u32
    }
}

fn compute_inverse_sqrt(input: i32, inverse_square_roots: &[u16]) -> u32 {
    let mask = input >> 31;
    let mut data = input ^ mask;
    if input > -32768 {
        data -= mask
    }
    if data == 0 {
        0x7fffffff
    } else if input == -32768 {
        0xffff0000
    } else {
        let shift = (data as u32).leading_zeros();
        let index = (((data as u64) << shift) & 0x7fc00000) as u32 >> 22;
        let mut result = inverse_square_roots[((index & 0x1fe) | (shift & 1)) as usize] as u32;
        result = (0x10000 | result) << 14;
        (result >> ((31 - shift) >> 1)) ^ mask as u32
    }
}

fn vte(device: &device::Device, vt: u32, index: usize) -> __m128i {
    unsafe {
        _mm_shuffle_epi8(
            device.rsp.cpu.vpr[vt as usize],
            device.rsp.cpu.shuffle[index],
        )
    }
}

pub fn vmulf(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let lo = _mm_mullo_epi16(vs_reg, vte);
        let hi = _mm_mulhi_epi16(vs_reg, vte);

        let sign1 = _mm_srli_epi16(lo, 15);
        let lo_doubled = _mm_add_epi16(lo, lo);
        let sign2 = _mm_srli_epi16(lo_doubled, 15);

        device.rsp.cpu.accl = _mm_add_epi16(_mm_set1_epi16(-32768_i16), lo_doubled); // round + lo
        device.rsp.cpu.accm = _mm_add_epi16(_mm_slli_epi16(hi, 1), _mm_add_epi16(sign1, sign2));
        let neg = _mm_srai_epi16(device.rsp.cpu.accm, 15);

        let neq = _mm_cmpeq_epi16(vs_reg, vte);
        let eq = _mm_and_si128(neq, neg);

        device.rsp.cpu.acch = _mm_andnot_si128(neq, neg);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_add_epi16(device.rsp.cpu.accm, eq);
    }
}

pub fn vmulu(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let lo = _mm_mullo_epi16(vs_reg, vte);
        let hi = _mm_mulhi_epi16(vs_reg, vte);

        let sign1 = _mm_srli_epi16(lo, 15);
        let lo_doubled = _mm_add_epi16(lo, lo);
        let sign2 = _mm_srli_epi16(lo_doubled, 15);

        device.rsp.cpu.accl = _mm_add_epi16(_mm_set1_epi16(-32768_i16), lo_doubled); // round + lo
        device.rsp.cpu.accm = _mm_add_epi16(_mm_slli_epi16(hi, 1), _mm_add_epi16(sign1, sign2));
        let neg = _mm_srai_epi16(device.rsp.cpu.accm, 15);

        let neq = _mm_cmpeq_epi16(vs_reg, vte);

        device.rsp.cpu.acch = _mm_andnot_si128(neq, neg);
        let result = _mm_or_si128(device.rsp.cpu.accm, neg);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_andnot_si128(device.rsp.cpu.acch, result);
    }
}

pub fn vrndp(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let acch: &mut __m128i = &mut device.rsp.cpu.acch;
    let accm: &mut __m128i = &mut device.rsp.cpu.accm;
    let accl: &mut __m128i = &mut device.rsp.cpu.accl;
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];
    let shift_amount = vs(opcode) & 1;

    for n in 0..8 {
        let mut product = get_vpr16(vte, n) as i16 as i32;
        if shift_amount != 0 {
            product <<= 16
        }
        let mut acc = 0;
        acc |= get_vpr16(*acch, n) as i64;
        acc <<= 16;
        acc |= get_vpr16(*accm, n) as i64;
        acc <<= 16;
        acc |= get_vpr16(*accl, n) as i64;
        acc <<= 16;
        acc >>= 16;
        if acc >= 0 {
            acc = s_clip(acc + (product as i64), 48)
        }
        modify_vpr16(acch, n, (acc >> 32) as u16);
        modify_vpr16(accm, n, (acc >> 16) as u16);
        modify_vpr16(accl, n, acc as u16);
        modify_vpr16(vd_reg, n, clamp_signed_64(acc >> 16) as u16);
    }
}

pub fn vmulq(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let acch: &mut __m128i = &mut device.rsp.cpu.acch;
    let accm: &mut __m128i = &mut device.rsp.cpu.accm;
    let accl: &mut __m128i = &mut device.rsp.cpu.accl;
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];

    for n in 0..8 {
        let mut product =
            (get_vpr16(vs_reg, n) as i16 as i32).wrapping_mul(get_vpr16(vte, n) as i16 as i32);
        if product < 0 {
            product += 31;
        }
        modify_vpr16(acch, n, (product >> 16) as u16);
        modify_vpr16(accm, n, product as u16);
        modify_vpr16(accl, n, 0);
        modify_vpr16(vd_reg, n, (clamp_signed_32(product >> 1) & !15) as u16);
    }
}

pub fn vmudl(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_mulhi_epu16(vs_reg, vte);
        device.rsp.cpu.accm = _mm_setzero_si128();
        device.rsp.cpu.acch = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vmudm(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_mullo_epi16(vs_reg, vte);
        device.rsp.cpu.accm = _mm_mulhi_epu16(vs_reg, vte);
        let sign = _mm_srai_epi16(vs_reg, 15);
        let vta = _mm_and_si128(vte, sign);
        device.rsp.cpu.accm = _mm_sub_epi16(device.rsp.cpu.accm, vta);
        device.rsp.cpu.acch = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accm;
    }
}

pub fn vmudn(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_mullo_epi16(vs_reg, vte);
        device.rsp.cpu.accm = _mm_mulhi_epu16(vs_reg, vte);
        let sign = _mm_srai_epi16(vte, 15);
        let vsa = _mm_and_si128(vs_reg, sign);
        device.rsp.cpu.accm = _mm_sub_epi16(device.rsp.cpu.accm, vsa);
        device.rsp.cpu.acch = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vmudh(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_setzero_si128();
        device.rsp.cpu.accm = _mm_mullo_epi16(vs_reg, vte);
        device.rsp.cpu.acch = _mm_mulhi_epi16(vs_reg, vte);
        let lo = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        let hi = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_packs_epi32(lo, hi);
    }
}

pub fn vmacf(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let lo = _mm_mullo_epi16(vs_reg, vte);
        let hi = _mm_mulhi_epi16(vs_reg, vte);

        let carry = _mm_srli_epi16(lo, 15);
        let md = _mm_or_si128(_mm_slli_epi16(hi, 1), carry);
        let lo_doubled = _mm_slli_epi16(lo, 1);
        let hi_sign = _mm_srai_epi16(hi, 15);

        let accl_old = device.rsp.cpu.accl;
        device.rsp.cpu.accl = _mm_add_epi16(accl_old, lo_doubled);
        let overflow_l = _mm_cmpeq_epi16(_mm_adds_epu16(accl_old, lo_doubled), device.rsp.cpu.accl);
        let borrow_from_l = _mm_cmpeq_epi16(overflow_l, _mm_setzero_si128());

        let md_adjusted = _mm_sub_epi16(md, borrow_from_l);
        let zero_md = _mm_cmpeq_epi16(md_adjusted, _mm_setzero_si128());
        let borrow_to_h = _mm_and_si128(zero_md, borrow_from_l);
        let hi_adjusted = _mm_sub_epi16(hi_sign, borrow_to_h);

        let accm_old = device.rsp.cpu.accm;
        device.rsp.cpu.accm = _mm_add_epi16(accm_old, md_adjusted);
        let overflow_m =
            _mm_cmpeq_epi16(_mm_adds_epu16(accm_old, md_adjusted), device.rsp.cpu.accm);
        let borrow_from_m = _mm_cmpeq_epi16(overflow_m, _mm_setzero_si128());

        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi_adjusted);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, borrow_from_m);

        let lo_packed = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        let hi_packed = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_packs_epi32(lo_packed, hi_packed);
    }
}

pub fn vmacu(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let lo = _mm_mullo_epi16(vs_reg, vte);
        let hi = _mm_mulhi_epi16(vs_reg, vte);

        let carry = _mm_srli_epi16(lo, 15);
        let md = _mm_or_si128(_mm_slli_epi16(hi, 1), carry);
        let lo_doubled = _mm_slli_epi16(lo, 1);
        let hi_sign = _mm_srai_epi16(hi, 15);

        let accl_old = device.rsp.cpu.accl;
        device.rsp.cpu.accl = _mm_add_epi16(accl_old, lo_doubled);
        let overflow_l = _mm_cmpeq_epi16(_mm_adds_epu16(accl_old, lo_doubled), device.rsp.cpu.accl);
        let borrow_from_l = _mm_cmpeq_epi16(overflow_l, _mm_setzero_si128());

        let md_adjusted = _mm_sub_epi16(md, borrow_from_l);
        let zero_md = _mm_cmpeq_epi16(md_adjusted, _mm_setzero_si128());
        let borrow_to_h = _mm_and_si128(zero_md, borrow_from_l);
        let hi_adjusted = _mm_sub_epi16(hi_sign, borrow_to_h);

        let accm_old = device.rsp.cpu.accm;
        device.rsp.cpu.accm = _mm_add_epi16(accm_old, md_adjusted);
        let overflow_m =
            _mm_cmpeq_epi16(_mm_adds_epu16(accm_old, md_adjusted), device.rsp.cpu.accm);
        let borrow_from_m = _mm_cmpeq_epi16(overflow_m, _mm_setzero_si128());

        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi_adjusted);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, borrow_from_m);

        let mmask = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        let hmask = _mm_srai_epi16(device.rsp.cpu.acch, 15);
        let md_result = _mm_or_si128(mmask, device.rsp.cpu.accm);
        let positive = _mm_cmpgt_epi16(device.rsp.cpu.acch, _mm_setzero_si128());
        let final_result = _mm_andnot_si128(hmask, md_result);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_or_si128(positive, final_result);
    }
}

pub fn vrndn(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let acch: &mut __m128i = &mut device.rsp.cpu.acch;
    let accm: &mut __m128i = &mut device.rsp.cpu.accm;
    let accl: &mut __m128i = &mut device.rsp.cpu.accl;
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];
    let shift_amount = vs(opcode) & 1;

    for n in 0..8 {
        let mut product = get_vpr16(vte, n) as i16 as i32;
        if shift_amount != 0 {
            product <<= 16
        }
        let mut acc = 0;
        acc |= get_vpr16(*acch, n) as i64;
        acc <<= 16;
        acc |= get_vpr16(*accm, n) as i64;
        acc <<= 16;
        acc |= get_vpr16(*accl, n) as i64;
        acc <<= 16;
        acc >>= 16;
        if acc < 0 {
            acc = s_clip(acc + (product as i64), 48)
        }
        modify_vpr16(acch, n, (acc >> 32) as u16);
        modify_vpr16(accm, n, (acc >> 16) as u16);
        modify_vpr16(accl, n, acc as u16);
        modify_vpr16(vd_reg, n, clamp_signed_64(acc >> 16) as u16);
    }
}

pub fn vmacq(device: &mut device::Device, opcode: u32) {
    let acch: &mut __m128i = &mut device.rsp.cpu.acch;
    let accm: &mut __m128i = &mut device.rsp.cpu.accm;
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];

    for n in 0..8 {
        let mut product = ((get_vpr16(*acch, n) as i32) << 16) | (get_vpr16(*accm, n) as i32);
        if product < 0 && (product & (1 << 5)) == 0 {
            product += 32
        } else if product >= 32 && (product & (1 << 5)) == 0 {
            product -= 32
        }
        modify_vpr16(acch, n, (product >> 16) as u16);
        modify_vpr16(accm, n, product as u16);
        modify_vpr16(vd_reg, n, (clamp_signed_32(product >> 1) & !15) as u16);
    }
}

pub fn vmadl(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let hi = _mm_mulhi_epu16(vs_reg, vte);

        let accl_old = device.rsp.cpu.accl;
        device.rsp.cpu.accl = _mm_add_epi16(accl_old, hi);
        let overflow_l = _mm_cmpeq_epi16(_mm_adds_epu16(accl_old, hi), device.rsp.cpu.accl);
        let borrow = _mm_cmpeq_epi16(overflow_l, _mm_setzero_si128());

        let accm_old = device.rsp.cpu.accm;
        device.rsp.cpu.accm = _mm_sub_epi16(accm_old, borrow);
        let overflow_m = _mm_cmpeq_epi16(
            _mm_adds_epu16(accm_old, _mm_sub_epi16(_mm_setzero_si128(), borrow)),
            device.rsp.cpu.accm,
        );
        let borrow_h = _mm_cmpeq_epi16(overflow_m, _mm_setzero_si128());

        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, borrow_h);

        let nhi = _mm_srai_epi16(device.rsp.cpu.acch, 15);
        let nmd = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        let shi = _mm_cmpeq_epi16(nhi, device.rsp.cpu.acch);
        let smd = _mm_cmpeq_epi16(nhi, nmd);
        let cmask = _mm_and_si128(smd, shi);
        let cval = _mm_cmpeq_epi16(nhi, _mm_setzero_si128());

        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_blendv_epi8(cval, device.rsp.cpu.accl, cmask);
    }
}

pub fn vmadm(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let lo = _mm_mullo_epi16(vs_reg, vte);
        let mut hi = _mm_mulhi_epu16(vs_reg, vte);
        let sign = _mm_srai_epi16(vs_reg, 15);
        let vta = _mm_and_si128(vte, sign);
        hi = _mm_sub_epi16(hi, vta);

        let accl_old = device.rsp.cpu.accl;
        device.rsp.cpu.accl = _mm_add_epi16(accl_old, lo);
        let overflow_l = _mm_cmpeq_epi16(_mm_adds_epu16(accl_old, lo), device.rsp.cpu.accl);
        let borrow = _mm_cmpeq_epi16(overflow_l, _mm_setzero_si128());

        hi = _mm_sub_epi16(hi, borrow);

        let accm_old = device.rsp.cpu.accm;
        device.rsp.cpu.accm = _mm_add_epi16(accm_old, hi);
        let overflow_m = _mm_cmpeq_epi16(_mm_adds_epu16(accm_old, hi), device.rsp.cpu.accm);
        let borrow_h = _mm_cmpeq_epi16(overflow_m, _mm_setzero_si128());

        let hi_sign = _mm_srai_epi16(hi, 15);
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi_sign);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, borrow_h);

        let lo_packed = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        let hi_packed = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_packs_epi32(lo_packed, hi_packed);
    }
}

pub fn vmadn(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let lo = _mm_mullo_epi16(vs_reg, vte);
        let mut hi = _mm_mulhi_epu16(vs_reg, vte);
        let sign = _mm_srai_epi16(vte, 15);
        let vsa = _mm_and_si128(vs_reg, sign);
        hi = _mm_sub_epi16(hi, vsa);

        let accl_old = device.rsp.cpu.accl;
        device.rsp.cpu.accl = _mm_add_epi16(accl_old, lo);
        let overflow_l = _mm_cmpeq_epi16(_mm_adds_epu16(accl_old, lo), device.rsp.cpu.accl);
        let borrow = _mm_cmpeq_epi16(overflow_l, _mm_setzero_si128());

        hi = _mm_sub_epi16(hi, borrow);

        let accm_old = device.rsp.cpu.accm;
        device.rsp.cpu.accm = _mm_add_epi16(accm_old, hi);
        let overflow_m = _mm_cmpeq_epi16(_mm_adds_epu16(accm_old, hi), device.rsp.cpu.accm);
        let borrow_h = _mm_cmpeq_epi16(overflow_m, _mm_setzero_si128());

        let hi_sign = _mm_srai_epi16(hi, 15);
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi_sign);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, borrow_h);

        let nhi = _mm_srai_epi16(device.rsp.cpu.acch, 15);
        let nmd = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        let shi = _mm_cmpeq_epi16(nhi, device.rsp.cpu.acch);
        let smd = _mm_cmpeq_epi16(nhi, nmd);
        let cmask = _mm_and_si128(smd, shi);
        let cval = _mm_cmpeq_epi16(nhi, _mm_setzero_si128());

        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_blendv_epi8(cval, device.rsp.cpu.accl, cmask);
    }
}

pub fn vmadh(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let lo = _mm_mullo_epi16(vs_reg, vte);
        let mut hi = _mm_mulhi_epi16(vs_reg, vte);

        let accm_old = device.rsp.cpu.accm;
        device.rsp.cpu.accm = _mm_add_epi16(accm_old, lo);
        let overflow = _mm_cmpeq_epi16(_mm_adds_epu16(accm_old, lo), device.rsp.cpu.accm);
        let borrow = _mm_cmpeq_epi16(overflow, _mm_setzero_si128());

        hi = _mm_sub_epi16(hi, borrow);
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi);

        let lo_packed = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        let hi_packed = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_packs_epi32(lo_packed, hi_packed);
    }
}

pub fn vadd(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let sum = _mm_add_epi16(vs_reg, vte);
        device.rsp.cpu.accl = _mm_sub_epi16(sum, device.rsp.cpu.vcol);

        let min = _mm_min_epi16(vs_reg, vte);
        let max = _mm_max_epi16(vs_reg, vte);
        let min_adjusted = _mm_subs_epi16(min, device.rsp.cpu.vcol);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_adds_epi16(min_adjusted, max);

        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
    }
}

pub fn vsub(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let udiff = _mm_sub_epi16(vte, device.rsp.cpu.vcol);
        let sdiff = _mm_subs_epi16(vte, device.rsp.cpu.vcol);
        device.rsp.cpu.accl = _mm_sub_epi16(vs_reg, udiff);

        let ov = _mm_cmpgt_epi16(sdiff, udiff);
        let sub_result = _mm_subs_epi16(vs_reg, sdiff);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_adds_epi16(sub_result, ov);

        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
    }
}

pub fn vzero(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_add_epi16(vs_reg, vte);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_setzero_si128();
    }
}

pub fn vabs(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let vs0 = _mm_cmpeq_epi16(vs_reg, _mm_setzero_si128());
        let slt = _mm_srai_epi16(vs_reg, 15);

        let mut result = _mm_andnot_si128(vs0, vte);
        result = _mm_xor_si128(result, slt);
        device.rsp.cpu.accl = _mm_sub_epi16(result, slt);
        device.rsp.cpu.vpr[vd(opcode) as usize] = _mm_subs_epi16(result, slt);
    }
}

pub fn vaddc(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let sum = _mm_adds_epu16(vs_reg, vte);
        device.rsp.cpu.accl = _mm_add_epi16(vs_reg, vte);
        device.rsp.cpu.vcol = _mm_cmpeq_epi16(
            _mm_cmpeq_epi16(sum, device.rsp.cpu.accl),
            _mm_setzero_si128(),
        );
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vsubc(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let udiff = _mm_subs_epu16(vs_reg, vte);
        let equal = _mm_cmpeq_epi16(vs_reg, vte);
        let diff0 = _mm_cmpeq_epi16(udiff, _mm_setzero_si128());

        device.rsp.cpu.vcoh = _mm_cmpeq_epi16(equal, _mm_setzero_si128());
        device.rsp.cpu.vcol = _mm_andnot_si128(equal, diff0);
        device.rsp.cpu.accl = _mm_sub_epi16(vs_reg, vte);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vsar(device: &mut device::Device, opcode: u32) {
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];
    match ve(opcode) {
        0x8 => {
            *vd_reg = device.rsp.cpu.acch;
        }
        0x9 => {
            *vd_reg = device.rsp.cpu.accm;
        }
        0xa => {
            *vd_reg = device.rsp.cpu.accl;
        }
        _ => {
            *vd_reg = unsafe { _mm_setzero_si128() };
        }
    }
}

pub fn vlt(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let eq = _mm_cmpeq_epi16(vs_reg, vte);
        let lt = _mm_cmplt_epi16(vs_reg, vte);
        let eq_and_carry =
            _mm_and_si128(_mm_and_si128(device.rsp.cpu.vcoh, eq), device.rsp.cpu.vcol);

        device.rsp.cpu.vccl = _mm_or_si128(lt, eq_and_carry);
        device.rsp.cpu.accl = _mm_blendv_epi8(vte, vs_reg, device.rsp.cpu.vccl);

        device.rsp.cpu.vcch = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn veq(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let eq = _mm_cmpeq_epi16(vs_reg, vte);
        device.rsp.cpu.vccl = _mm_andnot_si128(device.rsp.cpu.vcoh, eq);
        device.rsp.cpu.accl = _mm_blendv_epi8(vte, vs_reg, device.rsp.cpu.vccl);

        device.rsp.cpu.vcch = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vne(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let eq = _mm_cmpeq_epi16(vs_reg, vte);
        let ne = _mm_cmpeq_epi16(eq, _mm_setzero_si128());
        device.rsp.cpu.vccl = _mm_or_si128(_mm_and_si128(device.rsp.cpu.vcoh, eq), ne);
        device.rsp.cpu.accl = _mm_blendv_epi8(vte, vs_reg, device.rsp.cpu.vccl);

        device.rsp.cpu.vcch = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vge(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let eq = _mm_cmpeq_epi16(vs_reg, vte);
        let gt = _mm_cmpgt_epi16(vs_reg, vte);
        let es = _mm_and_si128(device.rsp.cpu.vcoh, device.rsp.cpu.vcol);
        let eq_filtered = _mm_andnot_si128(es, eq);

        device.rsp.cpu.vccl = _mm_or_si128(gt, eq_filtered);
        device.rsp.cpu.accl = _mm_blendv_epi8(vte, vs_reg, device.rsp.cpu.vccl);

        device.rsp.cpu.vcch = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vcl(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let mut nvt = _mm_xor_si128(vte, device.rsp.cpu.vcol);
        nvt = _mm_sub_epi16(nvt, device.rsp.cpu.vcol);
        let diff = _mm_sub_epi16(vs_reg, nvt);

        let ncarry = _mm_cmpeq_epi16(diff, _mm_adds_epu16(vs_reg, vte));
        let nvce = _mm_cmpeq_epi16(device.rsp.cpu.vce, _mm_setzero_si128());
        let diff0 = _mm_cmpeq_epi16(diff, _mm_setzero_si128());

        let lec1 = _mm_and_si128(_mm_and_si128(diff0, ncarry), nvce);
        let lec2 = _mm_and_si128(_mm_or_si128(diff0, ncarry), device.rsp.cpu.vce);
        let leeq = _mm_or_si128(lec1, lec2);

        let geeq = _mm_cmpeq_epi16(_mm_subs_epu16(vte, vs_reg), _mm_setzero_si128());

        let le_sel = _mm_andnot_si128(device.rsp.cpu.vcoh, device.rsp.cpu.vcol);
        let le = _mm_blendv_epi8(device.rsp.cpu.vccl, leeq, le_sel);

        let ge_sel = _mm_or_si128(device.rsp.cpu.vcol, device.rsp.cpu.vcoh);
        let ge = _mm_blendv_epi8(geeq, device.rsp.cpu.vcch, ge_sel);

        let mask = _mm_blendv_epi8(ge, le, device.rsp.cpu.vcol);
        device.rsp.cpu.accl = _mm_blendv_epi8(vs_reg, nvt, mask);

        device.rsp.cpu.vcch = ge;
        device.rsp.cpu.vccl = le;
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vce = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vch(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.vcol = _mm_cmplt_epi16(_mm_xor_si128(vs_reg, vte), _mm_setzero_si128());

        let mut nvt = _mm_xor_si128(vte, device.rsp.cpu.vcol);
        nvt = _mm_sub_epi16(nvt, device.rsp.cpu.vcol);
        let diff = _mm_sub_epi16(vs_reg, nvt);
        let diff0 = _mm_cmpeq_epi16(diff, _mm_setzero_si128());
        let vtn = _mm_cmplt_epi16(vte, _mm_setzero_si128());

        let dlez = _mm_cmpeq_epi16(
            _mm_setzero_si128(),
            _mm_cmpgt_epi16(diff, _mm_setzero_si128()),
        );
        let dgez = _mm_or_si128(_mm_cmpgt_epi16(diff, _mm_setzero_si128()), diff0);

        device.rsp.cpu.vcch = _mm_blendv_epi8(dgez, vtn, device.rsp.cpu.vcol);
        device.rsp.cpu.vccl = _mm_blendv_epi8(vtn, dlez, device.rsp.cpu.vcol);
        device.rsp.cpu.vce = _mm_and_si128(
            _mm_cmpeq_epi16(diff, device.rsp.cpu.vcol),
            device.rsp.cpu.vcol,
        );
        device.rsp.cpu.vcoh =
            _mm_cmpeq_epi16(_mm_or_si128(diff0, device.rsp.cpu.vce), _mm_setzero_si128());

        let mask = _mm_blendv_epi8(
            device.rsp.cpu.vcch,
            device.rsp.cpu.vccl,
            device.rsp.cpu.vcol,
        );
        device.rsp.cpu.accl = _mm_blendv_epi8(vs_reg, nvt, mask);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vcr(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let sign = _mm_srai_epi16(_mm_xor_si128(vs_reg, vte), 15);
        let dlez = _mm_add_epi16(_mm_and_si128(vs_reg, sign), vte);
        device.rsp.cpu.vccl = _mm_srai_epi16(dlez, 15);

        let dgez = _mm_min_epi16(_mm_or_si128(vs_reg, sign), vte);
        device.rsp.cpu.vcch = _mm_cmpeq_epi16(dgez, vte);

        let nvt = _mm_xor_si128(vte, sign);
        let mask = _mm_blendv_epi8(device.rsp.cpu.vcch, device.rsp.cpu.vccl, sign);
        device.rsp.cpu.accl = _mm_blendv_epi8(vs_reg, nvt, mask);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;

        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vce = _mm_setzero_si128();
    }
}

pub fn vmrg(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_blendv_epi8(vte, vs_reg, device.rsp.cpu.vccl);
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vand(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_and_si128(vs_reg, vte);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vnand(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let and_result = _mm_and_si128(vs_reg, vte);
        device.rsp.cpu.accl = _mm_xor_si128(and_result, _mm_set1_epi32(-1));
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_or_si128(vs_reg, vte);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vnor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let or_result = _mm_or_si128(vs_reg, vte);
        device.rsp.cpu.accl = _mm_xor_si128(or_result, _mm_set1_epi32(-1));
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vxor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        device.rsp.cpu.accl = _mm_xor_si128(vs_reg, vte);
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vnxor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vs_reg = device.rsp.cpu.vpr[vs(opcode) as usize];
    unsafe {
        let xor_result = _mm_xor_si128(vs_reg, vte);
        device.rsp.cpu.accl = _mm_xor_si128(xor_result, _mm_set1_epi32(-1));
        device.rsp.cpu.vpr[vd(opcode) as usize] = device.rsp.cpu.accl;
    }
}

pub fn vrcp(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input = get_vpr16(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32;
    let result = compute_reciprocal(input, &device.rsp.cpu.reciprocals);

    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr16(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrcpl(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input = if device.rsp.cpu.divdp {
        ((device.rsp.cpu.divin as i32) << 16)
            | get_vpr16(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as u16 as i32
    } else {
        get_vpr16(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32
    };
    let result = compute_reciprocal(input, &device.rsp.cpu.reciprocals);

    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr16(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrcph(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vt_reg = device.rsp.cpu.vpr[vt(opcode) as usize];
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];

    device.rsp.cpu.accl = vte;
    device.rsp.cpu.divdp = true;
    device.rsp.cpu.divin = get_vpr16(vt_reg, ve(opcode) as u8) as i16;

    modify_vpr16(vd_reg, de(opcode) as u8, device.rsp.cpu.divout as u16);
}

pub fn vmov(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];
    let de_index = de(opcode) as u8;

    let value = get_vpr16(vte, de_index);
    modify_vpr16(vd_reg, de_index, value);
    device.rsp.cpu.accl = vte;
}

pub fn vrsq(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input = get_vpr16(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32;
    let result = compute_inverse_sqrt(input, &device.rsp.cpu.inverse_square_roots);

    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr16(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrsql(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input = if device.rsp.cpu.divdp {
        ((device.rsp.cpu.divin as i32) << 16)
            | get_vpr16(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as u16 as i32
    } else {
        get_vpr16(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32
    };
    let result = compute_inverse_sqrt(input, &device.rsp.cpu.inverse_square_roots);

    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr16(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrsqh(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let vt_reg = device.rsp.cpu.vpr[vt(opcode) as usize];
    let vd_reg = &mut device.rsp.cpu.vpr[vd(opcode) as usize];

    device.rsp.cpu.accl = vte;
    device.rsp.cpu.divdp = true;
    device.rsp.cpu.divin = get_vpr16(vt_reg, ve(opcode) as u8) as i16;

    modify_vpr16(vd_reg, de(opcode) as u8, device.rsp.cpu.divout as u16);
}

pub fn vnop(_device: &mut device::Device, _opcode: u32) {}

pub fn execute_vec(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.instruction_type = device::rsp_cpu::InstructionType::Vu;
    device.rsp.cpu.vec_instrs[(opcode & 0x3F) as usize](device, opcode)
}

pub fn reserved(_device: &mut device::Device, _opcode: u32) {
    panic!("rsp vu reserved")
}
