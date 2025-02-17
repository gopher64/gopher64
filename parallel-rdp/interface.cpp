#include "wsi_platform.hpp"
#include "wsi.hpp"
#include "rdp_device.hpp"
#include "interface.hpp"
#include "spirv.hpp"
#include <SDL3/SDL_vulkan.h>

using namespace Vulkan;

#define DP_STATUS_XBUS_DMA 0x01
#define DP_STATUS_FREEZE 0x02
#define DP_STATUS_FLUSH 0x04
#define DP_STATUS_START_GCLK 0x008
#define DP_STATUS_TMEM_BUSY 0x010
#define DP_STATUS_PIPE_BUSY 0x020
#define DP_STATUS_CMD_BUSY 0x040
#define DP_STATUS_CBUF_READY 0x080
#define DP_STATUS_DMA_BUSY 0x100
#define DP_STATUS_END_VALID 0x200
#define DP_STATUS_START_VALID 0x400

enum dpc_registers
{
	DPC_START_REG,
	DPC_END_REG,
	DPC_CURRENT_REG,
	DPC_STATUS_REG,
	DPC_CLOCK_REG,
	DPC_BUFBUSY_REG,
	DPC_PIPEBUSY_REG,
	DPC_TMEM_REG,
	DPC_REGS_COUNT
};

enum vi_registers
{
	VI_STATUS_REG,
	VI_ORIGIN_REG,
	VI_WIDTH_REG,
	VI_V_INTR_REG,
	VI_CURRENT_REG,
	VI_BURST_REG,
	VI_V_SYNC_REG,
	VI_H_SYNC_REG,
	VI_LEAP_REG,
	VI_H_START_REG,
	VI_V_START_REG,
	VI_V_BURST_REG,
	VI_X_SCALE_REG,
	VI_Y_SCALE_REG,
	VI_REGS_COUNT
};

static bool fullscreen;
static bool integer_scaling;
static SDL_Window *window;
static RDP::CommandProcessor *processor;
static SDL_WSIPlatform *wsi_platform;
static WSI *wsi;
static uint32_t cmd_data[0x00040000 >> 2];
static int cmd_cur;
static int cmd_ptr;
static CALL_BACK callback;
static GFX_INFO gfx_info;
static uint32_t region;

static uint32_t depthbuffer_address;
static uint32_t framebuffer_address;
static uint32_t framebuffer_pixel_size;
static uint32_t framebuffer_width;
static uint32_t framebuffer_height;
static uint8_t *rdram_dirty;
static uint64_t sync_signal;

static const unsigned cmd_len_lut[64] = {
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	4,
	6,
	12,
	14,
	12,
	14,
	20,
	22,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	2,
	2,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
};

bool sdl_event_filter(void *userdata, SDL_Event *event)
{
	if (event->type == SDL_EVENT_WINDOW_CLOSE_REQUESTED)
	{
		callback.emu_running = false;
	}
	else if (event->type == SDL_EVENT_WINDOW_RESIZED && callback.emu_running)
	{
		wsi_platform->do_resize();
	}
	else if (event->type == SDL_EVENT_KEY_UP)
	{
		switch (event->key.scancode)
		{
		case SDL_SCANCODE_RETURN:
			if (event->key.mod & SDL_KMOD_ALT)
			{
				fullscreen = !fullscreen;
				SDL_SetWindowFullscreen(window, fullscreen);
			}
			break;
		case SDL_SCANCODE_F:
			if (event->key.mod & SDL_KMOD_ALT)
			{
				callback.enable_speedlimiter = !callback.enable_speedlimiter;
			}
			break;
		case SDL_SCANCODE_ESCAPE:
			if (fullscreen)
				callback.emu_running = false;
			break;
		case SDL_SCANCODE_F5:
			callback.save_state = true;
			break;
		case SDL_SCANCODE_F7:
			callback.load_state = true;
			break;
		default:
			break;
		}
	}

	return 0;
}

void rdp_new_processor(GFX_INFO _gfx_info, bool _upscale)
{
	gfx_info = _gfx_info;
	if (processor)
	{
		delete processor;
	}
	RDP::CommandProcessorFlags flags = 0;
	if (_upscale)
	{
		flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_2X_BIT;
		flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
	}
	processor = new RDP::CommandProcessor(wsi->get_device(), gfx_info.RDRAM, 0, gfx_info.RDRAM_SIZE, gfx_info.RDRAM_SIZE / 2, flags);
}

void rdp_init(void *_window, GFX_INFO _gfx_info, bool _upscale, bool _integer_scaling, bool _fullscreen)
{
	window = (SDL_Window *)_window;
	bool result = SDL_AddEventWatch(sdl_event_filter, nullptr);
	if (!result)
	{
		printf("Could not add event watch.\n");
		return;
	}

	gfx_info = _gfx_info;
	fullscreen = _fullscreen;
	integer_scaling = _integer_scaling;
	bool window_vsync = 0;
	wsi = new WSI;
	wsi_platform = new SDL_WSIPlatform;
	wsi_platform->set_window(window);
	wsi->set_platform(wsi_platform);
	wsi->set_present_mode(window_vsync ? PresentMode::SyncToVBlank : PresentMode::UnlockedMaybeTear);
	wsi->set_backbuffer_srgb(false);
	Context::SystemHandles handles = {};
	if (!::Vulkan::Context::init_loader((PFN_vkGetInstanceProcAddr)SDL_Vulkan_GetVkGetInstanceProcAddr()))
	{
		rdp_close();
	}
	if (!wsi->init_simple(1, handles))
	{
		rdp_close();
	}
	rdp_new_processor(gfx_info, _upscale);

	if (!processor->device_is_supported())
	{
		delete processor;
		delete wsi;
		processor = nullptr;
		rdp_close();
	}
	wsi->begin_frame();

	callback.emu_running = true;
	callback.enable_speedlimiter = true;

	sync_signal = 0;
	rdram_dirty = (uint8_t *)malloc(gfx_info.RDRAM_SIZE / 8);
	memset(rdram_dirty, 0, gfx_info.RDRAM_SIZE / 8);
}

void rdp_close()
{
	free(rdram_dirty);
	wsi->end_frame();

	if (processor)
	{
		delete processor;
		processor = nullptr;
	}
	if (wsi)
	{
		delete wsi;
		wsi = nullptr;
	}
	if (wsi_platform)
	{
		delete wsi_platform;
		wsi_platform = nullptr;
	}
}

static void calculate_viewport(float *x, float *y, float *width, float *height)
{
	const int32_t display_width = gfx_info.PAL ? 384 : 320;
	const int32_t display_height = gfx_info.PAL ? 288 : 240;

	int w, h;
	SDL_GetWindowSize(window, &w, &h);

	if (integer_scaling)
	{
		// Integer scaling path
		int scale_x = w / display_width;
		int scale_y = h / display_height;
		int scale = (scale_x < scale_y) ? scale_x : scale_y;
		if (scale < 1)
			scale = 1;

		// Calculate scaled dimensions
		int scaled_width = display_width * scale;
		int scaled_height = display_height * scale;

		*width = scaled_width;
		*height = scaled_height;
	}
	else
	{
		// Regular scaling path - maintain aspect ratio
		float scale_x = w / (float)display_width;
		float scale_y = h / (float)display_height;
		float scale = (scale_x < scale_y) ? scale_x : scale_y;

		*width = display_width * scale;
		*height = display_height * scale;
	}

	// Center the viewport
	*x = (w - *width) / 2.0f;
	*y = (h - *height) / 2.0f;
}

static void render_frame(Vulkan::Device &device)
{
	RDP::ScanoutOptions options = {};
	options.persist_frame_on_invalid_input = true;
	options.blend_previous_frame = true;
	options.upscale_deinterlacing = false;
	Vulkan::ImageHandle image = processor->scanout(options);

	// Normally reflection is automated.
	Vulkan::ResourceLayout vertex_layout = {};
	Vulkan::ResourceLayout fragment_layout = {};
	fragment_layout.output_mask = 1 << 0;
	fragment_layout.sets[0].sampled_image_mask = 1 << 0;

	// This request is cached.
	auto *program = device.request_program(vertex_spirv, sizeof(vertex_spirv),
										   fragment_spirv, sizeof(fragment_spirv),
										   &vertex_layout,
										   &fragment_layout);

	// Blit image on screen.
	auto cmd = device.request_command_buffer();
	{
		auto rp = device.get_swapchain_render_pass(Vulkan::SwapchainRenderPass::ColorOnly);
		cmd->begin_render_pass(rp);

		VkViewport vp = cmd->get_viewport();
		calculate_viewport(&vp.x, &vp.y, &vp.width, &vp.height);

		cmd->set_program(program);

		// Basic default render state.
		cmd->set_opaque_state();
		cmd->set_depth_test(false, false);
		cmd->set_cull_mode(VK_CULL_MODE_NONE);

		// If we don't have an image, we just get a cleared screen in the render pass.
		if (image)
		{
			cmd->set_texture(0, 0, image->get_view(), Vulkan::StockSampler::LinearClamp);
			cmd->set_viewport(vp);
			// The vertices are constants in the shader.
			// Draws fullscreen quad using oversized triangle.
			cmd->draw(3);
		}

		cmd->end_render_pass();
	}
	device.submit(cmd);
}

void rdp_set_vi_register(uint32_t reg, uint32_t value)
{
	processor->set_vi_register(RDP::VIRegister(reg), value);
}

void rdp_update_screen()
{
	auto &device = wsi->get_device();
	render_frame(device);
	wsi->end_frame();
	wsi->begin_frame();
}

CALL_BACK rdp_check_callback()
{
	CALL_BACK return_value = callback;
	callback.save_state = false;
	callback.load_state = false;
	return return_value;
}

void rdp_check_framebuffers(uint32_t address)
{
	if (sync_signal && rdram_dirty[address >> 3])
	{
		processor->wait_for_timeline(sync_signal);
		memset(rdram_dirty, 0, gfx_info.RDRAM_SIZE / 8);
		sync_signal = 0;
	}
}

void rdp_full_sync()
{
	processor->wait_for_timeline(processor->signal_timeline());
}

uint64_t rdp_process_commands()
{
	uint64_t interrupt_timer = 0;
	const uint32_t DP_CURRENT = *gfx_info.DPC_CURRENT_REG & 0x00FFFFF8;
	const uint32_t DP_END = *gfx_info.DPC_END_REG & 0x00FFFFF8;

	int length = DP_END - DP_CURRENT;
	if (length <= 0)
		return interrupt_timer;

	length = unsigned(length) >> 3;
	if ((cmd_ptr + length) & ~(0x0003FFFF >> 3))
		return interrupt_timer;

	uint32_t offset = DP_CURRENT;
	if (*gfx_info.DPC_STATUS_REG & DP_STATUS_XBUS_DMA)
	{
		do
		{
			offset &= 0xFF8;
			cmd_data[2 * cmd_ptr + 0] = SDL_Swap32BE(*reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset));
			cmd_data[2 * cmd_ptr + 1] = SDL_Swap32BE(*reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset + 4));
			offset += sizeof(uint64_t);
			cmd_ptr++;
		} while (--length > 0);
	}
	else
	{
		if (DP_END > 0x7ffffff || DP_CURRENT > 0x7ffffff)
		{
			return interrupt_timer;
		}
		else
		{
			do
			{
				offset &= 0xFFFFF8;
				cmd_data[2 * cmd_ptr + 0] = *reinterpret_cast<const uint32_t *>(gfx_info.RDRAM + offset);
				cmd_data[2 * cmd_ptr + 1] = *reinterpret_cast<const uint32_t *>(gfx_info.RDRAM + offset + 4);
				offset += sizeof(uint64_t);
				cmd_ptr++;
			} while (--length > 0);
		}
	}

	while (cmd_cur - cmd_ptr < 0)
	{
		uint32_t w1 = cmd_data[2 * cmd_cur];
		uint32_t w2 = cmd_data[2 * cmd_cur + 1];
		uint32_t command = (w1 >> 24) & 63;
		int cmd_length = cmd_len_lut[command];

		if (cmd_ptr - cmd_cur - cmd_length < 0)
		{
			*gfx_info.DPC_START_REG = *gfx_info.DPC_CURRENT_REG = *gfx_info.DPC_END_REG;
			return interrupt_timer;
		}

		if (command >= 8)
			processor->enqueue_command(cmd_length * 2, &cmd_data[2 * cmd_cur]);

		if ((RDP::Op(command) >= RDP::Op::FillTriangle && RDP::Op(command) <= RDP::Op::ShadeTextureZBufferTriangle) ||
			RDP::Op(command) == RDP::Op::TextureRectangle ||
			RDP::Op(command) == RDP::Op::TextureRectangleFlip ||
			RDP::Op(command) == RDP::Op::FillRectangle)
		{
			uint32_t size = 0;
			switch (framebuffer_pixel_size)
			{
			case 0:
				size = (framebuffer_width * framebuffer_height / 2) >> 3;
				break;
			case 1:
				size = (framebuffer_width * framebuffer_height) >> 3;
				break;
			case 2:
				size = (framebuffer_width * framebuffer_height * 2) >> 3;
				break;
			case 3:
				size = (framebuffer_width * framebuffer_height * 4) >> 3;
				break;
			}

			for (uint32_t i = framebuffer_address; i < framebuffer_address + size; ++i)
			{
				rdram_dirty[i] = 1;
			}
		}
		if (RDP::Op(command) == RDP::Op::FillZBufferTriangle ||
			RDP::Op(command) == RDP::Op::TextureZBufferTriangle ||
			RDP::Op(command) == RDP::Op::ShadeZBufferTriangle ||
			RDP::Op(command) == RDP::Op::ShadeTextureZBufferTriangle)
		{
			uint32_t size = (framebuffer_width * framebuffer_height * 2) >> 3;

			for (uint32_t i = depthbuffer_address; i < depthbuffer_address + size; ++i)
			{
				rdram_dirty[i] = 1;
			}
		}
		if (RDP::Op(command) == RDP::Op::SetColorImage)
		{
			framebuffer_address = (w2 & 0x00FFFFFF) >> 3;
			framebuffer_pixel_size = (w1 >> 19) & 0x3;
			framebuffer_width = (w1 & 0x3FF) + 1;
		}
		if (RDP::Op(command) == RDP::Op::SetMaskImage)
		{
			depthbuffer_address = (w2 & 0x00FFFFFF) >> 3;
		}
		if (RDP::Op(command) == RDP::Op::SetScissor)
		{
			uint32_t upper_left_x = ((w1 >> 12) & 0xFFF) >> 2;
			uint32_t upper_left_y = (w1 & 0xFFF) >> 2;
			uint32_t lower_right_x = ((w2 >> 12) & 0xFFF) >> 2;
			uint32_t lower_right_y = (w2 & 0xFFF) >> 2;
			region = (lower_right_x - upper_left_x) * (lower_right_y - upper_left_y);
			framebuffer_height = lower_right_y;
		}
		if (RDP::Op(command) == RDP::Op::SyncFull)
		{
			sync_signal = processor->signal_timeline();
			interrupt_timer = region;
			if (interrupt_timer == 0)
				interrupt_timer = 5000;
		}

		cmd_cur += cmd_length;
	}

	cmd_ptr = 0;
	cmd_cur = 0;
	*gfx_info.DPC_CURRENT_REG = *gfx_info.DPC_END_REG;

	return interrupt_timer;
}
