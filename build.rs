fn main() {
    let mut build = cc::Build::new();
    build
        .cpp(true)
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
        .includes(std::env::var("DEP_SDL2_INCLUDE"));

    #[cfg(target_os = "windows")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            build.flag("/arch:AVX2");
        }
        build.flag("-DVK_USE_PLATFORM_WIN32_KHR");

        winres::WindowsResource::new()
            .set_icon("data/icon.ico")
            .compile()
            .unwrap();
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        #[cfg(target_arch = "x86_64")]
        {
            build.flag("-march=x86-64-v3");
        }
        build
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-unused-parameter");
    }
    build.compile("parallel-rdp");
}
