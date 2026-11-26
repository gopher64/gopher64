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
        // https://github.com/ares-emulator/ares/issues/838
        // Iggy's Reckin' Balls (USA and Japan): retraces can fill the 64-entry game queue
        // during a level load, causing a graphics completion to be dropped. The
        // next transition then hangs waiting for the missing completion.
        "CD5FB12C0295F3911F688A2444152A2AADD417CF13F0FD89623F64EB36E630BC" => (
            &[
                // The scheduler is compressed. Install the hook after decompression,
                // before the boot code writes back the decompressed program.
                (0x1224, 0x0c00_04dd), // jal installer at 0x80001374
                // Unused bytes after the boot decompressor's scratch word.
                (0x1f74, 0x3c08_800a), // lui t0, hi(scheduler)
                (0x1f78, 0x3c09_0c00), // lui t1, high half of jal opcode
                (0x1f7c, 0x3529_04e4), // ori t1, t1, low half of jal 0x80001390
                (0x1f80, 0x0802_a024), // j osWritebackDCache
                (0x1f84, 0xad09_0570), // install the hook at the retrace send callsite
                // Only rate-limit retraces sent to the game queue, reserving four
                // entries for other messages (including graphics completions).
                (0x1f90, 0x3c08_8010), // lui t0, hi(game queue)
                (0x1f94, 0x2508_f850), // addiu t0, t0, lo(game queue)
                (0x1f98, 0x1504_0007), // bne t0, a0, send
                (0x1f9c, 0x0000_0000), // nop
                (0x1fa0, 0x8c88_0008), // lw t0, 8(a0) (validCount)
                (0x1fa4, 0x2d08_003c), // sltiu t0, t0, 60
                (0x1fa8, 0x1500_0003), // bnez t0, send
                (0x1fac, 0x2402_ffff), // li v0, -1 (message not queued)
                (0x1fb0, 0x03e0_0008), // jr ra
                (0x1fb4, 0x0000_0000), // nop
                (0x1fb8, 0x0802_a688), // send: j osSendMesg
                (0x1fbc, 0x0000_0000), // nop
            ],
            0x94c4_f69b_b505_4e32,
        ),
        // Iggy-kun no Bura Bura Poyon (Japan): same hook with relocated game
        // queue and scheduler functions. The decompressor's scratch word at
        // 0x80001370 is kept intact.
        "6B48035C9B6AF0FDE902AC8D5F29CA471AC040C8FAB090BE6142AE8FFD69284C" => (
            &[
                (0x1224, 0x0c00_04dd), // jal installer at 0x80001374
                (0x1f74, 0x3c08_800a), // lui t0, hi(scheduler)
                (0x1f78, 0x3c09_0c00),
                (0x1f7c, 0x3529_04e4), // build jal 0x80001390
                (0x1f80, 0x0802_a0cc), // j osWritebackDCache
                (0x1f84, 0xad09_0810), // install at the retrace send callsite
                (0x1f90, 0x3c08_8010),
                (0x1f94, 0x2508_0090), // game queue at 0x80100090
                (0x1f98, 0x1504_0007),
                (0x1f9c, 0x0000_0000),
                (0x1fa0, 0x8c88_0008),
                (0x1fa4, 0x2d08_003c),
                (0x1fa8, 0x1500_0003),
                (0x1fac, 0x2402_ffff),
                (0x1fb0, 0x03e0_0008),
                (0x1fb4, 0x0000_0000),
                (0x1fb8, 0x0802_a730), // send: j osSendMesg
                (0x1fbc, 0x0000_0000),
            ],
            0xd35d_d1ec_cf26_c388,
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
        // Knife Edge - Nose Gunner: use MiSTer's frame limits (0x2ab980 OS ticks
        // during dialogue, 0x1ab980 otherwise). Select the limit inside the
        // gameplay overlay: the timer thread can run when other data is loaded
        // at the overlay's address, so it must not rewrite the instruction.
        // Japan and USA: when skipping a briefing, clear the draw callback
        // before draining queued tasks so it can't keep submitting work.
        "07578B1E624FCE4AD366F1FC5C446E4516592DA3A1F072730656AE0C5CDFD103" => (
            // Europe
            &[
                (0x17b0c4, 0x11c0_0093), // Run the limiter even without dialogue.
                (0x17b324, 0xac23_d3dc), // Store the low word from osGetTime.
                (0x17b328, 0x1440_000a), // Exit if the high word is nonzero.
                (0x17b32c, 0x8c28_d408), // Load the dialogue state (delay slot).
                (0x17b330, 0x1500_0003), // With dialogue, use the original 0x2a LUI.
                (0x17b334, 0x3c01_001a), // Otherwise use 0x1a (delay slot).
                (0x17b338, 0x1000_0002), // Join at the original ORI.
                (0x17b33c, 0x0000_0000),
                (0x17b348, 0x0061_082b), // Compare osGetTime's low word to the limit.
            ],
            0x4a99_7c74_e208_7f99,
        ),
        "AB0DDAC8FCE7BA6DFD4F3CF83EB801A5BF64181515DE9FB4FE42F91E0BCB8143" => (
            // Japan
            &[
                (0x101c8, 0x3c01_800f), // lui at, 0x800f
                (0x101cc, 0xac20_bbb0), // sw zero, -0x4450(at)
                (0x180614, 0x11c0_008b),
                (0x180854, 0xac23_d25c),
                (0x180858, 0x1440_000a),
                (0x18085c, 0x8c28_d288),
                (0x180860, 0x1500_0003),
                (0x180864, 0x3c01_001a),
                (0x180868, 0x1000_0002),
                (0x18086c, 0x0000_0000),
                (0x180878, 0x0061_082b),
                (0x185be0, 0x0c03_4572), // jal 0x800d15c8, just before the setter
            ],
            0x2b59_1042_e146_cf22,
        ),
        "467FCB503F384D2479FB3A5FE22405089F56ECAB38A3ACD1A6AAF0D8B73372B5" => (
            // USA
            &[
                (0x10238, 0x3c01_800f), // lui at, 0x800f
                (0x1023c, 0xac20_bc00), // sw zero, -0x4400(at)
                (0x17a380, 0x11c0_0083),
                (0x17a5a0, 0xac23_d26c),
                (0x17a5a4, 0x1440_000a),
                (0x17a5a8, 0x8c28_d298),
                (0x17a5ac, 0x1500_0003),
                (0x17a5b0, 0x3c01_001a),
                (0x17a5b4, 0x1000_0002),
                (0x17a5b8, 0x0000_0000),
                (0x17a5c4, 0x0061_082b),
                (0x17f92c, 0x0c03_458e), // jal 0x800d1638, just before the setter
            ],
            0x74a3_2fa4_d498_9495,
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
