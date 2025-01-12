#[cfg(target_arch = "aarch64")]
use device::__m128i;
#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));
#[cfg(target_arch = "aarch64")]
include!("../compat/aarch64.rs");
use crate::device;
use crate::device::rsp_su_instructions::{get_vpr_element, modify_vpr_element};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn vt(opcode: u32) -> u32 {
    (opcode >> 16) & 0x1F
}

pub fn ve(opcode: u32) -> u32 {
    (opcode >> 21) & 0xF
}

pub fn vs(opcode: u32) -> u32 {
    (opcode >> 11) & 0x1F
}

pub fn vd(opcode: u32) -> u32 {
    (opcode >> 6) & 0x1F
}

pub fn de(opcode: u32) -> u32 {
    (opcode >> 11) & 0x7
}

pub fn clamp_signed_32(value: i32) -> i16 {
    value.clamp(-32768, 32767) as i16
}

pub fn clamp_signed_64(value: i64) -> i16 {
    value.clamp(-32768, 32767) as i16
}

pub fn s_clip(x: i64, bits: u32) -> i64 {
    let b = 1_u64 << (bits - 1);
    let m = b * 2 - 1;
    ((((x as u64) & m) ^ b).wrapping_sub(b)) as i64
}

pub fn vte(device: &device::Device, vt: u32, index: usize) -> __m128i {
    unsafe {
        _mm_shuffle_epi8(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vt as usize]),
            device.rsp.cpu.shuffle[index],
        )
    }
}

pub fn vmulf(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut lo, mut hi, mut round, mut sign1, sign2, neq, eq, neg);
    unsafe {
        lo = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        round = _mm_cmpeq_epi16(_mm_setzero_si128(), _mm_setzero_si128());
        sign1 = _mm_srli_epi16(lo, 15);
        lo = _mm_add_epi16(lo, lo);
        round = _mm_slli_epi16(round, 15);
        hi = _mm_mulhi_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        sign2 = _mm_srli_epi16(lo, 15);
        device.rsp.cpu.accl = _mm_add_epi16(round, lo);
        sign1 = _mm_add_epi16(sign1, sign2);
        hi = _mm_slli_epi16(hi, 1);
        neq = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accm = _mm_add_epi16(hi, sign1);
        neg = _mm_srai_epi16(device.rsp.cpu.accm, 15);

        eq = _mm_and_si128(neq, neg);
        device.rsp.cpu.acch = _mm_andnot_si128(neq, neg);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_add_epi16(device.rsp.cpu.accm, eq));
    }
}

pub fn vmulu(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut lo, mut hi, mut round, mut sign1, sign2, neq, neg);
    unsafe {
        lo = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        round = _mm_cmpeq_epi16(_mm_setzero_si128(), _mm_setzero_si128());
        sign1 = _mm_srli_epi16(lo, 15);
        lo = _mm_add_epi16(lo, lo);
        round = _mm_slli_epi16(round, 15);
        hi = _mm_mulhi_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        sign2 = _mm_srli_epi16(lo, 15);
        device.rsp.cpu.accl = _mm_add_epi16(round, lo);
        sign1 = _mm_add_epi16(sign1, sign2);
        hi = _mm_slli_epi16(hi, 1);
        neq = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accm = _mm_add_epi16(hi, sign1);
        neg = _mm_srai_epi16(device.rsp.cpu.accm, 15);

        device.rsp.cpu.acch = _mm_andnot_si128(neq, neg);
        hi = _mm_or_si128(device.rsp.cpu.accm, neg);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_andnot_si128(device.rsp.cpu.acch, hi));
    }
}

pub fn vrndp(device: &mut device::Device, opcode: u32) {
    let vte = unsafe {
        std::mem::transmute::<__m128i, u128>(vte(device, vt(opcode), ve(opcode) as usize))
    };
    let acch: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.acch) };
    let accm: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.accm) };
    let accl: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.accl) };

    for n in 0..8 {
        let mut product = get_vpr_element(vte, n) as i16 as i32;
        if vs(opcode) & 1 != 0 {
            product <<= 16
        }
        let mut acc = 0;
        acc |= get_vpr_element(*acch, n) as i64;
        acc <<= 16;
        acc |= get_vpr_element(*accm, n) as i64;
        acc <<= 16;
        acc |= get_vpr_element(*accl, n) as i64;
        acc <<= 16;
        acc >>= 16;
        if acc >= 0 {
            acc = s_clip(acc + (product as i64), 48)
        }
        modify_vpr_element(acch, n, (acc >> 32) as u16);
        modify_vpr_element(accm, n, (acc >> 16) as u16);
        modify_vpr_element(accl, n, acc as u16);
        modify_vpr_element(
            &mut device.rsp.cpu.vpr[vd(opcode) as usize],
            n,
            clamp_signed_64(acc >> 16) as u16,
        );
    }
}

pub fn vmulq(device: &mut device::Device, opcode: u32) {
    let vte = unsafe {
        std::mem::transmute::<__m128i, u128>(vte(device, vt(opcode), ve(opcode) as usize))
    };
    let acch: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.acch) };
    let accm: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.accm) };
    let accl: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.accl) };

    for n in 0..8 {
        let mut product = (get_vpr_element(device.rsp.cpu.vpr[vs(opcode) as usize], n) as i16
            as i32)
            .wrapping_mul(get_vpr_element(vte, n) as i16 as i32);
        if product < 0 {
            product += 31;
        }
        modify_vpr_element(acch, n, (product >> 16) as u16);
        modify_vpr_element(accm, n, (product) as u16);
        modify_vpr_element(accl, n, 0);
        modify_vpr_element(
            &mut device.rsp.cpu.vpr[vd(opcode) as usize],
            n,
            (clamp_signed_32(product >> 1) & !15) as u16,
        );
    }
}

pub fn vmudl(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_mulhi_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accm = _mm_setzero_si128();
        device.rsp.cpu.acch = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vmudm(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (sign, vta);
    unsafe {
        device.rsp.cpu.accl = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accm = _mm_mulhi_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        sign = _mm_srai_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            15,
        );
        vta = _mm_and_si128(vte, sign);
        device.rsp.cpu.accm = _mm_sub_epi16(device.rsp.cpu.accm, vta);
        device.rsp.cpu.acch = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accm);
    }
}

pub fn vmudn(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (sign, vsa);
    unsafe {
        device.rsp.cpu.accl = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accm = _mm_mulhi_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        sign = _mm_srai_epi16(vte, 15);
        vsa = _mm_and_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            sign,
        );
        device.rsp.cpu.accm = _mm_sub_epi16(device.rsp.cpu.accm, vsa);
        device.rsp.cpu.acch = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vmudh(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (lo, hi);
    unsafe {
        device.rsp.cpu.accl = _mm_setzero_si128();
        device.rsp.cpu.accm = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.acch = _mm_mulhi_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        lo = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        hi = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_packs_epi32(lo, hi));
    }
}

pub fn vmacf(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut lo, mut md, mut hi, mut carry, mut omask);
    unsafe {
        lo = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        hi = _mm_mulhi_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        md = _mm_slli_epi16(hi, 1);
        carry = _mm_srli_epi16(lo, 15);
        hi = _mm_srai_epi16(hi, 15);
        md = _mm_or_si128(md, carry);
        lo = _mm_slli_epi16(lo, 1);
        omask = _mm_adds_epu16(device.rsp.cpu.accl, lo);
        device.rsp.cpu.accl = _mm_add_epi16(device.rsp.cpu.accl, lo);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accl, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        md = _mm_sub_epi16(md, omask);
        carry = _mm_cmpeq_epi16(md, _mm_setzero_si128());
        carry = _mm_and_si128(carry, omask);
        hi = _mm_sub_epi16(hi, carry);
        omask = _mm_adds_epu16(device.rsp.cpu.accm, md);
        device.rsp.cpu.accm = _mm_add_epi16(device.rsp.cpu.accm, md);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accm, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, omask);

        lo = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        hi = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_packs_epi32(lo, hi));
    }
}

pub fn vmacu(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut lo, mut md, mut hi, mut carry, mut omask, mmask, hmask);
    unsafe {
        lo = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        hi = _mm_mulhi_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        md = _mm_slli_epi16(hi, 1);
        carry = _mm_srli_epi16(lo, 15);
        hi = _mm_srai_epi16(hi, 15);
        md = _mm_or_si128(md, carry);
        lo = _mm_slli_epi16(lo, 1);
        omask = _mm_adds_epu16(device.rsp.cpu.accl, lo);
        device.rsp.cpu.accl = _mm_add_epi16(device.rsp.cpu.accl, lo);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accl, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        md = _mm_sub_epi16(md, omask);
        carry = _mm_cmpeq_epi16(md, _mm_setzero_si128());
        carry = _mm_and_si128(carry, omask);
        hi = _mm_sub_epi16(hi, carry);
        omask = _mm_adds_epu16(device.rsp.cpu.accm, md);
        device.rsp.cpu.accm = _mm_add_epi16(device.rsp.cpu.accm, md);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accm, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, omask);

        mmask = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        hmask = _mm_srai_epi16(device.rsp.cpu.acch, 15);
        md = _mm_or_si128(mmask, device.rsp.cpu.accm);
        omask = _mm_cmpgt_epi16(device.rsp.cpu.acch, _mm_setzero_si128());
        md = _mm_andnot_si128(hmask, md);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_or_si128(omask, md));
    }
}

pub fn vrndn(device: &mut device::Device, opcode: u32) {
    let vte = unsafe {
        std::mem::transmute::<__m128i, u128>(vte(device, vt(opcode), ve(opcode) as usize))
    };
    let acch: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.acch) };
    let accm: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.accm) };
    let accl: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.accl) };

    for n in 0..8 {
        let mut product = get_vpr_element(vte, n) as i16 as i32;
        if vs(opcode) & 1 != 0 {
            product <<= 16
        }
        let mut acc = 0;
        acc |= get_vpr_element(*acch, n) as i64;
        acc <<= 16;
        acc |= get_vpr_element(*accm, n) as i64;
        acc <<= 16;
        acc |= get_vpr_element(*accl, n) as i64;
        acc <<= 16;
        acc >>= 16;
        if acc < 0 {
            acc = s_clip(acc + (product as i64), 48)
        }
        modify_vpr_element(acch, n, (acc >> 32) as u16);
        modify_vpr_element(accm, n, (acc >> 16) as u16);
        modify_vpr_element(accl, n, acc as u16);
        modify_vpr_element(
            &mut device.rsp.cpu.vpr[vd(opcode) as usize],
            n,
            clamp_signed_64(acc >> 16) as u16,
        );
    }
}

pub fn vmacq(device: &mut device::Device, opcode: u32) {
    let acch: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.acch) };
    let accm: &mut u128 = unsafe { std::mem::transmute(&mut device.rsp.cpu.accm) };

    for n in 0..8 {
        let mut product =
            ((get_vpr_element(*acch, n) as i32) << 16) | (get_vpr_element(*accm, n) as i32);
        if product < 0 && (product & (1 << 5)) == 0 {
            product += 32
        } else if product >= 32 && (product & (1 << 5)) == 0 {
            product -= 32
        }
        modify_vpr_element(acch, n, (product >> 16) as u16);
        modify_vpr_element(accm, n, (product) as u16);
        modify_vpr_element(
            &mut device.rsp.cpu.vpr[vd(opcode) as usize],
            n,
            (clamp_signed_32(product >> 1) & !15) as u16,
        );
    }
}

pub fn vmadl(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut hi, mut omask, nhi, nmd, shi, smd, cmask, cval);
    unsafe {
        hi = _mm_mulhi_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        omask = _mm_adds_epu16(device.rsp.cpu.accl, hi);
        device.rsp.cpu.accl = _mm_add_epi16(device.rsp.cpu.accl, hi);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accl, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        hi = _mm_sub_epi16(_mm_setzero_si128(), omask);
        omask = _mm_adds_epu16(device.rsp.cpu.accm, hi);
        device.rsp.cpu.accm = _mm_add_epi16(device.rsp.cpu.accm, hi);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accm, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, omask);
        nhi = _mm_srai_epi16(device.rsp.cpu.acch, 15);
        nmd = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        shi = _mm_cmpeq_epi16(nhi, device.rsp.cpu.acch);
        smd = _mm_cmpeq_epi16(nhi, nmd);
        cmask = _mm_and_si128(smd, shi);
        cval = _mm_cmpeq_epi16(nhi, _mm_setzero_si128());
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_blendv_epi8(cval, device.rsp.cpu.accl, cmask));
    }
}

pub fn vmadm(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut lo, mut hi, sign, vta, mut omask);
    unsafe {
        lo = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        hi = _mm_mulhi_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        sign = _mm_srai_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            15,
        );
        vta = _mm_and_si128(vte, sign);
        hi = _mm_sub_epi16(hi, vta);
        omask = _mm_adds_epu16(device.rsp.cpu.accl, lo);
        device.rsp.cpu.accl = _mm_add_epi16(device.rsp.cpu.accl, lo);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accl, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        hi = _mm_sub_epi16(hi, omask);
        omask = _mm_adds_epu16(device.rsp.cpu.accm, hi);
        device.rsp.cpu.accm = _mm_add_epi16(device.rsp.cpu.accm, hi);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accm, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        hi = _mm_srai_epi16(hi, 15);
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, omask);
        lo = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        hi = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_packs_epi32(lo, hi));
    }
}

pub fn vmadn(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (lo, mut hi, sign, vsa, mut omask, nhi, nmd, shi, smd, cmask, cval);
    unsafe {
        lo = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        hi = _mm_mulhi_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        sign = _mm_srai_epi16(vte, 15);
        vsa = _mm_and_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            sign,
        );
        hi = _mm_sub_epi16(hi, vsa);
        omask = _mm_adds_epu16(device.rsp.cpu.accl, lo);
        device.rsp.cpu.accl = _mm_add_epi16(device.rsp.cpu.accl, lo);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accl, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        hi = _mm_sub_epi16(hi, omask);
        omask = _mm_adds_epu16(device.rsp.cpu.accm, hi);
        device.rsp.cpu.accm = _mm_add_epi16(device.rsp.cpu.accm, hi);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accm, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        hi = _mm_srai_epi16(hi, 15);
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi);
        device.rsp.cpu.acch = _mm_sub_epi16(device.rsp.cpu.acch, omask);
        nhi = _mm_srai_epi16(device.rsp.cpu.acch, 15);
        nmd = _mm_srai_epi16(device.rsp.cpu.accm, 15);
        shi = _mm_cmpeq_epi16(nhi, device.rsp.cpu.acch);
        smd = _mm_cmpeq_epi16(nhi, nmd);
        cmask = _mm_and_si128(smd, shi);
        cval = _mm_cmpeq_epi16(nhi, _mm_setzero_si128());
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_blendv_epi8(cval, device.rsp.cpu.accl, cmask));
    }
}

pub fn vmadh(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut lo, mut hi, mut omask);
    unsafe {
        lo = _mm_mullo_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        hi = _mm_mulhi_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        omask = _mm_adds_epu16(device.rsp.cpu.accm, lo);
        device.rsp.cpu.accm = _mm_add_epi16(device.rsp.cpu.accm, lo);
        omask = _mm_cmpeq_epi16(device.rsp.cpu.accm, omask);
        omask = _mm_cmpeq_epi16(omask, _mm_setzero_si128());
        hi = _mm_sub_epi16(hi, omask);
        device.rsp.cpu.acch = _mm_add_epi16(device.rsp.cpu.acch, hi);
        lo = _mm_unpacklo_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        hi = _mm_unpackhi_epi16(device.rsp.cpu.accm, device.rsp.cpu.acch);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_packs_epi32(lo, hi));
    }
}

pub fn vadd(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (sum, mut min, max);
    unsafe {
        sum = _mm_add_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accl = _mm_sub_epi16(sum, device.rsp.cpu.vcol);
        min = _mm_min_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        max = _mm_max_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        min = _mm_subs_epi16(min, device.rsp.cpu.vcol);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_adds_epi16(min, max));
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
    }
}

pub fn vsub(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (udiff, sdiff, ov);
    unsafe {
        udiff = _mm_sub_epi16(vte, device.rsp.cpu.vcol);
        sdiff = _mm_subs_epi16(vte, device.rsp.cpu.vcol);
        device.rsp.cpu.accl = _mm_sub_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            udiff,
        );
        ov = _mm_cmpgt_epi16(sdiff, udiff);
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_subs_epi16(
                std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
                sdiff,
            ));
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_adds_epi16(
                std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vd(opcode) as usize]),
                ov,
            ));
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
    }
}

pub fn vzero(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_add_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_xor_si128(
                std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vd(opcode) as usize]),
                std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vd(opcode) as usize]),
            ));
    }
}

pub fn vabs(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (vs0, slt);
    unsafe {
        vs0 = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            _mm_setzero_si128(),
        );
        slt = _mm_srai_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            15,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_andnot_si128(vs0, vte));
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_xor_si128(
                std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vd(opcode) as usize]),
                slt,
            ));
        device.rsp.cpu.accl = _mm_sub_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vd(opcode) as usize]),
            slt,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(_mm_subs_epi16(
                std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vd(opcode) as usize]),
                slt,
            ));
    }
}

pub fn vaddc(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let sum;
    unsafe {
        sum = _mm_adds_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accl = _mm_add_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vcol = _mm_cmpeq_epi16(sum, device.rsp.cpu.accl);
        device.rsp.cpu.vcol = _mm_cmpeq_epi16(device.rsp.cpu.vcol, _mm_setzero_si128());
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vsubc(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (equal, udiff, diff0);
    unsafe {
        udiff = _mm_subs_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        equal = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        diff0 = _mm_cmpeq_epi16(udiff, _mm_setzero_si128());
        device.rsp.cpu.vcoh = _mm_cmpeq_epi16(equal, _mm_setzero_si128());
        device.rsp.cpu.vcol = _mm_andnot_si128(equal, diff0);
        device.rsp.cpu.accl = _mm_sub_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vsar(device: &mut device::Device, opcode: u32) {
    match ve(opcode) {
        0x8 => {
            device.rsp.cpu.vpr[vd(opcode) as usize] =
                unsafe { std::mem::transmute::<__m128i, u128>(device.rsp.cpu.acch) };
        }
        0x9 => {
            device.rsp.cpu.vpr[vd(opcode) as usize] =
                unsafe { std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accm) };
        }
        0xa => {
            device.rsp.cpu.vpr[vd(opcode) as usize] =
                unsafe { std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl) };
        }
        _ => {
            device.rsp.cpu.vpr[vd(opcode) as usize] = 0;
        }
    }
}

pub fn vlt(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut eq, lt);
    unsafe {
        eq = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        lt = _mm_cmplt_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        eq = _mm_and_si128(device.rsp.cpu.vcoh, eq);
        eq = _mm_and_si128(device.rsp.cpu.vcol, eq);
        device.rsp.cpu.vccl = _mm_or_si128(lt, eq);
        device.rsp.cpu.accl = _mm_blendv_epi8(
            vte,
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            device.rsp.cpu.vccl,
        );
        device.rsp.cpu.vcch = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn veq(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let eq;
    unsafe {
        eq = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vccl = _mm_andnot_si128(device.rsp.cpu.vcoh, eq);
        device.rsp.cpu.accl = _mm_blendv_epi8(
            vte,
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            device.rsp.cpu.vccl,
        );
        device.rsp.cpu.vcch = _mm_setzero_si128(); //unverified
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vne(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (eq, ne);
    unsafe {
        eq = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        ne = _mm_cmpeq_epi16(eq, _mm_setzero_si128());
        device.rsp.cpu.vccl = _mm_and_si128(device.rsp.cpu.vcoh, eq);
        device.rsp.cpu.vccl = _mm_or_si128(device.rsp.cpu.vccl, ne);
        device.rsp.cpu.accl = _mm_blendv_epi8(
            vte,
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            device.rsp.cpu.vccl,
        );
        device.rsp.cpu.vcch = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vge(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut eq, gt, es);
    unsafe {
        eq = _mm_cmpeq_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        gt = _mm_cmpgt_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        es = _mm_and_si128(device.rsp.cpu.vcoh, device.rsp.cpu.vcol);
        eq = _mm_andnot_si128(es, eq);
        device.rsp.cpu.vccl = _mm_or_si128(gt, eq);
        device.rsp.cpu.accl = _mm_blendv_epi8(
            vte,
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            device.rsp.cpu.vccl,
        );
        device.rsp.cpu.vcch = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vcl(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (
        mut nvt,
        diff,
        mut ncarry,
        nvce,
        diff0,
        mut lec1,
        mut lec2,
        leeq,
        mut geeq,
        mut le,
        mut ge,
        mask,
    );
    unsafe {
        nvt = _mm_xor_si128(vte, device.rsp.cpu.vcol);
        nvt = _mm_sub_epi16(nvt, device.rsp.cpu.vcol);
        diff = _mm_sub_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            nvt,
        );
        ncarry = _mm_adds_epu16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        ncarry = _mm_cmpeq_epi16(diff, ncarry);
        nvce = _mm_cmpeq_epi16(device.rsp.cpu.vce, _mm_setzero_si128());
        diff0 = _mm_cmpeq_epi16(diff, _mm_setzero_si128());
        lec1 = _mm_and_si128(diff0, ncarry);
        lec1 = _mm_and_si128(nvce, lec1);
        lec2 = _mm_or_si128(diff0, ncarry);
        lec2 = _mm_and_si128(device.rsp.cpu.vce, lec2);
        leeq = _mm_or_si128(lec1, lec2);
        geeq = _mm_subs_epu16(
            vte,
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
        );
        geeq = _mm_cmpeq_epi16(geeq, _mm_setzero_si128());
        le = _mm_andnot_si128(device.rsp.cpu.vcoh, device.rsp.cpu.vcol);
        le = _mm_blendv_epi8(device.rsp.cpu.vccl, leeq, le);
        ge = _mm_or_si128(device.rsp.cpu.vcol, device.rsp.cpu.vcoh);
        ge = _mm_blendv_epi8(geeq, device.rsp.cpu.vcch, ge);
        mask = _mm_blendv_epi8(ge, le, device.rsp.cpu.vcol);
        device.rsp.cpu.accl = _mm_blendv_epi8(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            nvt,
            mask,
        );
        device.rsp.cpu.vcch = ge;
        device.rsp.cpu.vccl = le;
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vce = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vch(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut nvt, diff, diff0, vtn, mut dlez, dgez, mask);
    unsafe {
        device.rsp.cpu.vcol = _mm_xor_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vcol = _mm_cmplt_epi16(device.rsp.cpu.vcol, _mm_setzero_si128());
        nvt = _mm_xor_si128(vte, device.rsp.cpu.vcol);
        nvt = _mm_sub_epi16(nvt, device.rsp.cpu.vcol);
        diff = _mm_sub_epi16(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            nvt,
        );
        diff0 = _mm_cmpeq_epi16(diff, _mm_setzero_si128());
        vtn = _mm_cmplt_epi16(vte, _mm_setzero_si128());
        dlez = _mm_cmpgt_epi16(diff, _mm_setzero_si128());
        dgez = _mm_or_si128(dlez, diff0);
        dlez = _mm_cmpeq_epi16(_mm_setzero_si128(), dlez);
        device.rsp.cpu.vcch = _mm_blendv_epi8(dgez, vtn, device.rsp.cpu.vcol);
        device.rsp.cpu.vccl = _mm_blendv_epi8(vtn, dlez, device.rsp.cpu.vcol);
        device.rsp.cpu.vce = _mm_cmpeq_epi16(diff, device.rsp.cpu.vcol);
        device.rsp.cpu.vce = _mm_and_si128(device.rsp.cpu.vce, device.rsp.cpu.vcol);
        device.rsp.cpu.vcoh = _mm_or_si128(diff0, device.rsp.cpu.vce);
        device.rsp.cpu.vcoh = _mm_cmpeq_epi16(device.rsp.cpu.vcoh, _mm_setzero_si128());
        mask = _mm_blendv_epi8(
            device.rsp.cpu.vcch,
            device.rsp.cpu.vccl,
            device.rsp.cpu.vcol,
        );
        device.rsp.cpu.accl = _mm_blendv_epi8(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            nvt,
            mask,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vcr(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let (mut sign, mut dlez, mut dgez, nvt, mask);
    unsafe {
        sign = _mm_xor_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        sign = _mm_srai_epi16(sign, 15);
        dlez = _mm_and_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            sign,
        );
        dlez = _mm_add_epi16(dlez, vte);
        device.rsp.cpu.vccl = _mm_srai_epi16(dlez, 15);
        dgez = _mm_or_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            sign,
        );
        dgez = _mm_min_epi16(dgez, vte);
        device.rsp.cpu.vcch = _mm_cmpeq_epi16(dgez, vte);
        nvt = _mm_xor_si128(vte, sign);
        mask = _mm_blendv_epi8(device.rsp.cpu.vcch, device.rsp.cpu.vccl, sign);
        device.rsp.cpu.accl = _mm_blendv_epi8(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            nvt,
            mask,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vce = _mm_setzero_si128();
    }
}

pub fn vmrg(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_blendv_epi8(
            vte,
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            device.rsp.cpu.vccl,
        );
        device.rsp.cpu.vcoh = _mm_setzero_si128();
        device.rsp.cpu.vcol = _mm_setzero_si128();
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vand(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_and_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vnand(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_and_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accl = _mm_xor_si128(device.rsp.cpu.accl, _mm_set1_epi32(-1));
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_or_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vnor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_or_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accl = _mm_xor_si128(device.rsp.cpu.accl, _mm_set1_epi32(-1));
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vxor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_xor_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vnxor(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    unsafe {
        device.rsp.cpu.accl = _mm_xor_si128(
            std::mem::transmute::<u128, __m128i>(device.rsp.cpu.vpr[vs(opcode) as usize]),
            vte,
        );
        device.rsp.cpu.accl = _mm_xor_si128(device.rsp.cpu.accl, _mm_set1_epi32(-1));
        device.rsp.cpu.vpr[vd(opcode) as usize] =
            std::mem::transmute::<__m128i, u128>(device.rsp.cpu.accl);
    }
}

pub fn vrcp(device: &mut device::Device, opcode: u32) {
    let mut result;
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input =
        get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32;
    let mask = input >> 31;
    let mut data = input ^ mask;
    if input > -32768 {
        data -= mask
    }
    if data == 0 {
        result = 0x7fffffff
    } else if input == -32768 {
        result = 0xffff0000
    } else {
        let shift = (data as u32).leading_zeros();
        let index = (((data as u64) << shift) & 0x7fc00000) >> 22;
        result = device.rsp.cpu.reciprocals[index as usize] as u32;
        result = (0x10000 | result) << 14;
        result = (result >> (31 - shift)) ^ mask as u32
    }
    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr_element(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrcpl(device: &mut device::Device, opcode: u32) {
    let mut result;
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input = if device.rsp.cpu.divdp {
        ((device.rsp.cpu.divin as i32) << 16)
            | get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as u16
                as i32
    } else {
        get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32
    };
    let mask = input >> 31;
    let mut data = input ^ mask;
    if input > -32768 {
        data -= mask
    }
    if data == 0 {
        result = 0x7fffffff
    } else if input == -32768 {
        result = 0xffff0000
    } else {
        let shift = (data as u32).leading_zeros();
        let index = (((data as u64) << shift) & 0x7fc00000) >> 22;
        result = device.rsp.cpu.reciprocals[index as usize] as u32;
        result = (0x10000 | result) << 14;
        result = (result >> (31 - shift)) ^ mask as u32
    }
    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr_element(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrcph(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    device.rsp.cpu.accl = vte;
    device.rsp.cpu.divdp = true;

    device.rsp.cpu.divin =
        get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16;
    modify_vpr_element(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        device.rsp.cpu.divout as u16,
    );
}

pub fn vmov(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let value = get_vpr_element(
        unsafe { std::mem::transmute::<__m128i, u128>(vte) },
        de(opcode) as u8,
    );
    modify_vpr_element(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        value,
    );
    device.rsp.cpu.accl = vte;
}

pub fn vrsq(device: &mut device::Device, opcode: u32) {
    let mut result;
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input =
        get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32;
    let mask = input >> 31;
    let mut data = input ^ mask;
    if input > -32768 {
        data -= mask
    }
    if data == 0 {
        result = 0x7fffffff
    } else if input == -32768 {
        result = 0xffff0000
    } else {
        let shift = (data as u32).leading_zeros();
        let index = (((data as u64) << shift) & 0x7fc00000) as u32 >> 22;
        result =
            (device.rsp.cpu.inverse_square_roots[((index & 0x1fe) | (shift & 1)) as usize]) as u32;
        result = (0x10000 | result) << 14;
        result = (result >> ((31 - shift) >> 1)) ^ mask as u32
    }
    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr_element(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrsql(device: &mut device::Device, opcode: u32) {
    let mut result;
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    let input = if device.rsp.cpu.divdp {
        ((device.rsp.cpu.divin as i32) << 16)
            | get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as u16
                as i32
    } else {
        get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16 as i32
    };
    let mask = input >> 31;
    let mut data = input ^ mask;
    if input > -32768 {
        data -= mask
    }
    if data == 0 {
        result = 0x7fffffff
    } else if input == -32768 {
        result = 0xffff0000
    } else {
        let shift = (data as u32).leading_zeros();
        let index = (((data as u64) << shift) & 0x7fc00000) as u32 >> 22;
        result =
            (device.rsp.cpu.inverse_square_roots[((index & 0x1fe) | (shift & 1)) as usize]) as u32;
        result = (0x10000 | result) << 14;
        result = (result >> ((31 - shift) >> 1)) ^ mask as u32
    }
    device.rsp.cpu.divdp = false;
    device.rsp.cpu.divout = (result >> 16) as i16;
    device.rsp.cpu.accl = vte;
    modify_vpr_element(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        result as u16,
    );
}

pub fn vrsqh(device: &mut device::Device, opcode: u32) {
    let vte = vte(device, vt(opcode), ve(opcode) as usize);
    device.rsp.cpu.accl = vte;
    device.rsp.cpu.divdp = true;

    device.rsp.cpu.divin =
        get_vpr_element(device.rsp.cpu.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16;
    modify_vpr_element(
        &mut device.rsp.cpu.vpr[vd(opcode) as usize],
        de(opcode) as u8,
        device.rsp.cpu.divout as u16,
    );
}

pub fn vnop(_device: &mut device::Device, _opcode: u32) {}

pub fn execute_vec(device: &mut device::Device, opcode: u32) {
    device.rsp.cpu.instruction_type = device::rsp_cpu::InstructionType::Vu;
    device.rsp.cpu.vec_instrs[(opcode & 0x3F) as usize](device, opcode)
}

pub fn reserved(_device: &mut device::Device, _opcode: u32) {
    panic!("rsp vu reserved")
}
