fn main() {
    println!("cargo::rerun-if-changed=parallel-rdp");
    println!("cargo::rerun-if-changed=src/compat");

    let mut simd_build = cc::Build::new();
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .warnings(false)
        .std("c++17")
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
        .file("parallel-rdp/parallel-rdp-standalone/volk/volk.c")
        .file("parallel-rdp/interface.cpp")
        .file("parallel-rdp/wsi_platform.cpp")
        .include("parallel-rdp/parallel-rdp-standalone/parallel-rdp")
        .include("parallel-rdp/parallel-rdp-standalone/volk")
        .include("parallel-rdp/parallel-rdp-standalone/vulkan")
        .include("parallel-rdp/parallel-rdp-standalone/vulkan-headers/include")
        .include("parallel-rdp/parallel-rdp-standalone/util")
        .include(
            std::path::PathBuf::from(std::env::var("DEP_SDL3_OUT_DIR").to_owned().unwrap())
                .join("include"),
        );

    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap();
    if os == "windows" {
        if arch == "x86_64" {
            if env == "msvc" {
                build.flag("/arch:AVX2");
            } else if env == "gnu" {
                build.flag("-march=x86-64-v3");
            } else {
                panic!("unknown env")
            }
        } else if arch == "aarch64" {
            if env == "msvc" {
                build.flag("/arch:armv8.2");
                simd_build.flag("/arch:armv8.2");
            } else if env == "gnu" {
                build.flag("-march=armv8.2-a");
                simd_build.flag("-march=armv8.2-a");
            } else {
                panic!("unknown env")
            }
            simd_build.flag("-DSSE2NEON_SUPPRESS_WARNINGS");
            simd_build.file("src/compat/aarch64.cpp").cpp(true);
        } else {
            panic!("unknown arch")
        }
        build.flag("-DVK_USE_PLATFORM_WIN32_KHR");

        winresource::WindowsResource::new()
            .set_icon("data/icon.ico")
            .compile()
            .unwrap();
    } else if os == "linux" || os == "macos" {
        if arch == "x86_64" {
            build.flag("-march=x86-64-v3");
        } else if arch == "aarch64" {
            build.flag("-march=armv8.2-a");
            simd_build.flag("-march=armv8.2-a");
            simd_build.flag("-DSSE2NEON_SUPPRESS_WARNINGS");
            simd_build.file("src/compat/aarch64.cpp").cpp(true);
        } else {
            panic!("unknown arch")
        }
    } else {
        panic!("unknown OS")
    }

    build.compile("parallel-rdp");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let parallel_bindings = bindgen::Builder::default()
        .header("parallel-rdp/interface.hpp")
        .allowlist_function("rdp_init")
        .allowlist_function("rdp_close")
        .allowlist_function("rdp_set_vi_register")
        .allowlist_function("rdp_update_screen")
        .allowlist_function("rdp_process_commands")
        .allowlist_function("rdp_check_callback")
        .allowlist_function("rdp_new_processor")
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
            .header("src/compat/sse2neon/v1.8.0/sse2neon.h")
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
            .blocklist_type("__m128i")
            .blocklist_type("int64x2_t")
            .wrap_static_fns(true)
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Unable to generate bindings");

        simd_bindings
            .write_to_file(out_path.join("simd_bindings.rs"))
            .expect("Couldn't write bindings!");

        simd_build.file(std::env::temp_dir().join("bindgen").join("extern.c"));
        simd_build.include(".");
        simd_build.compile("simd");
    }

    let git_output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();

    let git_hash = String::from_utf8(git_output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    println!("cargo:rustc-env=N64_STACK_SIZE={}", 8 * 1024 * 1024);
}
