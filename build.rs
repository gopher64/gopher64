fn main() {
    let mut build_parallel = cc::Build::new();
    build_parallel
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
        .includes(std::env::var("DEP_SDL2_INCLUDE"));

    let mut build_gliden64 = cc::Build::new();
    build_gliden64
        .cpp(true)
        .warnings(false)
        .std("c++17")
        .file("gliden64/GLideN64/src/3DMath.cpp")
        .file("gliden64/GLideN64/src/Combiner.cpp")
        .file("gliden64/GLideN64/src/CombinerKey.cpp")
        .file("gliden64/GLideN64/src/CommonPluginAPI.cpp")
        .file("gliden64/GLideN64/src/Config.cpp")
        .file("gliden64/GLideN64/src/convert.cpp")
        .file("gliden64/GLideN64/src/CRC_OPT.cpp")
        .file("gliden64/GLideN64/src/DebugDump.cpp")
        .file("gliden64/GLideN64/src/Debugger.cpp")
        .file("gliden64/GLideN64/src/DepthBuffer.cpp")
        .file("gliden64/GLideN64/src/DisplayWindow.cpp")
        .file("gliden64/GLideN64/src/DisplayLoadProgress.cpp")
        .file("gliden64/GLideN64/src/FrameBuffer.cpp")
        .file("gliden64/GLideN64/src/FrameBufferInfo.cpp")
        .file("gliden64/GLideN64/src/GBI.cpp")
        .file("gliden64/GLideN64/src/gDP.cpp")
        .file("gliden64/GLideN64/src/GLideN64.cpp")
        .file("gliden64/GLideN64/src/GraphicsDrawer.cpp")
        .file("gliden64/GLideN64/src/gSP.cpp")
        .file("gliden64/GLideN64/src/Log.cpp")
        .file("gliden64/GLideN64/src/N64.cpp")
        .file("gliden64/GLideN64/src/PaletteTexture.cpp")
        .file("gliden64/GLideN64/src/Performance.cpp")
        .file("gliden64/GLideN64/src/PostProcessor.cpp")
        .file("gliden64/GLideN64/src/RDP.cpp")
        .file("gliden64/GLideN64/src/RSP.cpp")
        .file("gliden64/GLideN64/src/RSP_LoadMatrix.cpp")
        .file("gliden64/GLideN64/src/SoftwareRender.cpp")
        .file("gliden64/GLideN64/src/TexrectDrawer.cpp")
        .file("gliden64/GLideN64/src/TextDrawerStub.cpp")
        .file("gliden64/GLideN64/src/TextureFilterHandler.cpp")
        .file("gliden64/GLideN64/src/Textures.cpp")
        .file("gliden64/GLideN64/src/VI.cpp")
        .file("gliden64/GLideN64/src/ZlutTexture.cpp")
        .file("gliden64/GLideN64/src/BufferCopy/BlueNoiseTexture.cpp")
        .file("gliden64/GLideN64/src/BufferCopy/ColorBufferToRDRAM.cpp")
        .file("gliden64/GLideN64/src/BufferCopy/DepthBufferToRDRAM.cpp")
        .file("gliden64/GLideN64/src/BufferCopy/RDRAMtoColorBuffer.cpp")
        .file("gliden64/GLideN64/src/DepthBufferRender/ClipPolygon.cpp")
        .file("gliden64/GLideN64/src/DepthBufferRender/DepthBufferRender.cpp")
        .file("gliden64/GLideN64/src/common/CommonAPIImpl_common.cpp")
        .file("gliden64/GLideN64/src/Graphics/Context.cpp")
        .file("gliden64/GLideN64/src/Graphics/ColorBufferReader.cpp")
        .file("gliden64/GLideN64/src/Graphics/CombinerProgram.cpp")
        .file("gliden64/GLideN64/src/Graphics/ObjectHandle.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLFunctions.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/ThreadedOpenGl/opengl_Command.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/ThreadedOpenGl/opengl_ObjectPool.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/ThreadedOpenGl/opengl_Wrapper.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/ThreadedOpenGl/opengl_WrappedFunctions.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/ThreadedOpenGl/RingBufferPool.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_Attributes.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_BufferedDrawer.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_BufferManipulationObjectFactory.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_CachedFunctions.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_ColorBufferReaderWithBufferStorage.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_ColorBufferReaderWithPixelBuffer.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_ColorBufferReaderWithReadPixels.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_ContextImpl.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_GLInfo.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_Parameters.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_TextureManipulationObjectFactory.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_UnbufferedDrawer.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/opengl_Utils.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerInputs.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramBuilder.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramBuilderCommon.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramBuilderAccurate.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramBuilderFast.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramImpl.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramUniformFactory.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramUniformFactoryCommon.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramUniformFactoryAccurate.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_CombinerProgramUniformFactoryFast.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_FXAA.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_ShaderStorage.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_SpecialShadersFactory.cpp")
        .file("gliden64/GLideN64/src/Graphics/OpenGLContext/GLSL/glsl_Utils.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3D.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DBETA.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DDKR.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DEX.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DEX095.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DAM.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DEX2.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DEX3.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DEX2ACCLAIM.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DEX2CBFD.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DZEX2.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DFLX2.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DGOLDEN.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DTEXA.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DPD.cpp")
        .file("gliden64/GLideN64/src/uCodes/F3DSETA.cpp")
        .file("gliden64/GLideN64/src/uCodes/F5Indi_Naboo.cpp")
        .file("gliden64/GLideN64/src/uCodes/F5Rogue.cpp")
        .file("gliden64/GLideN64/src/uCodes/L3D.cpp")
        .file("gliden64/GLideN64/src/uCodes/L3DEX2.cpp")
        .file("gliden64/GLideN64/src/uCodes/L3DEX.cpp")
        .file("gliden64/GLideN64/src/uCodes/S2DEX2.cpp")
        .file("gliden64/GLideN64/src/uCodes/S2DEX.cpp")
        .file("gliden64/GLideN64/src/uCodes/T3DUX.cpp")
        .file("gliden64/GLideN64/src/uCodes/Turbo3D.cpp")
        .file("gliden64/GLideN64/src/uCodes/ZSort.cpp")
        .file("gliden64/GLideN64/src/uCodes/ZSortBOSS.cpp")
        .file("gliden64/GLideN64/src/TxFilterStub.cpp")
        .include("gliden64/custom")
        .include("gliden64/GLideN64/src")
        .include("gliden64/GLideN64/src/inc")
        .include("gliden64/GLideN64/src/osal")
        .flag("-DMUPENPLUSAPI")
        .flag("-D__VEC4_OPT");

    #[cfg(target_os = "windows")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            build_parallel.flag("/arch:AVX2");
            build_gliden64.flag("/arch:AVX2");
        }
        build_parallel.flag("-DVK_USE_PLATFORM_WIN32_KHR");
        build_gliden64.flag("-DOS_WINDOWS");

        winres::WindowsResource::new()
            .set_icon("data/icon.ico")
            .compile()
            .unwrap();
    }
    #[cfg(target_os = "linux")]
    {
        build_gliden64.flag("-DOS_LINUX");
    }
    #[cfg(target_os = "macos")]
    {
        build_gliden64.flag("-DOS_MAC_OS_X");
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        #[cfg(target_arch = "x86_64")]
        {
            build_parallel.flag("-march=x86-64-v3");
            build_gliden64.flag("-march=x86-64-v3");
        }
    }

    build_parallel.compile("parallel-rdp");
    build_gliden64.compile("gliden64");
}
