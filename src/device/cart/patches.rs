use crate::device;

pub fn apply(device: &mut device::Device) {
    let (instructions, checksum): (&[(usize, u32)], u64) = match device.ui.game_hash.as_str() {
        // https://github.com/gopher64/gopher64/issues/1436
        // WCW Nitro (USA): call osWritebackDCacheAll instead of writing back only the task.
        "4ADE28775C54157FCA8763DE388442B5CA149111B13CD7F1D2E6429C90EAF065" => {
            (&[(0xd9f0, 0x0c01_7a60)], 0xd4c4_5a5a_3138_2c95)
        }
        // https://github.com/gopher64/gopher64/issues/1427
        // Wonder Project J2: update the shop only with zero graphics tasks outstanding.
        // Otherwise switching items can reuse memory still referenced by the previous task.
        "8BBDA06B90CA6CBB897286B05D6BBC3D87464AF6A14C3641EC84D2B25610A3AA" => {
            (&[(0x67a4, 0x2b21_0001)], 0xe43c_9767_d6f9_4f42) // Japanese retail
        }
        "CF0E08C8349291EE5561BA03FA82287A505ACECE1644588001937B0ABC732D31" => {
            (&[(0x67a4, 0x2b21_0001)], 0x4f1e_88f5_8c52_7523) // English fan translation
        }
        // https://github.com/gopher64/gopher64/issues/1446
        // John Romero's Daikatana (Japan): the fade flag clears before the splash callback
        // returns. Wait for the callback to unregister itself before loading
        // the title overlay over its code.
        "D733DEBF3A38C643127D58E84B086F44948BD630BF6449030DB8CEA1CA51ABF8" => (
            &[
                (0x38ce8, 0x3c01_801b), // lui at, 0x801b
                (0x38cf0, 0x8c23_f4a0), // lw v1, -0xb60(at), callback pointer tested by beqz
            ],
            0x8c7c_ff4d_b5f2_fa26,
        ),
        // https://github.com/MiSTer-devel/N64_MiSTer/issues/21
        // Nintama Rantarou 64 Game Gallery (Japan): set OS_TASK_LOADABLE so a
        // yielded graphics task resumes with the microcode its display list switched to.
        "25179FBD5ABC34EEE034F4218888693D8AA1D3950693850D34917FEA35605FDF" => (
            &[
                (0xaabc, 0x35ef_0004), // ori t7, t7, 4 (graphics task flags)
                (0xaacc, 0x2518_00d0), // addiu t8, t0, 0xd0 (boot microcode end from start)
            ],
            0xce90_8dcf_597e_0b61,
        ),
        // https://github.com/ares-emulator/ares/issues/2321
        // Pro Mahjong Kiwame 64 (Japan): the Pak menu builds overlapping display lists.
        // Let its thread run before the frame loop, then submit and wait for the RSP
        // to finish reading before flipping buffers and waking the menu.
        "EF0C1333AA1BC6AC40AD6A3C18E17B392183E29DE04A47188B4AFFF06722134B" => (
            &[
                (0x14a0, 0x1100_67d4), // beqz t0, graphics hook at 0x8001a7f4
                (0x14a4, 0x3c09_8005), // lui t1, hi(__osActiveQueue)
                (0x14cc, 0x0611_2a89), // bgezal s0, submit (skip if already submitted)
                (0xc150, 0xad18_0004), // mark the task pending before enqueueing it
                // Hook in padding after the PI read routine.
                (0x1b3f4, 0x8d29_242c),  // lw t1, __osActiveQueue(t1)
                (0x1b3f8, 0x3c0a_8016),  // lui t2, hi(pak_thread)
                (0x1b3fc, 0x354a_18c0),  // ori t2, t2, lo(pak_thread)
                (0x1b400, 0x152a_982d),  // bne t1, t2, flip_buffers
                (0x1b404, 0x0000_8025),  // move s0, zero (submit after flip)
                (0x1b414, 0x0c00_2cbd),  // jal submit
                (0x1b418, 0x2410_ffff),  // li s0, -1 (submit before flip)
                (0x1b41c, 0x8c2a_347c), // wait: lw t2, task.state(at); submit leaves at = 0x800d0000
                (0x1b420, 0x314a_0002), // andi t2, t2, OS_SC_NEEDS_RSP
                (0x1b424, 0x1540_fffd), // bnez t2, wait
                (0x1b42c, 0x0800_022e), // j flip_buffers at 0x800008b8
                (0x1f5a10, 0x2418_0051), // Pak thread priority 81 (frame loop is 80)
            ],
            0x1b74_35d5_ebe5_c4e0,
        ),
        "EA587B80388D764BF1CBB67B5C7AAB9265B0590561B83F943E71770A3554D435" => (
            // Rev 1
            &[
                (0x14a0, 0x1100_67d4),
                (0x14a4, 0x3c09_8005),
                (0x14cc, 0x0611_2a89),
                (0xc150, 0xad18_0004),
                (0x1b3f4, 0x8d29_244c),
                (0x1b3f8, 0x3c0a_8016),
                (0x1b3fc, 0x354a_30e0),
                (0x1b400, 0x152a_982d),
                (0x1b404, 0x0000_8025),
                (0x1b414, 0x0c00_2cbd),
                (0x1b418, 0x2410_ffff),
                (0x1b41c, 0x8c2a_349c),
                (0x1b420, 0x314a_0002),
                (0x1b424, 0x1540_fffd),
                (0x1b42c, 0x0800_022e),
                (0x1f5b40, 0x2418_0051),
            ],
            0x8af3_0df9_accb_24d5,
        ),
        // Replacement words derived from MiSTer's XOR patches.
        // https://github.com/MiSTer-devel/N64_ROM_Database/blob/main/N64-database.txt
        // Batman of the Future - Return of the Joker (Europe)
        "F96AFFC5A89BFF0843A2E35A6DEE9EA1FE8AE65719519A742780877419F5CE60" => {
            (&[(0x72ac0, 0x2444_54ac)], 0x259f_728c_e125_6c79)
        }
        // Batman Beyond - Return of the Joker (USA)
        "7CE0F1304C46F16613FF3D414C9B011A0A34BCD2A67DB1B3CBD063CFB7D5D38B" => {
            (&[(0x2d17c, 0x2444_0048)], 0x2044_89c1_72a8_4241)
        }
        // Blues Brothers 2000 (Europe)
        "2A7540A8CB2D0F2EEC24BC99E8C9AF505A17F32E3AA2E43394A4AC94867DC2C8" => (
            &[(0xab40, 0x3c44_9ba6), (0xae04, 0x3c03_126f)],
            0xd431_b8df_f75c_0049,
        ),
        // Blues Brothers 2000 (USA)
        "3CA0AC99C1611125783D474825B9D912B7B2E491D17468B0DB6FB29B941FC985" => (
            &[(0x9de0, 0x3c44_9ba6), (0xa0a4, 0x3c03_126f)],
            0x7cb0_fb5e_f665_b5c2,
        ),
        // Knife Edge - Nose Gunner
        "07578B1E624FCE4AD366F1FC5C446E4516592DA3A1F072730656AE0C5CDFD103" => (
            // Europe
            &[
                (0x19338, 0x1000_0006),
                (0x19354, 0x3c04_8012),
                (0x19358, 0x8c84_d408),
                (0x1935c, 0x3c05_8019),
                (0x19360, 0x34a5_7f63),
                (0x19364, 0x1480_0005),
                (0x19368, 0x2404_002a),
                (0x1936c, 0x2404_001a),
                (0x19370, 0xa0a4_0000),
                (0x19374, 0x1000_0003),
                (0x19378, 0x0000_0000),
                (0x1937c, 0xa0a4_0000),
                (0x19380, 0x0000_0000),
                (0x19384, 0x1000_ffbb),
                (0x19388, 0x0000_0000),
                (0x17b0c4, 0x11c0_0093),
            ],
            0x8fd6_0561_899a_0f19,
        ),
        "AB0DDAC8FCE7BA6DFD4F3CF83EB801A5BF64181515DE9FB4FE42F91E0BCB8143" => (
            // Japan
            &[
                (0x191c8, 0x1000_0006),
                (0x191e4, 0x3c04_8012),
                (0x191e8, 0x8c84_d288),
                (0x191ec, 0x3c05_8019),
                (0x191f0, 0x34a5_7dc3),
                (0x191f4, 0x1480_0005),
                (0x191f8, 0x2404_002a),
                (0x191fc, 0x2404_001a),
                (0x19200, 0xa0a4_0000),
                (0x19204, 0x1000_0003),
                (0x19208, 0x0000_0000),
                (0x1920c, 0xa0a4_0000),
                (0x19210, 0x0000_0000),
                (0x19214, 0x1000_ffbb),
                (0x19218, 0x0000_0000),
                (0x180614, 0x11c0_008b),
            ],
            0x564c_916e_086f_0fb0,
        ),
        "467FCB503F384D2479FB3A5FE22405089F56ECAB38A3ACD1A6AAF0D8B73372B5" => (
            // USA
            &[
                (0x19228, 0x1000_0006),
                (0x19244, 0x3c04_8012),
                (0x19248, 0x8c84_d298),
                (0x1924c, 0x3c05_8019),
                (0x19250, 0x34a5_7b0f),
                (0x19254, 0x1480_0005),
                (0x19258, 0x2404_002a),
                (0x1925c, 0x2404_001a),
                (0x19260, 0xa0a4_0000),
                (0x19264, 0x1000_0003),
                (0x19268, 0x0000_0000),
                (0x1926c, 0xa0a4_0000),
                (0x19270, 0x0000_0000),
                (0x19274, 0x1000_ffbb),
                (0x19278, 0x0000_0000),
                (0x17a380, 0x11c0_0083),
            ],
            0xb9d6_ca85_cc96_5fde,
        ),
        _ => return,
    };
    for &(offset, instruction) in instructions {
        device.cart.rom[offset..offset + 4].copy_from_slice(&instruction.to_be_bytes());
    }
    // Updated CIC-NUS-6102 header checksums for the patched ROM.
    device.cart.rom[0x10..0x18].copy_from_slice(&checksum.to_be_bytes());
    println!("Applied compatibility patch");
}
