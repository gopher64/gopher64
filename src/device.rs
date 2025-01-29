#![allow(non_camel_case_types)]
#![allow(improper_ctypes)]

#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct __m128i(std::arch::aarch64::int64x2_t);
#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use crate::netplay;
use crate::ui;
use std::collections::HashMap;
use std::fs;
use std::io::Read;

pub mod ai;
pub mod cache;
pub mod cart;
pub mod cart_rom;
pub mod controller;
pub mod cop0;
pub mod cop1;
pub mod cop2;
pub mod cpu;
pub mod cpu_instructions;
pub mod events;
pub mod exceptions;
pub mod fpu_instructions;
pub mod is_viewer;
pub mod memory;
pub mod mi;
pub mod pi;
pub mod pif;
pub mod rdp;
pub mod rdram;
pub mod ri;
pub mod rsp_cpu;
pub mod rsp_interface;
pub mod rsp_su_instructions;
pub mod rsp_vu_instructions;
pub mod si;
pub mod sram;
pub mod tlb;
pub mod unmapped;
pub mod vi;

pub fn run_game(rom_contents: Vec<u8>, device: &mut Device, fullscreen: bool) {
    cart_rom::init(device, rom_contents); // cart needs to come before rdram

    // rdram pointer is shared with parallel-rdp
    rdram::init(device);

    ui::audio::init(&mut device.ui, 33600);
    ui::video::init(device, fullscreen);
    ui::input::init(&mut device.ui);

    mi::init(device);
    pif::init(device);
    if device.ui.config.input.emulate_vru {
        controller::vru::init(device);
    }
    memory::init(device);
    rsp_interface::init(device);
    rdp::init(device);
    vi::init(device);
    cpu::init(device);

    ui::storage::init(&mut device.ui);
    ui::storage::load_saves(&mut device.ui, &mut device.netplay);
    cart_rom::load_rom_save(device);

    cpu::run(device);

    ui::input::close(&mut device.ui);
    ui::video::close(&device.ui);
    ui::audio::close(&mut device.ui);
    ui::storage::write_saves(&device.ui, &device.netplay);
}

fn swap_rom(contents: Vec<u8>) -> Vec<u8> {
    let test = u32::from_be_bytes(contents[0..4].try_into().unwrap());
    if test == 0x80371240 {
        // z64
        contents
    } else if test == 0x37804012 {
        // v64
        let mut data: Vec<u8> = vec![0; contents.len()];
        for i in (0..contents.len()).step_by(2) {
            let temp = u16::from_ne_bytes(contents[i..i + 2].try_into().unwrap());
            data[i..i + 2].copy_from_slice(&temp.to_be_bytes());
        }
        return data;
    } else if test == 0x40123780 {
        // n64
        let mut data: Vec<u8> = vec![0; contents.len()];
        for i in (0..contents.len()).step_by(4) {
            let temp = u32::from_ne_bytes(contents[i..i + 4].try_into().unwrap());
            data[i..i + 4].copy_from_slice(&temp.to_be_bytes());
        }
        return data;
    } else {
        let data: Vec<u8> = vec![];
        data
    }
}

pub fn get_rom_contents(file_path: &std::path::Path) -> Vec<u8> {
    let mut contents = vec![];
    if file_path.extension().unwrap().eq_ignore_ascii_case("zip") {
        let zip_file = fs::File::open(file_path).unwrap();
        let mut archive = zip::ZipArchive::new(zip_file).unwrap();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let extension = file
                .enclosed_name()
                .unwrap()
                .extension()
                .unwrap()
                .to_ascii_lowercase();
            if extension == "z64" || extension == "n64" || extension == "v64" {
                file.read_to_end(&mut contents)
                    .expect("could not read zip file");
                break;
            }
        }
    } else if file_path.extension().unwrap().eq_ignore_ascii_case("7z") {
        let mut archive =
            sevenz_rust::SevenZReader::open(file_path, sevenz_rust::Password::empty()).unwrap();

        let mut found = false;
        archive
            .for_each_entries(
                &mut |entry: &sevenz_rust::SevenZArchiveEntry, reader: &mut dyn std::io::Read| {
                    let name = entry.name().to_ascii_lowercase();
                    if !found
                        && (name.ends_with("z64") || name.ends_with("n64") || name.ends_with("v64"))
                    {
                        reader
                            .read_to_end(&mut contents)
                            .expect("could not read zip file");
                        found = true;
                    } else {
                        //skip other files
                        std::io::copy(reader, &mut std::io::sink())?;
                    }
                    Ok(true)
                },
            )
            .expect("ok");
    } else {
        contents = fs::read(file_path).expect("Should have been able to read the file");
    }

    swap_rom(contents)
}

pub struct Device {
    pub netplay: Option<netplay::Netplay>,
    pub ui: ui::Ui,
    byte_swap: usize,
    pub cpu: cpu::Cpu,
    pif: pif::Pif,
    cart: cart_rom::Cart,
    memory: memory::Memory,
    pub rsp: rsp_interface::Rsp,
    pub rdp: rdp::Rdp,
    pub rdram: rdram::Rdram,
    mi: mi::Mi,
    pi: pi::Pi,
    pub vi: vi::Vi,
    ai: ai::Ai,
    si: si::Si,
    ri: ri::Ri,
    flashram: sram::Flashram,
    pub vru: controller::vru::Vru,
}

impl Device {
    pub fn new() -> Device {
        let mut byte_swap: usize = 0;
        let test: [u8; 4] = [1, 2, 3, 4];
        // if the host computer is little endian, that means the RDRAM will be stored as little endian
        // when accessing bytes in RDRAM, we need to swap them by XORing with the byte_swap value (3)
        if u32::from_le_bytes(test) == u32::from_ne_bytes(test) {
            byte_swap = 3;
        }
        Device {
            netplay: None,
            ui: ui::Ui::new(),
            byte_swap,
            cpu: cpu::Cpu {
                cop0: cop0::Cop0 {
                    regs: [0; cop0::COP0_REGS_COUNT as usize],
                    reg_write_masks: [0; cop0::COP0_REGS_COUNT as usize],
                    reg_latch: 0,
                    instrs: [cop0::reserved; 32],
                    instrs2: [cop0::reserved; 32],
                    tlb_lut_w: vec![
                        tlb::TlbLut {
                            address: 0,
                            cached: false,
                        };
                        0x100000
                    ],
                    tlb_lut_r: vec![
                        tlb::TlbLut {
                            address: 0,
                            cached: false,
                        };
                        0x100000
                    ],
                    tlb_entries: [tlb::TlbEntry {
                        mask: 0,
                        vpn2: 0,
                        region: 0,
                        g: 0,
                        asid: 0,
                        pfn_even: 0,
                        c_even: 0,
                        d_even: 0,
                        v_even: 0,
                        pfn_odd: 0,
                        c_odd: 0,
                        d_odd: 0,
                        v_odd: 0,
                        start_even: 0,
                        end_even: 0,
                        phys_even: 0,
                        start_odd: 0,
                        end_odd: 0,
                        phys_odd: 0,
                    }; 32],
                },
                cop1: cop1::Cop1 {
                    fcr0: 0,
                    fcr31: 0,
                    //#[cfg(target_arch = "x86_64")]
                    //flush_mode: 0,
                    fgr32: [[0; 4]; 32],
                    fgr64: [[0; 8]; 32],
                    instrs: [cop1::reserved; 32],
                    b_instrs: [cop1::reserved; 4],
                    s_instrs: [cop1::reserved; 64],
                    d_instrs: [cop1::reserved; 64],
                    w_instrs: [cop1::reserved; 64],
                    l_instrs: [cop1::reserved; 64],
                },
                cop2: cop2::Cop2 {
                    instrs: [cop2::reserved; 32],
                    reg_latch: 0,
                },
                branch_state: cpu::BranchState {
                    state: cpu::State::Step,
                    pc: 0,
                },
                gpr: [0; 32],
                clock_rate: 0,
                pc: 0xBFC00000,
                pc_phys: 0,
                llbit: false,
                lo: 0,
                hi: 0,
                running: false,
                instrs: [cop0::reserved; 64],
                special_instrs: [cop0::reserved; 64],
                regimm_instrs: [cop0::reserved; 32],
                events: [events::Event {
                    enabled: false,
                    count: u64::MAX,
                    handler: events::dummy_event,
                }; events::EventType::Count as usize],
                next_event_count: u64::MAX,
                next_event: 0,
            },
            pif: pif::Pif {
                rom: [0; 1984],
                ram: [0; 64],
                channels: [pif::PifChannel {
                    tx: None,
                    tx_buf: None,
                    rx: None,
                    rx_buf: None,
                    process: None,
                    pak_handler: None,
                    change_pak: controller::PakType::None,
                }; 5],
            },
            cart: cart_rom::Cart {
                rom: Vec::new(),
                is_viewer_buffer: [0; 0xFFFF],
                pal: false,
                latch: 0,
                cic_seed: 0,
                cic_type: cart_rom::CicType::CicNus6102,
                rdram_size_offset: 0,
                rtc: cart::AfRtc { control: 0x0200 },
            },
            memory: memory::Memory {
                fast_read: [unmapped::read_mem; 0x2000],
                memory_map_read: [unmapped::read_mem; 0x2000],
                memory_map_write: [unmapped::write_mem; 0x2000],
                icache: [cache::ICache {
                    valid: false,
                    index: 0,
                    tag: 0,
                    words: [0; 8],
                    instruction: [cop0::reserved; 8],
                }; 512],
                dcache: [cache::DCache {
                    valid: false,
                    dirty: false,
                    tag: 0,
                    index: 0,
                    words: [0; 4],
                }; 512],
            },
            rdram: rdram::Rdram {
                mem: vec![],
                size: 0x800000,
                regs: [[0; rdram::RDRAM_REGS_COUNT as usize]; 4],
            },
            rsp: rsp_interface::Rsp {
                cpu: rsp_cpu::Cpu {
                    instructions: [rsp_cpu::Instructions {
                        func: rsp_su_instructions::reserved,
                        opcode: 0,
                    }; 0x1000 / 4],
                    last_instruction_type: rsp_cpu::InstructionType::Su,
                    instruction_type: rsp_cpu::InstructionType::Su,
                    pipeline_full: false,
                    branch_state: rsp_cpu::BranchState {
                        state: cpu::State::Step,
                        pc: 0,
                    },
                    broken: false,
                    halted: false,
                    sync_point: false,
                    cycle_counter: 0,
                    gpr: [0; 32],
                    vpr: unsafe { [_mm_setzero_si128(); 32] },
                    reciprocals: [0; 512],
                    inverse_square_roots: [0; 512],
                    divdp: false,
                    divin: 0,
                    divout: 0,
                    shuffle: unsafe { [_mm_setzero_si128(); 16] },
                    vcol: unsafe { _mm_setzero_si128() },
                    vcoh: unsafe { _mm_setzero_si128() },
                    vccl: unsafe { _mm_setzero_si128() },
                    vcch: unsafe { _mm_setzero_si128() },
                    vce: unsafe { _mm_setzero_si128() },
                    accl: unsafe { _mm_setzero_si128() },
                    accm: unsafe { _mm_setzero_si128() },
                    acch: unsafe { _mm_setzero_si128() },
                    special_instrs: [rsp_su_instructions::reserved; 64],
                    regimm_instrs: [rsp_su_instructions::reserved; 32],
                    cop0_instrs: [rsp_su_instructions::reserved; 32],
                    cop2_instrs: [rsp_su_instructions::reserved; 32],
                    lwc2_instrs: [rsp_su_instructions::reserved; 32],
                    swc2_instrs: [rsp_su_instructions::reserved; 32],
                    instrs: [rsp_su_instructions::reserved; 64],
                    vec_instrs: [rsp_vu_instructions::reserved; 64],
                },
                regs: [0; rsp_interface::SP_REGS_COUNT as usize],
                regs2: [0; rsp_interface::SP_REGS2_COUNT as usize],
                mem: [0; 0x2000],
                last_status_value: 0,
                fifo: [rsp_interface::RspDma {
                    dir: rsp_interface::DmaDir::None,
                    length: 0,
                    memaddr: 0,
                    dramaddr: 0,
                }; 2],
            },
            rdp: rdp::Rdp {
                regs_dpc: [0; rdp::DPC_REGS_COUNT as usize],
                regs_dps: [0; rdp::DPS_REGS_COUNT as usize],
                wait_frozen: false,
            },
            mi: mi::Mi {
                regs: [0; mi::MI_REGS_COUNT as usize],
            },
            pi: pi::Pi {
                regs: [0; pi::PI_REGS_COUNT as usize],
            },
            ai: ai::Ai {
                regs: [0; ai::AI_REGS_COUNT as usize],
                last_read: 0,
                delayed_carry: false,
                fifo: [ai::AiDma {
                    address: 0,
                    length: 0,
                    duration: 0,
                }; 2],
            },
            si: si::Si {
                regs: [0; si::SI_REGS_COUNT as usize],
                dma_dir: si::DmaDir::None,
            },
            ri: ri::Ri {
                regs: [0; ri::RI_REGS_COUNT as usize],
                ram_init: false,
            },
            vi: vi::Vi {
                regs: [0; vi::VI_REGS_COUNT as usize],
                clock: 0,
                field: 0,
                delay: 0,
                count_per_scanline: 0,
                limiter: None,
                vi_counter: 0,
            },
            flashram: sram::Flashram {
                status: 0,
                erase_page: 0,
                page_buf: [0xff; 128],
                silicon_id: [sram::FLASHRAM_TYPE_ID, sram::MX29L1100_ID],
                mode: sram::FlashramMode::ReadArray,
            },
            vru: controller::vru::Vru {
                status: 0,
                voice_state: 0,
                load_offset: 0,
                voice_init: 0,
                word_buffer: [0; 40],
                words: Vec::new(),
                talking: false,
                word_mappings: HashMap::new(),
                window_notifier: None,
                word_receiver: None,
                gui_ctx: None,
            },
        }
    }
}
