use crate::device;

pub struct Cop2 {
    pub instrs: [fn(&mut device::Device, u32); 32],
    pub reg_latch: u64,
}

pub fn mfc2(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        device::cpu_instructions::se32((device.cpu.cop2.reg_latch) as u32 as i32)
}

pub fn dmfc2(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] = device.cpu.cop2.reg_latch
}

pub fn cfc2(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        device::cpu_instructions::se32((device.cpu.cop2.reg_latch) as u32 as i32)
}

pub fn mtc2(device: &mut device::Device, opcode: u32) {
    device.cpu.cop2.reg_latch = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize]
}

pub fn dmtc2(device: &mut device::Device, opcode: u32) {
    device.cpu.cop2.reg_latch = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize]
}

pub fn ctc2(device: &mut device::Device, opcode: u32) {
    device.cpu.cop2.reg_latch = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize]
}

pub fn reserved(device: &mut device::Device, _opcode: u32) {
    device::exceptions::reserved_exception(device, device::cop0::COP0_CAUSE_CE2);
}

pub fn unusable(device: &mut device::Device, _opcode: u32) {
    device::exceptions::cop_unusable_exception(device, device::cop0::COP0_CAUSE_CE2)
}

pub fn init(device: &mut device::Device) {
    set_usable(device, false);
}

pub fn set_usable(device: &mut device::Device, usable: bool) {
    if !usable {
        device.cpu.cop2.instrs = [
            device::cop2::unusable, // 0
            device::cop2::unusable, // 1
            device::cop2::unusable, // 2
            device::cop2::unusable, // 3
            device::cop2::unusable, // 4
            device::cop2::unusable, // 5
            device::cop2::unusable, // 6
            device::cop2::unusable, // 7
            device::cop2::unusable, // 8
            device::cop2::unusable, // 9
            device::cop2::unusable, // 10
            device::cop2::unusable, // 11
            device::cop2::unusable, // 12
            device::cop2::unusable, // 13
            device::cop2::unusable, // 14
            device::cop2::unusable, // 15
            device::cop2::unusable, // 16
            device::cop2::unusable, // 17
            device::cop2::unusable, // 18
            device::cop2::unusable, // 19
            device::cop2::unusable, // 20
            device::cop2::unusable, // 21
            device::cop2::unusable, // 22
            device::cop2::unusable, // 23
            device::cop2::unusable, // 24
            device::cop2::unusable, // 25
            device::cop2::unusable, // 26
            device::cop2::unusable, // 27
            device::cop2::unusable, // 28
            device::cop2::unusable, // 29
            device::cop2::unusable, // 30
            device::cop2::unusable, // 31
        ]
    } else {
        device.cpu.cop2.instrs = [
            device::cop2::mfc2,     // 0
            device::cop2::dmfc2,    // 1
            device::cop2::cfc2,     // 2
            device::cop2::reserved, // 3
            device::cop2::mtc2,     // 4
            device::cop2::dmtc2,    // 5
            device::cop2::ctc2,     // 6
            device::cop2::reserved, // 7
            device::cop2::reserved, // 8
            device::cop2::reserved, // 9
            device::cop2::reserved, // 10
            device::cop2::reserved, // 11
            device::cop2::reserved, // 12
            device::cop2::reserved, // 13
            device::cop2::reserved, // 14
            device::cop2::reserved, // 15
            device::cop2::reserved, // 16
            device::cop2::reserved, // 17
            device::cop2::reserved, // 18
            device::cop2::reserved, // 19
            device::cop2::reserved, // 20
            device::cop2::reserved, // 21
            device::cop2::reserved, // 22
            device::cop2::reserved, // 23
            device::cop2::reserved, // 24
            device::cop2::reserved, // 25
            device::cop2::reserved, // 26
            device::cop2::reserved, // 27
            device::cop2::reserved, // 28
            device::cop2::reserved, // 29
            device::cop2::reserved, // 30
            device::cop2::reserved, // 31
        ]
    }
    // we have to recalculate the instruction pointers in the icache
    for i in 0..512 {
        device.memory.icache[i].instruction[0] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[0]);
        device.memory.icache[i].instruction[1] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[1]);
        device.memory.icache[i].instruction[2] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[2]);
        device.memory.icache[i].instruction[3] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[3]);
        device.memory.icache[i].instruction[4] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[4]);
        device.memory.icache[i].instruction[5] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[5]);
        device.memory.icache[i].instruction[6] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[6]);
        device.memory.icache[i].instruction[7] =
            device::cpu::decode_opcode(device, device.memory.icache[i].words[7])
    }
}
