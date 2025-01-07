#include "wsi_platform.hpp"
#include "wsi.hpp"
#include "rdp_device.hpp"
#include "interface.hpp"
#include "spirv.hpp"

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
static SDL_Window *window;
static RDP::CommandProcessor *processor;
static SDL_WSIPlatform *wsi_platform;
static WSI *wsi;
static uint32_t cmd_data[0x00040000 >> 2];
static int cmd_cur;
static int cmd_ptr;
static bool emu_running;
static uint64_t rdp_sync_signal;
static GFX_INFO gfx_info;

static uint64_t last_frame_counter;
static uint64_t frame_counter;

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

void rdp_init(void *_window, GFX_INFO _gfx_info, bool _fullscreen, bool _upscale)
{
	window = (SDL_Window *)_window;
	SDL_SetEventFilter(sdl_event_filter, nullptr);

	gfx_info = _gfx_info;
	fullscreen = _fullscreen;
	bool window_vsync = 0;
	wsi = new WSI;
	wsi_platform = new SDL_WSIPlatform;
	wsi_platform->set_window(window);
	wsi->set_platform(wsi_platform);
	wsi->set_present_mode(window_vsync ? PresentMode::SyncToVBlank : PresentMode::UnlockedMaybeTear);
	wsi->set_backbuffer_srgb(false);
	Context::SystemHandles handles = {};
	if (!::Vulkan::Context::init_loader(nullptr))
	{
		rdp_close();
	}
	if (!wsi->init_simple(1, handles))
	{
		rdp_close();
	}
	RDP::CommandProcessorFlags flags = 0;
	if (_upscale)
	{
		flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_2X_BIT;
		flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
	}
	processor = new RDP::CommandProcessor(wsi->get_device(), gfx_info.RDRAM, 0, gfx_info.RDRAM_SIZE, gfx_info.RDRAM_SIZE / 2, flags);

	if (!processor->device_is_supported())
	{
		delete processor;
		delete wsi;
		processor = nullptr;
		rdp_close();
	}
	wsi->begin_frame();

	emu_running = true;
	last_frame_counter = 0;
	frame_counter = 0;
}

void rdp_close()
{
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

int sdl_event_filter(void *userdata, SDL_Event *event)
{
	if (event->type == SDL_WINDOWEVENT)
	{
		switch (event->window.event)
		{
		case SDL_WINDOWEVENT_CLOSE:
			emu_running = false;
			break;
		case SDL_WINDOWEVENT_RESIZED:
			wsi_platform->do_resize();
			break;
		default:
			break;
		}
	}
	else if (fullscreen && event->type == SDL_KEYDOWN)
	{
		switch (event->key.keysym.scancode)
		{
		case SDL_SCANCODE_ESCAPE:
			emu_running = false;
			break;
		default:
			break;
		}
	}

	return 0;
}

static void calculate_viewport(float *x, float *y, float *width, float *height)
{
	bool window_widescreen = false;
	int32_t display_width = (window_widescreen ? 854 : 640);
	int32_t display_height = 480;

	int w, h;
	SDL_GetWindowSize(window, &w, &h);

	*width = w;
	*height = h;
	*x = 0;
	*y = 0;
	int32_t hw = display_height * *width;
	int32_t wh = display_width * *height;

	// add letterboxes or pillarboxes if the window has a different aspect ratio
	// than the current display mode
	if (hw > wh)
	{
		int32_t w_max = wh / display_height;
		*x += (*width - w_max) / 2;
		*width = w_max;
	}
	else if (hw < wh)
	{
		int32_t h_max = hw / display_width;
		*y += (*height - h_max) / 2;
		*height = h_max;
	}
}

static void render_frame(Vulkan::Device &device)
{
	RDP::ScanoutOptions options = {};
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

bool rdp_update_screen()
{
	auto &device = wsi->get_device();
	render_frame(device);
	wsi->end_frame();
	wsi->begin_frame();
	frame_counter++;
	return emu_running;
}

static uint32_t viCalculateHorizonalWidth(uint32_t hstart, uint32_t xscale, uint32_t width)
{
	if (xscale == 0)
		return 320;

	uint32_t start = ((hstart & 0x03FF0000) >> 16) & 0x3FF;
	uint32_t end = (hstart & 0x3FF);
	uint32_t delta;
	if (end > start)
		delta = end - start;
	else
		delta = start - end;
	uint32_t scale = (xscale & 0xFFF);

	if (delta == 0)
	{
		delta = width;
	}

	return (delta * scale) / 0x400;
}

static uint32_t viCalculateVerticalHeight(uint32_t vstart, uint32_t yscale)
{
	if (yscale == 0)
		return 240;

	uint32_t start = ((vstart & 0x03FF0000) >> 16) & 0x3FF;
	uint32_t end = (vstart & 0x3FF);
	uint32_t delta;
	if (end > start)
		delta = end - start;
	else
		delta = start - end;
	uint32_t scale = (yscale & 0xFFF);

	return (delta * scale) / 0x800;
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

	*gfx_info.DPC_STATUS_REG |= DP_STATUS_PIPE_BUSY | DP_STATUS_START_GCLK;

	uint32_t offset = DP_CURRENT;
	if (*gfx_info.DPC_STATUS_REG & DP_STATUS_XBUS_DMA)
	{
		do
		{
			offset &= 0xFF8;
			cmd_data[2 * cmd_ptr + 0] = SDL_SwapBE32(*reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset));
			cmd_data[2 * cmd_ptr + 1] = SDL_SwapBE32(*reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset + 4));
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
		uint32_t command = (w1 >> 24) & 63;
		int cmd_length = cmd_len_lut[command];

		if (cmd_ptr - cmd_cur - cmd_length < 0)
		{
			*gfx_info.DPC_START_REG = *gfx_info.DPC_CURRENT_REG = *gfx_info.DPC_END_REG;
			return interrupt_timer;
		}

		if (command >= 8)
			processor->enqueue_command(cmd_length * 2, &cmd_data[2 * cmd_cur]);

		if (RDP::Op(command) == RDP::Op::SyncFull)
		{
			if (frame_counter != last_frame_counter) // Only sync once per frame
			{
				rdp_sync_signal = processor->signal_timeline();
				last_frame_counter = frame_counter;
			}
			else
			{
				rdp_sync_signal = 0;
			}

			uint32_t width = viCalculateHorizonalWidth(*gfx_info.VI_H_START_REG, *gfx_info.VI_X_SCALE_REG, *gfx_info.VI_WIDTH_REG);
			if (width == 0)
			{
				width = 320;
			}
			uint32_t height = viCalculateVerticalHeight(*gfx_info.VI_V_START_REG, *gfx_info.VI_Y_SCALE_REG);
			if (height == 0)
			{
				height = 240;
			}
			interrupt_timer = width * height * 4;

			*gfx_info.DPC_STATUS_REG &= ~(DP_STATUS_PIPE_BUSY | DP_STATUS_START_GCLK);
		}

		cmd_cur += cmd_length;
	}

	cmd_ptr = 0;
	cmd_cur = 0;
	*gfx_info.DPC_CURRENT_REG = *gfx_info.DPC_END_REG;
	*gfx_info.DPC_STATUS_REG |= DP_STATUS_CBUF_READY;

	return interrupt_timer;
}

void rdp_full_sync()
{
	if (rdp_sync_signal)
	{
		processor->wait_for_timeline(rdp_sync_signal);
	}
}
