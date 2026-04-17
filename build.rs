fn main() {
    println!("cargo::rerun-if-changed=parallel-rdp");
    println!("cargo::rerun-if-changed=retroachievements");
    println!("cargo::rerun-if-changed=src/compat");

    let slint_config = slint_build::CompilerConfiguration::new();
    slint_build::compile_with_config("src/ui/gui/appwindow.slint", slint_config).unwrap();

    let mut simd_build = cc::Build::new();
    let mut volk_build = cc::Build::new();
    volk_build
        .std("c17")
        .include("parallel-rdp/parallel-rdp-standalone/vulkan-headers/include")
        .file("parallel-rdp/parallel-rdp-standalone/volk/volk.c");
    let mut rdp_build = cc::Build::new();
    rdp_build
        .cpp(true)
        .std("c++20")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-missing-field-initializers")
        .file("parallel-rdp/parallel-rdp-standalone/parallel-rdp/command_ring.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/parallel-rdp/rdp_device.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/parallel-rdp/rdp_dump_write.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/parallel-rdp/rdp_renderer.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/parallel-rdp/video_interface.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/buffer.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/buffer_pool.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/command_buffer.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/command_pool.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/context.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/cookie.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/descriptor_set.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/device.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/event_manager.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/fence.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/fence_manager.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/image.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/indirect_layout.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/memory_allocator.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/pipeline_event.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/query_pool.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/render_pass.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/sampler.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/semaphore.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/semaphore_manager.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/shader.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/texture/texture_format.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/vulkan/wsi.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/arena_allocator.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/logging.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/thread_id.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/aligned_alloc.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/timer.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/timeline_trace_file.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/environment.cpp")
        .file("parallel-rdp/parallel-rdp-standalone/util/thread_name.cpp")
        .file("parallel-rdp/interface.cpp")
        .file("parallel-rdp/wsi_platform.cpp")
        .include("parallel-rdp/parallel-rdp-standalone/parallel-rdp")
        .include("parallel-rdp/parallel-rdp-standalone/volk")
        .include("parallel-rdp/parallel-rdp-standalone/vulkan")
        .include("parallel-rdp/parallel-rdp-standalone/vulkan-headers/include")
        .include("parallel-rdp/parallel-rdp-standalone/util")
        .include(
            std::path::PathBuf::from(std::env::var("DEP_SDL3_OUT_DIR").unwrap()).join("include"),
        )
        .include(
            std::path::PathBuf::from(std::env::var("DEP_SDL3_TTF_OUT_DIR").unwrap())
                .join("include"),
        );

    let mut retroachievements_build = cc::Build::new();
    retroachievements_build
        .flag("-Wno-unused-parameter")
        .include("retroachievements/rcheevos/include")
        .flag("-DRC_CLIENT_SUPPORTS_HASH")
        .file("retroachievements/rcheevos/src/rc_client.c")
        .file("retroachievements/rcheevos/src/rc_compat.c")
        .file("retroachievements/rcheevos/src/rc_util.c")
        .file("retroachievements/rcheevos/src/rcheevos/alloc.c")
        .file("retroachievements/rcheevos/src/rcheevos/condition.c")
        .file("retroachievements/rcheevos/src/rcheevos/condset.c")
        .file("retroachievements/rcheevos/src/rcheevos/consoleinfo.c")
        .file("retroachievements/rcheevos/src/rcheevos/format.c")
        .file("retroachievements/rcheevos/src/rcheevos/lboard.c")
        .file("retroachievements/rcheevos/src/rcheevos/memref.c")
        .file("retroachievements/rcheevos/src/rcheevos/operand.c")
        .file("retroachievements/rcheevos/src/rcheevos/richpresence.c")
        .file("retroachievements/rcheevos/src/rcheevos/runtime.c")
        .file("retroachievements/rcheevos/src/rcheevos/runtime_progress.c")
        .file("retroachievements/rcheevos/src/rcheevos/trigger.c")
        .file("retroachievements/rcheevos/src/rcheevos/value.c")
        .file("retroachievements/rcheevos/src/rapi/rc_api_common.c")
        .file("retroachievements/rcheevos/src/rapi/rc_api_info.c")
        .file("retroachievements/rcheevos/src/rapi/rc_api_runtime.c")
        .file("retroachievements/rcheevos/src/rapi/rc_api_user.c")
        .file("retroachievements/rcheevos/src/rhash/aes.c")
        .file("retroachievements/rcheevos/src/rhash/cdreader.c")
        .file("retroachievements/rcheevos/src/rhash/md5.c")
        .file("retroachievements/rcheevos/src/rhash/hash.c")
        .file("retroachievements/rcheevos/src/rhash/hash_disc.c")
        .file("retroachievements/rcheevos/src/rhash/hash_encrypted.c")
        .file("retroachievements/rcheevos/src/rhash/hash_rom.c")
        .file("retroachievements/rcheevos/src/rhash/hash_zip.c")
        .file("retroachievements/retroachievements.c");

    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let opt_flag = if arch == "x86_64" {
        "-march=x86-64-v3"
    } else if arch == "aarch64" && os == "macos" {
        "-march=armv8.4-a"
    } else if arch == "aarch64" && os != "macos" {
        "-march=armv8.2-a"
    } else {
        panic!("unknown arch")
    };

    volk_build.flag(opt_flag);
    rdp_build.flag(opt_flag);
    simd_build.flag(opt_flag);
    retroachievements_build.flag(opt_flag);

    if os == "windows" {
        volk_build.flag("-DVK_USE_PLATFORM_WIN32_KHR");
        rdp_build.flag("-DVK_USE_PLATFORM_WIN32_KHR");

        winresource::WindowsResource::new()
            .set_icon("data/icon/icon.ico")
            .compile()
            .unwrap();
    } else if os == "macos" {
        println!("cargo:rustc-link-search=native=/opt/homebrew/opt/freetype/lib");
        println!("cargo:rustc-link-lib=dylib=freetype");

        let output = std::process::Command::new("clang")
            .args(["--print-runtime-dir"])
            .output()
            .unwrap();

        let runtime_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();

        println!("cargo:rustc-link-search=native={}", runtime_dir);
        println!("cargo:rustc-link-lib=static=clang_rt.osx");
    }

    volk_build.flag("-flto=thin");
    rdp_build.flag("-flto=thin");
    simd_build.flag("-flto=thin");
    retroachievements_build.flag("-flto=thin");

    volk_build.compile("volk");
    rdp_build.compile("parallel-rdp");
    retroachievements_build.compile("retroachievements");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let retroachievements_bindings = bindgen::Builder::default()
        .header("retroachievements/retroachievements.h")
        .allowlist_function("ra_init_client")
        .allowlist_function("ra_welcome")
        .allowlist_function("ra_shutdown_client")
        .allowlist_function("ra_get_hardcore")
        .allowlist_function("ra_load_game")
        .allowlist_function("ra_set_dmem")
        .allowlist_function("ra_do_frame")
        .allowlist_function("ra_do_idle")
        .allowlist_function("ra_http_callback")
        .allowlist_function("ra_logout_user")
        .allowlist_function("ra_login_user")
        .allowlist_function("ra_login_token_user")
        .allowlist_function("ra_is_user_logged_in")
        .allowlist_function("ra_get_username")
        .allowlist_function("ra_get_token")
        .allowlist_function("ra_state_size")
        .allowlist_function("ra_save_state")
        .allowlist_function("ra_load_state")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    retroachievements_bindings
        .write_to_file(out_path.join("retroachievements_bindings.rs"))
        .expect("Couldn't write bindings!");

    let parallel_bindings = bindgen::Builder::default()
        .header("parallel-rdp/interface.hpp")
        .allowlist_function("rdp_init")
        .allowlist_function("rdp_close")
        .allowlist_function("rdp_set_vi_register")
        .allowlist_function("rdp_update_screen")
        .allowlist_function("rdp_render_frame")
        .allowlist_function("rdp_process_commands")
        .allowlist_function("rdp_onscreen_message")
        .allowlist_function("rdp_check_callback")
        .allowlist_function("rdp_new_processor")
        .allowlist_function("rdp_check_framebuffers")
        .allowlist_function("rdp_state_size")
        .allowlist_function("rdp_save_state")
        .allowlist_function("rdp_load_state")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    parallel_bindings
        .write_to_file(out_path.join("parallel_bindings.rs"))
        .expect("Couldn't write bindings!");

    if arch == "aarch64" {
        let simd_bindings = bindgen::Builder::default()
            .header("src/compat/sse2neon/sse2neon.h")
            .allowlist_function("_mm_setzero_si128")
            .allowlist_function("_mm_set_epi8")
            .allowlist_function("_mm_movemask_epi8")
            .allowlist_function("_mm_shuffle_epi8")
            .allowlist_function("_mm_packs_epi16")
            .allowlist_function("_mm_set_epi16")
            .allowlist_function("_mm_cmpeq_epi8")
            .allowlist_function("_mm_and_si128")
            .allowlist_function("_mm_set1_epi8")
            .allowlist_function("_mm_mullo_epi16")
            .allowlist_function("_mm_cmpeq_epi16")
            .allowlist_function("_mm_add_epi16")
            .allowlist_function("_mm_slli_epi16")
            .allowlist_function("_mm_mulhi_epi16")
            .allowlist_function("_mm_srai_epi16")
            .allowlist_function("_mm_andnot_si128")
            .allowlist_function("_mm_or_si128")
            .allowlist_function("_mm_mulhi_epu16")
            .allowlist_function("_mm_sub_epi16")
            .allowlist_function("_mm_unpacklo_epi16")
            .allowlist_function("_mm_unpackhi_epi16")
            .allowlist_function("_mm_packs_epi32")
            .allowlist_function("_mm_adds_epu16")
            .allowlist_function("_mm_cmpgt_epi16")
            .allowlist_function("_mm_blendv_epi8")
            .allowlist_function("_mm_min_epi16")
            .allowlist_function("_mm_max_epi16")
            .allowlist_function("_mm_subs_epi16")
            .allowlist_function("_mm_adds_epi16")
            .allowlist_function("_mm_xor_si128")
            .allowlist_function("_mm_cmplt_epi16")
            .allowlist_function("_mm_subs_epu16")
            .allowlist_function("_mm_set1_epi32")
            .allowlist_function("_mm_set1_epi16")
            .blocklist_type("__m128i")
            .blocklist_type("int64x2_t")
            .wrap_static_fns(true)
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Unable to generate bindings");

        simd_bindings
            .write_to_file(out_path.join("simd_bindings.rs"))
            .expect("Couldn't write bindings!");

        simd_build
            .std("c17")
            .flag("-D_POSIX_C_SOURCE=200112L")
            .flag("-DSSE2NEON_SUPPRESS_WARNINGS")
            .file("src/compat/aarch64.c")
            .file(std::env::temp_dir().join("bindgen").join("extern.c"))
            .include(".")
            .compile("simd");
    }

    let git_output = std::process::Command::new("git")
        .args(["describe", "--always", "--dirty"])
        .output()
        .unwrap();

    let git_describe = String::from_utf8(git_output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_DESCRIBE={git_describe}");

    println!("cargo:rerun-if-env-changed=NETPLAY_ID");
    let netplay_id = std::env::var("NETPLAY_ID").unwrap_or("gopher64".to_string());
    println!("cargo:rustc-env=NETPLAY_ID={netplay_id}");

    println!("cargo:rerun-if-env-changed=RA_HARDCORE");
    println!("cargo:rustc-check-cfg=cfg(ra_hardcore_enabled)");
    if let Ok(ra_hardcore) = std::env::var("RA_HARDCORE")
        && ra_hardcore.to_lowercase() == "true"
    {
        println!("cargo:rustc-cfg=ra_hardcore_enabled");
    }
}
