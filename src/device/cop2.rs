use crate::device;
use crate::savestates;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cop2 {
    #[serde(skip)]
    #[serde(default = "savestates::default_instructions")]
    pub instrs: [fn(&mut device::Device, u32); 32],
    pub reg_latch: u64,
}

fn mfc2(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU2
        == 0
    {
        return unusable(device, opcode);
    }
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        device::cpu_instructions::se32((device.cpu.cop2.reg_latch) as u32 as i32)
}

fn dmfc2(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU2
        == 0
    {
        return unusable(device, opcode);
    }
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] = device.cpu.cop2.reg_latch
}

fn cfc2(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU2
        == 0
    {
        return unusable(device, opcode);
    }
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        device::cpu_instructions::se32((device.cpu.cop2.reg_latch) as u32 as i32)
}

fn mtc2(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU2
        == 0
    {
        return unusable(device, opcode);
    }
    device.cpu.cop2.reg_latch = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize]
}

fn dmtc2(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU2
        == 0
    {
        return unusable(device, opcode);
    }
    device.cpu.cop2.reg_latch = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize]
}

fn ctc2(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU2
        == 0
    {
        return unusable(device, opcode);
    }

    device.cpu.cop2.reg_latch = device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize]
}

pub fn reserved(device: &mut device::Device, opcode: u32) {
    if device.cpu.cop0.regs[device::cop0::COP0_STATUS_REG as usize] & device::cop0::COP0_STATUS_CU2
        == 0
    {
        return unusable(device, opcode);
    }

    device::exceptions::reserved_exception(device, device::cop0::COP0_CAUSE_CE2);
}

fn unusable(device: &mut device::Device, _opcode: u32) {
    device::exceptions::cop_unusable_exception(device, device::cop0::COP0_CAUSE_CE2)
}

pub fn init(device: &mut device::Device) {
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
