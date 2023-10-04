use crate::device;

pub struct BranchState {
    pub state: device::cpu::State,
    pub pc: u32,
}

pub struct Cpu {
    pub branch_state: BranchState,
    pub broken: bool,
    pub halted: bool,
    pub sync_point: bool,
    pub gpr: [u32; 32],
    pub vpr: [u128; 32],
    pub reciprocals: [u16; 512],
    pub inverse_square_roots: [u16; 512],
    pub vcol: std::arch::x86_64::__m128i,
    pub vcoh: std::arch::x86_64::__m128i,
    pub vccl: std::arch::x86_64::__m128i,
    pub vcch: std::arch::x86_64::__m128i,
    pub vce: std::arch::x86_64::__m128i,
    pub accl: std::arch::x86_64::__m128i,
    pub accm: std::arch::x86_64::__m128i,
    pub acch: std::arch::x86_64::__m128i,
    pub divdp: bool,
    pub divin: i16,
    pub divout: i16,
    pub special_instrs: [fn(&mut device::Device, u32); 48],
    pub regimm_instrs: [fn(&mut device::Device, u32); 18],
    pub cop0_instrs: [fn(&mut device::Device, u32); 5],
    pub cop2_instrs: [fn(&mut device::Device, u32); 32],
    pub lwc2_instrs: [fn(&mut device::Device, u32); 12],
    pub swc2_instrs: [fn(&mut device::Device, u32); 12],
    pub instrs: [fn(&mut device::Device, u32); 64],
    pub vec_instrs: [fn(&mut device::Device, u32); 64],
}

pub fn in_delay_slot(device: &mut device::Device) -> bool {
    return device.rsp.cpu.branch_state.state == device::cpu::State::DelaySlotTaken
        || device.rsp.cpu.branch_state.state == device::cpu::State::DelaySlotNotTaken;
}

pub fn in_delay_slot_taken(device: &mut device::Device) -> bool {
    return device.rsp.cpu.branch_state.state == device::cpu::State::DelaySlotTaken;
}

pub fn run(device: &mut device::Device) -> u64 {
    device.rsp.cpu.broken = false;
    let mut cycle_counter = 0;
    while !device.rsp.cpu.halted && !device.rsp.cpu.sync_point {
        device.rsp.cpu.gpr[0] = 0; // gpr 0 is read only
        let offset = 0x1000 + device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] as usize;
        let opcode = u32::from_be_bytes(device.rsp.mem[offset..offset + 4].try_into().unwrap());
        execute_opcode(device, opcode)(device, opcode);
        match device.rsp.cpu.branch_state.state {
            device::cpu::State::Step => {
                device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] += 4
            }
            device::cpu::State::Take => {
                device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] += 4;
                device.rsp.cpu.branch_state.state = device::cpu::State::DelaySlotTaken
            }
            device::cpu::State::NotTaken => {
                device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] += 4;
                device.rsp.cpu.branch_state.state = device::cpu::State::DelaySlotNotTaken
            }
            device::cpu::State::DelaySlotTaken => {
                device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] =
                    device.rsp.cpu.branch_state.pc;
                device.rsp.cpu.branch_state.state = device::cpu::State::Step
            }
            device::cpu::State::DelaySlotNotTaken => {
                device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] += 4;
                device.rsp.cpu.branch_state.state = device::cpu::State::Step
            }
            device::cpu::State::Discard => {
                device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] += 8;
                device.rsp.cpu.branch_state.state = device::cpu::State::Step
            }
            device::cpu::State::Exception => {
                device.rsp.cpu.branch_state.state = device::cpu::State::Step
            }
        }
        device.rsp.regs2[device::rsp_interface::SP_PC_REG as usize] &= 0xFFC;

        cycle_counter += 1;

        if device.rsp.cpu.broken && device.rsp.cpu.branch_state.state == device::cpu::State::Step {
            break;
        }
    }
    return (cycle_counter as f64 * 1.5) as u64; // converting RCP clock to CPU clock
}

pub fn execute_opcode(device: &mut device::Device, opcode: u32) -> fn(&mut device::Device, u32) {
    match opcode >> 26 {
        0 => {
            // SPECIAL
            return device.rsp.cpu.special_instrs[(opcode & 0x3F) as usize];
        }
        1 => {
            // REGIMM
            return device.rsp.cpu.regimm_instrs[((opcode >> 16) & 0x1F) as usize];
        }
        16 => {
            // COP0
            return device.rsp.cpu.cop0_instrs[((opcode >> 21) & 0x1F) as usize];
        }
        18 => {
            // COP2
            return device.rsp.cpu.cop2_instrs[((opcode >> 21) & 0x1F) as usize];
        }
        50 => {
            // LWC2
            return device.rsp.cpu.lwc2_instrs[((opcode >> 11) & 0x1F) as usize];
        }
        58 => {
            // SWC2
            return device.rsp.cpu.swc2_instrs[((opcode >> 11) & 0x1F) as usize];
        }
        _ => return device.rsp.cpu.instrs[(opcode >> 26) as usize],
    }
}

pub fn init(device: &mut device::Device) {
    device.rsp.cpu.reciprocals[0] = !0 as u16;
    let mut index = 1;
    while index < 512 {
        let a = (index + 512) as u64;
        let b = ((1 as u64) << 34) / a;
        device.rsp.cpu.reciprocals[index] = ((b + 1) >> 8) as u16;
        index += 1;
    }

    index = 0;
    while index < 512 {
        let mut shift = 0;
        if index % 2 == 1 {
            shift = 1
        }
        let a = ((index + 512) >> shift) as u64;
        let mut b = (1 << 17) as u64;
        //find the largest b where b < 1.0 / sqrt(a)
        while a * (b + 1) * (b + 1) < ((1 as u64) << 44) {
            b += 1;
        }
        device.rsp.cpu.inverse_square_roots[index] = (b >> 1) as u16;
        index += 1;
    }

    device.rsp.cpu.instrs = [
        device::rsp_su_instructions::reserved, // SPECIAL
        device::rsp_su_instructions::reserved, // REGIMM
        device::rsp_su_instructions::j,        // 2
        device::rsp_su_instructions::jal,      // 3
        device::rsp_su_instructions::beq,      // 4
        device::rsp_su_instructions::bne,      // 5
        device::rsp_su_instructions::blez,     // 6
        device::rsp_su_instructions::bgtz,     // 7
        device::rsp_su_instructions::addi,     // 8
        device::rsp_su_instructions::addiu,    // 9
        device::rsp_su_instructions::slti,     // 10
        device::rsp_su_instructions::sltiu,    // 11
        device::rsp_su_instructions::andi,     // 12
        device::rsp_su_instructions::ori,      // 13
        device::rsp_su_instructions::xori,     // 14
        device::rsp_su_instructions::lui,      // 15
        device::rsp_su_instructions::reserved, // COP0
        device::rsp_su_instructions::reserved, // COP1
        device::rsp_su_instructions::reserved, // COP2
        device::rsp_su_instructions::reserved, // 19
        device::rsp_su_instructions::reserved, // 20
        device::rsp_su_instructions::reserved, // 21
        device::rsp_su_instructions::reserved, // 22
        device::rsp_su_instructions::reserved, // 23
        device::rsp_su_instructions::reserved, // 24
        device::rsp_su_instructions::reserved, // 25
        device::rsp_su_instructions::reserved, // 26
        device::rsp_su_instructions::reserved, // 27
        device::rsp_su_instructions::reserved, // 28
        device::rsp_su_instructions::reserved, // 29
        device::rsp_su_instructions::reserved, // 30
        device::rsp_su_instructions::reserved, // 31
        device::rsp_su_instructions::lb,       // 32
        device::rsp_su_instructions::lh,       // 33
        device::rsp_su_instructions::reserved, // 34
        device::rsp_su_instructions::lw,       // 35
        device::rsp_su_instructions::lbu,      // 36
        device::rsp_su_instructions::lhu,      // 37
        device::rsp_su_instructions::reserved, // 38
        device::rsp_su_instructions::lwu,      // 39
        device::rsp_su_instructions::sb,       // 40
        device::rsp_su_instructions::sh,       // 41
        device::rsp_su_instructions::reserved, // 42
        device::rsp_su_instructions::sw,       // 43
        device::rsp_su_instructions::reserved, // 44
        device::rsp_su_instructions::reserved, // 45
        device::rsp_su_instructions::reserved, // 46
        device::rsp_su_instructions::reserved, // 47
        device::rsp_su_instructions::reserved, // 48
        device::rsp_su_instructions::reserved, // 49
        device::rsp_su_instructions::reserved, // lwc2
        device::rsp_su_instructions::reserved, // 51
        device::rsp_su_instructions::reserved, // 52
        device::rsp_su_instructions::reserved, // 53
        device::rsp_su_instructions::reserved, // 54
        device::rsp_su_instructions::reserved, // 55
        device::rsp_su_instructions::reserved, // 56
        device::rsp_su_instructions::reserved, // 57
        device::rsp_su_instructions::reserved, // swc2
        device::rsp_su_instructions::reserved, // 59
        device::rsp_su_instructions::reserved, // 60
        device::rsp_su_instructions::reserved, // 61
        device::rsp_su_instructions::reserved, // 62
        device::rsp_su_instructions::reserved, // 63
    ];

    device.rsp.cpu.special_instrs = [
        device::rsp_su_instructions::sll,      // 0
        device::rsp_su_instructions::reserved, // 1
        device::rsp_su_instructions::srl,      // 2
        device::rsp_su_instructions::sra,      // 3
        device::rsp_su_instructions::sllv,     // 4
        device::rsp_su_instructions::reserved, // 5
        device::rsp_su_instructions::srlv,     // 6
        device::rsp_su_instructions::srav,     // 7
        device::rsp_su_instructions::jr,       // 8
        device::rsp_su_instructions::jalr,     // 9
        device::rsp_su_instructions::reserved, // 10
        device::rsp_su_instructions::reserved, // 11
        device::rsp_su_instructions::reserved, // 12
        device::rsp_su_instructions::break_,   // 13
        device::rsp_su_instructions::reserved, // 14
        device::rsp_su_instructions::reserved, // 15
        device::rsp_su_instructions::reserved, // 16
        device::rsp_su_instructions::reserved, // 17
        device::rsp_su_instructions::reserved, // 18
        device::rsp_su_instructions::reserved, // 19
        device::rsp_su_instructions::reserved, // 20
        device::rsp_su_instructions::reserved, // 21
        device::rsp_su_instructions::reserved, // 22
        device::rsp_su_instructions::reserved, // 23
        device::rsp_su_instructions::reserved, // 24
        device::rsp_su_instructions::reserved, // 25
        device::rsp_su_instructions::reserved, // 26
        device::rsp_su_instructions::reserved, // 27
        device::rsp_su_instructions::reserved, // 28
        device::rsp_su_instructions::reserved, // 29
        device::rsp_su_instructions::reserved, // 30
        device::rsp_su_instructions::reserved, // 31
        device::rsp_su_instructions::add,      // 32
        device::rsp_su_instructions::addu,     // 33
        device::rsp_su_instructions::sub,      // 34
        device::rsp_su_instructions::subu,     // 35
        device::rsp_su_instructions::and,      // 36
        device::rsp_su_instructions::or,       // 37
        device::rsp_su_instructions::xor,      // 38
        device::rsp_su_instructions::nor,      // 39
        device::rsp_su_instructions::reserved, // 40
        device::rsp_su_instructions::reserved, // 41
        device::rsp_su_instructions::slt,      // 42
        device::rsp_su_instructions::sltu,     // 43
        device::rsp_su_instructions::reserved, // 44
        device::rsp_su_instructions::reserved, // 45
        device::rsp_su_instructions::reserved, // 46
        device::rsp_su_instructions::reserved, // 47
    ];

    device.rsp.cpu.regimm_instrs = [
        device::rsp_su_instructions::bltz,     // 0
        device::rsp_su_instructions::bgez,     // 1
        device::rsp_su_instructions::reserved, // 2
        device::rsp_su_instructions::reserved, // 3
        device::rsp_su_instructions::reserved, // 4
        device::rsp_su_instructions::reserved, // 5
        device::rsp_su_instructions::reserved, // 6
        device::rsp_su_instructions::reserved, // 7
        device::rsp_su_instructions::reserved, // 8
        device::rsp_su_instructions::reserved, // 9
        device::rsp_su_instructions::reserved, // 10
        device::rsp_su_instructions::reserved, // 11
        device::rsp_su_instructions::reserved, // 12
        device::rsp_su_instructions::reserved, // 13
        device::rsp_su_instructions::reserved, // 14
        device::rsp_su_instructions::reserved, // 15
        device::rsp_su_instructions::bltzal,   // 16
        device::rsp_su_instructions::bgezal,   // 17
    ];

    device.rsp.cpu.cop0_instrs = [
        device::rsp_su_instructions::mfc0,     // 0
        device::rsp_su_instructions::reserved, // 1
        device::rsp_su_instructions::reserved, // 2
        device::rsp_su_instructions::reserved, // 3
        device::rsp_su_instructions::mtc0,     // 4
    ];

    device.rsp.cpu.cop2_instrs = [
        device::rsp_su_instructions::mfc2,        // 0
        device::rsp_su_instructions::reserved,    // 1
        device::rsp_su_instructions::cfc2,        // 2
        device::rsp_su_instructions::reserved,    // 3
        device::rsp_su_instructions::mtc2,        // 4
        device::rsp_su_instructions::reserved,    // 5
        device::rsp_su_instructions::ctc2,        // 6
        device::rsp_su_instructions::reserved,    // 7
        device::rsp_su_instructions::reserved,    // 8
        device::rsp_su_instructions::reserved,    // 9
        device::rsp_su_instructions::reserved,    // 10
        device::rsp_su_instructions::reserved,    // 11
        device::rsp_su_instructions::reserved,    // 12
        device::rsp_su_instructions::reserved,    // 13
        device::rsp_su_instructions::reserved,    // 14
        device::rsp_su_instructions::reserved,    // 15
        device::rsp_vu_instructions::execute_vec, // 16
        device::rsp_vu_instructions::execute_vec, // 17
        device::rsp_vu_instructions::execute_vec, // 18
        device::rsp_vu_instructions::execute_vec, // 19
        device::rsp_vu_instructions::execute_vec, // 20
        device::rsp_vu_instructions::execute_vec, // 21
        device::rsp_vu_instructions::execute_vec, // 22
        device::rsp_vu_instructions::execute_vec, // 23
        device::rsp_vu_instructions::execute_vec, // 24
        device::rsp_vu_instructions::execute_vec, // 25
        device::rsp_vu_instructions::execute_vec, // 26
        device::rsp_vu_instructions::execute_vec, // 27
        device::rsp_vu_instructions::execute_vec, // 28
        device::rsp_vu_instructions::execute_vec, // 29
        device::rsp_vu_instructions::execute_vec, // 30
        device::rsp_vu_instructions::execute_vec, // 31
    ];

    device.rsp.cpu.lwc2_instrs = [
        device::rsp_su_instructions::lbv, // 0
        device::rsp_su_instructions::lsv, // 1
        device::rsp_su_instructions::llv, // 2
        device::rsp_su_instructions::ldv, // 3
        device::rsp_su_instructions::lqv, // 4
        device::rsp_su_instructions::lrv, // 5
        device::rsp_su_instructions::lpv, // 6
        device::rsp_su_instructions::luv, // 7
        device::rsp_su_instructions::lhv, // 8
        device::rsp_su_instructions::lfv, // 9
        device::rsp_su_instructions::lwv, // 10
        device::rsp_su_instructions::ltv, // 11
    ];

    device.rsp.cpu.swc2_instrs = [
        device::rsp_su_instructions::sbv, // 0
        device::rsp_su_instructions::ssv, // 1
        device::rsp_su_instructions::slv, // 2
        device::rsp_su_instructions::sdv, // 3
        device::rsp_su_instructions::sqv, // 4
        device::rsp_su_instructions::srv, // 5
        device::rsp_su_instructions::spv, // 6
        device::rsp_su_instructions::suv, // 7
        device::rsp_su_instructions::shv, // 8
        device::rsp_su_instructions::sfv, // 9
        device::rsp_su_instructions::swv, // 10
        device::rsp_su_instructions::stv, // 11
    ];

    device.rsp.cpu.vec_instrs = [
        device::rsp_vu_instructions::vmulf, // 0
        device::rsp_vu_instructions::vmulu, // 1
        device::rsp_vu_instructions::vrndp, // 2
        device::rsp_vu_instructions::vmulq, // 3
        device::rsp_vu_instructions::vmudl, // 4
        device::rsp_vu_instructions::vmudm, // 5
        device::rsp_vu_instructions::vmudn, // 6
        device::rsp_vu_instructions::vmudh, // 7
        device::rsp_vu_instructions::vmacf, // 8
        device::rsp_vu_instructions::vmacu, // 9
        device::rsp_vu_instructions::vrndn, // 10
        device::rsp_vu_instructions::vmacq, // 11
        device::rsp_vu_instructions::vmadl, // 12
        device::rsp_vu_instructions::vmadm, // 13
        device::rsp_vu_instructions::vmadn, // 14
        device::rsp_vu_instructions::vmadh, // 15
        device::rsp_vu_instructions::vadd,  // 16
        device::rsp_vu_instructions::vsub,  // 17
        device::rsp_vu_instructions::vzero, // 18
        device::rsp_vu_instructions::vabs,  // 19
        device::rsp_vu_instructions::vaddc, // 20
        device::rsp_vu_instructions::vsubc, // 21
        device::rsp_vu_instructions::vzero, // 22
        device::rsp_vu_instructions::vzero, // 23
        device::rsp_vu_instructions::vzero, // 24
        device::rsp_vu_instructions::vzero, // 25
        device::rsp_vu_instructions::vzero, // 26
        device::rsp_vu_instructions::vzero, // 27
        device::rsp_vu_instructions::vzero, // 28
        device::rsp_vu_instructions::vsar,  // 29
        device::rsp_vu_instructions::vzero, // 30
        device::rsp_vu_instructions::vzero, // 31
        device::rsp_vu_instructions::vlt,   // 32
        device::rsp_vu_instructions::veq,   // 33
        device::rsp_vu_instructions::vne,   // 34
        device::rsp_vu_instructions::vge,   // 35
        device::rsp_vu_instructions::vcl,   // 36
        device::rsp_vu_instructions::vch,   // 37
        device::rsp_vu_instructions::vcr,   // 38
        device::rsp_vu_instructions::vmrg,  // 39
        device::rsp_vu_instructions::vand,  // 40
        device::rsp_vu_instructions::vnand, // 41
        device::rsp_vu_instructions::vor,   // 42
        device::rsp_vu_instructions::vnor,  // 43
        device::rsp_vu_instructions::vxor,  // 44
        device::rsp_vu_instructions::vnxor, // 45
        device::rsp_vu_instructions::vzero, // 46
        device::rsp_vu_instructions::vzero, // 47
        device::rsp_vu_instructions::vrcp,  // 48
        device::rsp_vu_instructions::vrcpl, // 49
        device::rsp_vu_instructions::vrcph, // 50
        device::rsp_vu_instructions::vmov,  // 51
        device::rsp_vu_instructions::vrsq,  // 52
        device::rsp_vu_instructions::vrsql, // 53
        device::rsp_vu_instructions::vrsqh, // 54
        device::rsp_vu_instructions::vnop,  // 55
        device::rsp_vu_instructions::vzero, // 56
        device::rsp_vu_instructions::vzero, // 57
        device::rsp_vu_instructions::vzero, // 58
        device::rsp_vu_instructions::vzero, // 59
        device::rsp_vu_instructions::vzero, // 60
        device::rsp_vu_instructions::vzero, // 61
        device::rsp_vu_instructions::vzero, // 62
        device::rsp_vu_instructions::vnop,  // 63
    ]
}