#include "wsi_platform.hpp"
#include "wsi.hpp"
#include "rdp_device.hpp"
#include "interface.hpp"
#include "spirv.hpp"
#include "spirv_crt.hpp"
#include <SDL3/SDL_vulkan.h>
#include <SDL3_ttf/SDL_ttf.h>

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

typedef struct
{
	uint32_t depthbuffer_address;
	uint32_t framebuffer_address;
	uint32_t framebuffer_y_offset;
	uint32_t texture_address;
	uint32_t framebuffer_pixel_size;
	uint32_t framebuffer_width;
	uint32_t texture_pixel_size;
	uint32_t texture_width;
	uint32_t framebuffer_height;
} FrameBufferInfo;

typedef struct
{
	uint32_t cmd_data[0x00040000 >> 2];
	int cmd_cur;
	int cmd_ptr;
	uint32_t region;
	FrameBufferInfo frame_buffer_info;
} RDP_DEVICE;

static SDL_Window *window;
static RDP::CommandProcessor *processor;
static SDL_WSIPlatform *wsi_platform;
static WSI *wsi;

static RDP_DEVICE rdp_device;
static bool crop_letterbox;
static CALL_BACK callback;
static GFX_INFO gfx_info;
static const uint32_t *fragment_spirv;
static size_t fragment_size;

std::vector<bool> rdram_dirty;
uint64_t sync_signal;

static TTF_Font *message_font;
static std::queue<std::string> messages;
static uint64_t message_timer;

#define MESSAGE_TIME 3000 // 3 seconds

typedef struct
{
	float SourceSize[4];
	float OutputSize[4];
} Push;

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
		callback.paused = false;
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
		case SDL_SCANCODE_P:
			if (event->key.mod & SDL_KMOD_ALT)
			{
				callback.paused = !callback.paused;
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
		case SDL_SCANCODE_LEFTBRACKET:
			callback.lower_volume = true;
			break;
		case SDL_SCANCODE_RIGHTBRACKET:
			callback.raise_volume = true;
			break;
		case SDL_SCANCODE_SLASH:
			callback.frame_advance = true;
			break;
		case SDL_SCANCODE_0:
		case SDL_SCANCODE_1:
		case SDL_SCANCODE_2:
		case SDL_SCANCODE_3:
		case SDL_SCANCODE_4:
		case SDL_SCANCODE_5:
		case SDL_SCANCODE_6:
		case SDL_SCANCODE_7:
		case SDL_SCANCODE_8:
		case SDL_SCANCODE_9:
			if (event->key.mod & SDL_KMOD_ALT)
			{
				if (event->key.scancode == SDL_SCANCODE_0)
					callback.save_state_slot = 0;
				else
					callback.save_state_slot = event->key.scancode - SDL_SCANCODE_1 + 1;
			}
			break;
		default:
			break;
		}
	}

	return 0;
}

void rdp_new_processor(GFX_INFO _gfx_info)
{
	gfx_info = _gfx_info;

	sync_signal = 0;
	rdram_dirty.assign(gfx_info.RDRAM_SIZE >> 3, false);

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
	else if (gfx_info.upscale == 8)
	{
		flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
		flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_8X_BIT;
	}

	processor = new RDP::CommandProcessor(wsi->get_device(), gfx_info.RDRAM, 0, gfx_info.RDRAM_SIZE, gfx_info.RDRAM_SIZE / 2, flags);
}

static ImageHandle create_message_image(Vulkan::Device &device, int width, const char *message)
{
	SDL_Color fg = {255, 255, 255, 255};
	SDL_Color bg = {0, 0, 0, 0};
	SDL_Surface *surface = TTF_RenderText_LCD_Wrapped(message_font, message, 0, fg, bg, width);
	ImageCreateInfo info = ImageCreateInfo::immutable_2d_image(surface->w, surface->h, VK_FORMAT_B8G8R8A8_UNORM, false);
	ImageInitialData initial_data = {};
	initial_data.data = surface->pixels;
	initial_data.row_length = surface->pitch / 4;
	initial_data.image_height = surface->h;

	ImageHandle handle = device.create_image(info, &initial_data);
	SDL_DestroySurface(surface);
	return handle;
}

void rdp_init(void *_window, GFX_INFO _gfx_info, const void *font, size_t font_size)
{
	memset(&rdp_device, 0, sizeof(RDP_DEVICE));

	window = (SDL_Window *)_window;
	SDL_SyncWindow(window);
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
		return;
	}
	if (!wsi->init_simple(1, handles))
	{
		rdp_close();
		return;
	}

	rdp_new_processor(gfx_info);

	if (!processor->device_is_supported())
	{
		rdp_close();
		return;
	}

	message_font = TTF_OpenFontIO(SDL_IOFromConstMem(font, font_size), true, 30.0);
	if (!message_font)
	{
		rdp_close();
		return;
	}

	wsi->begin_frame();

	callback.emu_running = true;
	callback.enable_speedlimiter = true;
	callback.paused = false;
	callback.save_state_slot = 0;
	crop_letterbox = false;

	messages = std::queue<std::string>();
	message_timer = 0;
}

void rdp_close()
{
	if (wsi)
		wsi->end_frame();

	if (message_font)
	{
		TTF_CloseFont(message_font);
		message_font = nullptr;
	}
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
			calculate_viewport(&vp.x, &vp.y, &vp.width, &vp.height, image->get_height() / gfx_info.upscale);

			if (gfx_info.crt)
			{
				// Set shader parameters
				Push push = {
					{float(image->get_width()), float(image->get_height()), 1.0f / float(image->get_width()), 1.0f / float(image->get_height())},
					{vp.width, vp.height, 1.0f / vp.width, 1.0f / vp.height},
				};
				cmd->push_constants(&push, 0, sizeof(push));
			}

			cmd->set_texture(0, 0, image->get_view(), Vulkan::StockSampler::NearestClamp);
			cmd->set_viewport(vp);
			// The vertices are constants in the shader.
			// Draws fullscreen quad using oversized triangle.
			cmd->draw(3);

			if (!messages.empty())
			{
				Vulkan::ImageHandle message_image = create_message_image(device, vp.width, messages.front().c_str());
				cmd->set_texture(0, 0, message_image->get_view(), Vulkan::StockSampler::NearestClamp);
				vp.x = vp.x + (vp.width - message_image->get_width()) / 2;
				vp.y = vp.y + vp.height - message_image->get_height();
				vp.height = message_image->get_height();
				vp.width = message_image->get_width();
				cmd->set_viewport(vp);

				cmd->draw(3);

				if (SDL_GetTicks() > message_timer)
				{
					messages.pop();
					message_timer = SDL_GetTicks() + MESSAGE_TIME;
				}
			}
		}

		cmd->end_render_pass();
	}
	device.submit(cmd);
}

void rdp_set_vi_register(uint32_t reg, uint32_t value)
{
	processor->set_vi_register(RDP::VIRegister(reg), value);
}

void rdp_render_frame()
{
	auto &device = wsi->get_device();
	render_frame(device);
}

void rdp_update_screen()
{
	wsi->end_frame();
	wsi->begin_frame();
}

CALL_BACK rdp_check_callback()
{
	CALL_BACK return_value = callback;
	callback.save_state = false;
	callback.load_state = false;
	callback.lower_volume = false;
	callback.raise_volume = false;
	callback.frame_advance = false;
	return return_value;
}

void rdp_check_framebuffers(uint32_t address, uint32_t length)
{
	if (sync_signal)
	{
		address >>= 3;
		length = (length + 7) >> 3;

		if (address >= rdram_dirty.size())
			return;

		uint32_t end_addr = std::min(address + length, static_cast<uint32_t>(rdram_dirty.size()));

		auto it = std::find(rdram_dirty.begin() + address, rdram_dirty.begin() + end_addr, true);
		if (it != rdram_dirty.begin() + end_addr)
		{
			processor->wait_for_timeline(sync_signal);
			rdram_dirty.assign(gfx_info.RDRAM_SIZE >> 3, false);
			sync_signal = 0;
		}
	}
}

size_t rdp_state_size()
{
	return sizeof(RDP_DEVICE);
}

void rdp_save_state(uint8_t *state)
{
	processor->wait_for_timeline(processor->signal_timeline());
	memcpy(state, &rdp_device, sizeof(RDP_DEVICE));
}

void rdp_load_state(const uint8_t *state)
{
	memcpy(&rdp_device, state, sizeof(RDP_DEVICE));
}

void rdp_onscreen_message(const char *_message)
{
	if (messages.empty())
		message_timer = SDL_GetTicks() + MESSAGE_TIME;
	messages.push(_message);
}

uint32_t pixel_size(uint32_t pixel_type, uint32_t area)
{
	switch (pixel_type)
	{
	case 0:
		return area / 2;
	case 1:
		return area;
	case 2:
		return area * 2;
	case 3:
		return area * 4;
	default:
		printf("Invalid pixel size: %u\n", pixel_type);
		return 0;
	}
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
	if ((rdp_device.cmd_ptr + length) & ~(0x0003FFFF >> 3))
		return interrupt_timer;

	uint32_t offset = DP_CURRENT;
	if (*gfx_info.DPC_STATUS_REG & DP_STATUS_XBUS_DMA)
	{
		do
		{
			offset &= 0xFF8;
			rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 0] = SDL_Swap32BE(*reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset));
			rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 1] = SDL_Swap32BE(*reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset + 4));
			offset += sizeof(uint64_t);
			rdp_device.cmd_ptr++;
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
				rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 0] = *reinterpret_cast<const uint32_t *>(gfx_info.RDRAM + offset);
				rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 1] = *reinterpret_cast<const uint32_t *>(gfx_info.RDRAM + offset + 4);
				offset += sizeof(uint64_t);
				rdp_device.cmd_ptr++;
			} while (--length > 0);
		}
	}

	while (rdp_device.cmd_cur - rdp_device.cmd_ptr < 0)
	{
		uint32_t w1 = rdp_device.cmd_data[2 * rdp_device.cmd_cur];
		uint32_t w2 = rdp_device.cmd_data[2 * rdp_device.cmd_cur + 1];
		uint32_t command = (w1 >> 24) & 63;
		int cmd_length = cmd_len_lut[command];

		if (rdp_device.cmd_ptr - rdp_device.cmd_cur - cmd_length < 0)
		{
			*gfx_info.DPC_START_REG = *gfx_info.DPC_CURRENT_REG = *gfx_info.DPC_END_REG;
			return interrupt_timer;
		}

		if (command >= 8)
			processor->enqueue_command(cmd_length * 2, &rdp_device.cmd_data[2 * rdp_device.cmd_cur]);

		switch (RDP::Op(command))
		{
		case RDP::Op::FillZBufferTriangle:
		case RDP::Op::TextureZBufferTriangle:
		case RDP::Op::ShadeZBufferTriangle:
		case RDP::Op::ShadeTextureZBufferTriangle:
		{
			uint32_t offset_address = (rdp_device.frame_buffer_info.depthbuffer_address + pixel_size(2, rdp_device.frame_buffer_info.framebuffer_y_offset * rdp_device.frame_buffer_info.framebuffer_width)) >> 3;
			if (offset_address < rdram_dirty.size() && !rdram_dirty[offset_address])
			{
				std::fill_n(rdram_dirty.begin() + offset_address, (pixel_size(2, rdp_device.frame_buffer_info.framebuffer_width * rdp_device.frame_buffer_info.framebuffer_height) + 7) >> 3, true);
			}
		}
		case RDP::Op::FillTriangle:
		case RDP::Op::TextureTriangle:
		case RDP::Op::ShadeTriangle:
		case RDP::Op::ShadeTextureTriangle:
		case RDP::Op::TextureRectangle:
		case RDP::Op::TextureRectangleFlip:
		case RDP::Op::FillRectangle:
		{
			uint32_t offset_address = (rdp_device.frame_buffer_info.framebuffer_address + pixel_size(rdp_device.frame_buffer_info.framebuffer_pixel_size, rdp_device.frame_buffer_info.framebuffer_y_offset * rdp_device.frame_buffer_info.framebuffer_width)) >> 3;
			if (offset_address < rdram_dirty.size() && !rdram_dirty[offset_address])
			{
				std::fill_n(rdram_dirty.begin() + offset_address, (pixel_size(rdp_device.frame_buffer_info.framebuffer_pixel_size, rdp_device.frame_buffer_info.framebuffer_width * rdp_device.frame_buffer_info.framebuffer_height) + 7) >> 3, true);
			}
		}
		break;
		case RDP::Op::LoadTLut:
		case RDP::Op::LoadTile:
		{
			uint32_t upper_left_t = (w1 & 0xFFF) >> 2;
			uint32_t offset_address = (rdp_device.frame_buffer_info.texture_address + pixel_size(rdp_device.frame_buffer_info.texture_pixel_size, upper_left_t * rdp_device.frame_buffer_info.texture_width)) >> 3;
			if (offset_address < rdram_dirty.size() && !rdram_dirty[offset_address])
			{
				uint32_t lower_right_t = (w2 & 0xFFF) >> 2;
				std::fill_n(rdram_dirty.begin() + offset_address, (pixel_size(rdp_device.frame_buffer_info.texture_pixel_size, (lower_right_t - upper_left_t) * rdp_device.frame_buffer_info.texture_width) + 7) >> 3, true);
			}
		}
		break;
		case RDP::Op::LoadBlock:
		{
			uint32_t upper_left_s = ((w1 >> 12) & 0xFFF) >> 2;
			uint32_t upper_left_t = (w1 & 0xFFF) >> 2;
			uint32_t offset_address = (rdp_device.frame_buffer_info.texture_address + pixel_size(rdp_device.frame_buffer_info.texture_pixel_size, upper_left_s + upper_left_t * rdp_device.frame_buffer_info.texture_width)) >> 3;
			if (offset_address < rdram_dirty.size() && !rdram_dirty[offset_address])
			{
				uint32_t lower_right_s = ((w2 >> 12) & 0xFFF) >> 2;
				std::fill_n(rdram_dirty.begin() + offset_address, (pixel_size(rdp_device.frame_buffer_info.texture_pixel_size, lower_right_s - upper_left_s) + 7) >> 3, true);
			}
		}
		break;
		case RDP::Op::SetColorImage:
			rdp_device.frame_buffer_info.framebuffer_address = (w2 & 0x00FFFFFF);
			rdp_device.frame_buffer_info.framebuffer_pixel_size = (w1 >> 19) & 0x3;
			rdp_device.frame_buffer_info.framebuffer_width = (w1 & 0x3FF) + 1;
			break;
		case RDP::Op::SetMaskImage:
			rdp_device.frame_buffer_info.depthbuffer_address = (w2 & 0x00FFFFFF);
			break;
		case RDP::Op::SetTextureImage:
			rdp_device.frame_buffer_info.texture_address = (w2 & 0x00FFFFFF);
			rdp_device.frame_buffer_info.texture_pixel_size = (w1 >> 19) & 0x3;
			rdp_device.frame_buffer_info.texture_width = (w1 & 0x3FF) + 1;
			break;
		case RDP::Op::SetScissor:
		{
			uint32_t upper_left_x = ((w1 >> 12) & 0xFFF) >> 2;
			uint32_t upper_left_y = (w1 & 0xFFF) >> 2;
			uint32_t lower_right_x = ((w2 >> 12) & 0xFFF) >> 2;
			uint32_t lower_right_y = (w2 & 0xFFF) >> 2;
			if (lower_right_x > upper_left_x && lower_right_y > upper_left_y)
			{
				rdp_device.region = (lower_right_x - upper_left_x) * (lower_right_y - upper_left_y);
			}
			else
			{
				rdp_device.region = 0;
			}

			rdp_device.frame_buffer_info.framebuffer_y_offset = upper_left_y;
			rdp_device.frame_buffer_info.framebuffer_height = lower_right_y - upper_left_y;
		}
		break;
		case RDP::Op::SyncFull:
			sync_signal = processor->signal_timeline();

			interrupt_timer = rdp_device.region;
			if (interrupt_timer == 0)
				interrupt_timer = 5000;
			break;
		default:
			break;
		}

		rdp_device.cmd_cur += cmd_length;
	}

	rdp_device.cmd_ptr = 0;
	rdp_device.cmd_cur = 0;
	*gfx_info.DPC_CURRENT_REG = *gfx_info.DPC_END_REG;

	return interrupt_timer;
}
