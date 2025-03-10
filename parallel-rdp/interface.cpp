#include "wsi_platform.hpp"
#include "wsi.hpp"
#include "rdp_device.hpp"
#include "interface.hpp"
#include "spirv.hpp"
#include "spirv_crt.hpp"
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
static bool crop_letterbox;
static const uint32_t *fragment_spirv;
static size_t fragment_size;

typedef struct
{
	uint32_t depthbuffer_address;
	uint32_t framebuffer_address;
	uint32_t framebuffer_pixel_size;
	uint32_t framebuffer_width;
	uint32_t framebuffer_height;
	uint32_t framebuffer_size;
	uint32_t depthbuffer_size;
	uint8_t depthbuffer_enabled;
} FrameBufferInfo;

typedef struct
{
	float SourceSize[4];
	float OutputSize[4];
} Push;

static uint8_t *rdram_dirty;
static uint64_t sync_signal;
static FrameBufferInfo frame_buffer_info;

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
	else if (event->type == SDL_EVENT_KEY_DOWN && !event->key.repeat)
	{
		switch (event->key.scancode)
		{
		case SDL_SCANCODE_RETURN:
			if (event->key.mod & SDL_KMOD_ALT)
			{
				gfx_info.fullscreen = !gfx_info.fullscreen;
				SDL_SetWindowFullscreen(window, gfx_info.fullscreen);
			}
			break;
		case SDL_SCANCODE_F:
			if (event->key.mod & SDL_KMOD_ALT)
			{
				callback.enable_speedlimiter = !callback.enable_speedlimiter;
			}
			break;
		case SDL_SCANCODE_ESCAPE:
			if (gfx_info.fullscreen)
				callback.emu_running = false;
			break;
		case SDL_SCANCODE_F4:
			crop_letterbox = !crop_letterbox;
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

void rdp_new_processor(GFX_INFO _gfx_info)
{
	memset(&frame_buffer_info, 0, sizeof(FrameBufferInfo));
	sync_signal = 0;
	memset(rdram_dirty, 0, gfx_info.RDRAM_SIZE / 8);

	gfx_info = _gfx_info;
	if (processor)
	{
		delete processor;
	}
	RDP::CommandProcessorFlags flags = 0;

	if (gfx_info.upscale == 2)
	{
		flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
		flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_2X_BIT;
	}
	else if (gfx_info.upscale == 4)
	{
		flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
		flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_4X_BIT;
	}

	processor = new RDP::CommandProcessor(wsi->get_device(), gfx_info.RDRAM, 0, gfx_info.RDRAM_SIZE, gfx_info.RDRAM_SIZE / 2, flags);
}

void rdp_init(void *_window, GFX_INFO _gfx_info)
{
	window = (SDL_Window *)_window;
	bool result = SDL_AddEventWatch(sdl_event_filter, nullptr);
	if (!result)
	{
		printf("Could not add event watch.\n");
		return;
	}

	gfx_info = _gfx_info;

	if (gfx_info.crt)
	{
		fragment_spirv = crt_fragment_spirv;
		fragment_size = sizeof(crt_fragment_spirv);
	}
	else
	{
		fragment_spirv = plain_fragment_spirv;
		fragment_size = sizeof(plain_fragment_spirv);
	}

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

	rdram_dirty = (uint8_t *)malloc(gfx_info.RDRAM_SIZE / 8);
	rdp_new_processor(gfx_info);

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
	crop_letterbox = false;
}

void rdp_close()
{
	if (rdram_dirty)
	{
		free(rdram_dirty);
		rdram_dirty = nullptr;
	}

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

static void calculate_viewport(float *x, float *y, float *width, float *height, uint32_t display_height)
{
	uint32_t display_width = gfx_info.widescreen ? display_height * 16 / 9 : display_height * 4 / 3;

	int w, h;
	SDL_GetWindowSize(window, &w, &h);

	if (gfx_info.integer_scaling)
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

		// Center the viewport
		int integer_x = (w - *width) / 2.0f;
		int integer_y = (h - *height) / 2.0f;

		*x = integer_x;
		*y = integer_y;
	}
	else
	{
		// Regular scaling path - maintain aspect ratio
		float scale_x = w / (float)display_width;
		float scale_y = h / (float)display_height;
		float scale = (scale_x < scale_y) ? scale_x : scale_y;

		*width = display_width * scale;
		*height = display_height * scale;

		// Center the viewport
		*x = (w - *width) / 2.0f;
		*y = (h - *height) / 2.0f;
	}
}

static void render_frame(Vulkan::Device &device)
{
	RDP::ScanoutOptions options = {};
	options.persist_frame_on_invalid_input = true;
	options.blend_previous_frame = true;
	options.upscale_deinterlacing = false;

	if (crop_letterbox && gfx_info.widescreen)
	{
		options.crop_rect.enable = true;
		if (gfx_info.PAL)
		{
			options.crop_rect.top = 36;
			options.crop_rect.bottom = 36;
		}
		else
		{
			options.crop_rect.top = 30;
			options.crop_rect.bottom = 30;
		}
	}

	Vulkan::ImageHandle image = processor->scanout(options);

	Vulkan::ResourceLayout vertex_layout = {};
	Vulkan::ResourceLayout fragment_layout = {};
	fragment_layout.output_mask = 1 << 0;
	fragment_layout.sets[0].sampled_image_mask = 1 << 0;
	if (gfx_info.crt)
		fragment_layout.push_constant_size = sizeof(Push);

	// This request is cached.
	auto *program = device.request_program(vertex_spirv, sizeof(vertex_spirv),
										   fragment_spirv, fragment_size,
										   &vertex_layout,
										   &fragment_layout);

	// Blit image on screen.
	auto cmd = device.request_command_buffer();
	{
		auto rp = device.get_swapchain_render_pass(Vulkan::SwapchainRenderPass::ColorOnly);
		cmd->begin_render_pass(rp);

		cmd->set_program(program);

		// Basic default render state.
		cmd->set_opaque_state();
		cmd->set_depth_test(false, false);
		cmd->set_cull_mode(VK_CULL_MODE_NONE);

		// If we don't have an image, we just get a cleared screen in the render pass.
		if (image)
		{
			VkViewport vp = cmd->get_viewport();
			calculate_viewport(&vp.x, &vp.y, &vp.width, &vp.height, image->get_height());

			if (gfx_info.crt)
			{
				// Set shader parameters
				Push push = {
					{float(image->get_width()), float(image->get_height()), 1.0f / float(image->get_width()), 1.0f / float(image->get_height())},
					{vp.width, vp.height, 1.0f / vp.width, 1.0f / vp.height},
				};
				cmd->push_constants(&push, 0, sizeof(push));
			}

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

void calculate_buffer_size()
{
	switch (frame_buffer_info.framebuffer_pixel_size)
	{
	case 0:
		frame_buffer_info.framebuffer_size = (frame_buffer_info.framebuffer_width * frame_buffer_info.framebuffer_height / 2) >> 3;
		break;
	case 1:
		frame_buffer_info.framebuffer_size = (frame_buffer_info.framebuffer_width * frame_buffer_info.framebuffer_height) >> 3;
		break;
	case 2:
		frame_buffer_info.framebuffer_size = (frame_buffer_info.framebuffer_width * frame_buffer_info.framebuffer_height * 2) >> 3;
		break;
	case 3:
		frame_buffer_info.framebuffer_size = (frame_buffer_info.framebuffer_width * frame_buffer_info.framebuffer_height * 4) >> 3;
		break;
	}
	frame_buffer_info.depthbuffer_size = (frame_buffer_info.framebuffer_width * frame_buffer_info.framebuffer_height * 2) >> 3;
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
			if (!rdram_dirty[frame_buffer_info.framebuffer_address])
			{
				for (uint32_t i = frame_buffer_info.framebuffer_address; i < frame_buffer_info.framebuffer_address + frame_buffer_info.framebuffer_size; ++i)
				{
					rdram_dirty[i] = 1;
				}
			}

			if (frame_buffer_info.depthbuffer_enabled && !rdram_dirty[frame_buffer_info.depthbuffer_address])
			{
				for (uint32_t i = frame_buffer_info.depthbuffer_address; i < frame_buffer_info.depthbuffer_address + frame_buffer_info.depthbuffer_size; ++i)
				{
					rdram_dirty[i] = 1;
				}
			}
		}
		else if (RDP::Op(command) == RDP::Op::SetOtherModes)
		{
			frame_buffer_info.depthbuffer_enabled = (w2 >> 5) & 1;
		}
		else if (RDP::Op(command) == RDP::Op::SetColorImage)
		{
			frame_buffer_info.framebuffer_address = (w2 & 0x00FFFFFF) >> 3;
			frame_buffer_info.framebuffer_pixel_size = (w1 >> 19) & 0x3;
			frame_buffer_info.framebuffer_width = (w1 & 0x3FF) + 1;
			calculate_buffer_size();
		}
		else if (RDP::Op(command) == RDP::Op::SetMaskImage)
		{
			frame_buffer_info.depthbuffer_address = (w2 & 0x00FFFFFF) >> 3;
		}
		else if (RDP::Op(command) == RDP::Op::SetScissor)
		{
			uint32_t upper_left_x = ((w1 >> 12) & 0xFFF) >> 2;
			uint32_t upper_left_y = (w1 & 0xFFF) >> 2;
			uint32_t lower_right_x = ((w2 >> 12) & 0xFFF) >> 2;
			uint32_t lower_right_y = (w2 & 0xFFF) >> 2;
			region = (lower_right_x - upper_left_x) * (lower_right_y - upper_left_y);
			frame_buffer_info.framebuffer_height = lower_right_y;
			calculate_buffer_size();
		}
		else if (RDP::Op(command) == RDP::Op::SyncFull)
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
