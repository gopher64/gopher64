#include "interface.hpp"
#include "../retroachievements/retroachievements.h"
#include "rdp_device.hpp"
#include "rdram_tracker.hpp"
#include "spirv.hpp"
#include "spirv_crt.hpp"
#include "wsi.hpp"
#include "wsi_platform.hpp"
#include <SDL3/SDL_vulkan.h>
#include <SDL3_ttf/SDL_ttf.h>
#include <cmath>
#include <map>

#ifdef _WIN32
extern "C" {
__declspec(dllexport) unsigned long NvOptimusEnablement = 0x00000001;
__declspec(dllexport) int AmdPowerXpressRequestHighPerformance = 1;
}
#endif

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

enum dpc_registers {
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

enum vi_registers {
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

enum user_event_codes {
  USER_EVENT_SAVE_STATE = 1,
  USER_EVENT_LOAD_STATE = 2,
  USER_EVENT_EXIT_GAME = 3,
  USER_EVENT_FAST_FORWARD = 4,
  USER_EVENT_LOAD_REWIND = 5,
};

typedef struct {
  uint32_t depthbuffer_address;
  uint32_t framebuffer_address;
  uint32_t framebuffer_pixel_size;
  uint32_t framebuffer_width;
  uint32_t texture_pixel_size;
  bool depth_compare;
  bool depth_update;
} FrameBufferInfo;

typedef struct {
  uint32_t cmd_data[0x00040000 >> 2];
  int cmd_cur;
  int cmd_ptr;
  int32_t scissor_xh, scissor_yh, scissor_xl, scissor_yl;
  // last pixel column/row the scissor lets through (inclusive)
  int32_t scissor_x_last, scissor_y_last;
  uint32_t scissor_field;    // interlace: only every other line is drawn
  uint32_t scissor_keep_odd; // which lines are kept in field mode
  uint32_t cycle_type;
  uint64_t busy_until; // CPU cycle at which the RDP finishes its queued work
  FrameBufferInfo frame_buffer_info;
} RDP_DEVICE;

static void *g_tmem = nullptr;
static void *g_hidden_rdram = nullptr;

typedef struct {
  uint32_t fps;
  uint32_t vis;
} FPS_DATA;

typedef struct {
  std::string message;
  uint64_t milliseconds;
  Vulkan::ImageHandle image;
} Message;

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

// RDRAM handed to parallel-rdp that the GPU may not have finished with yet
static RdramTracker rdram_tracker;

// the timeline value that will cover commands enqueued now
static uint64_t mark_stamp() { return processor->get_timeline_value() + 1; }

// A primitive covering pixels [x0, x1] x [y0, y1] before scissoring.
// parallel-rdp only writes back pixels a primitive actually touched, so the
// RDRAM involved is the primitive's footprint inside the scissor. The color
// buffer always counts as written (that also covers blender reads).
static void mark_primitive(int32_t x0, int32_t x1, int32_t y0, int32_t y1) {
  auto &fb = rdp_device.frame_buffer_info;
  x0 = std::max(x0, rdp_device.scissor_xh);
  x1 = std::min(
      {x1, rdp_device.scissor_x_last, int32_t(fb.framebuffer_width) - 1});
  y0 = std::max(y0, rdp_device.scissor_yh);
  y1 = std::min(y1, rdp_device.scissor_y_last);
  if (x1 < x0 || y1 < y0)
    return;

  // parallel-rdp stores I4 color buffers as one byte per pixel
  uint32_t bytes =
      fb.framebuffer_pixel_size ? 1u << (fb.framebuffer_pixel_size - 1) : 1u;
  uint64_t stamp = mark_stamp();
  rdram_tracker.mark_rect(fb.framebuffer_address, fb.framebuffer_width, bytes,
                          x0, x1, y0, y1, RdramTracker::Access::Write, stamp);

  if (fb.depth_update)
    rdram_tracker.mark_rect(fb.depthbuffer_address, fb.framebuffer_width, 2, x0,
                            x1, y0, y1, RdramTracker::Access::Write, stamp);
  else if (fb.depth_compare)
    rdram_tracker.mark_rect(fb.depthbuffer_address, fb.framebuffer_width, 2, x0,
                            x1, y0, y1, RdramTracker::Access::Read, stamp);
}

// Texture loads: parallel-rdp asks this about the RDRAM each TMEM upload
// reads, as the load is enqueued (see
// Renderer::set_texture_load_snapshot_filter). Accepted ranges are copied
// right away, so later writes to them don't have to wait for the GPU. Ranges
// the RDP still has pending writes to (render-to-texture) can't be copied from
// host memory; the GPU reads those in place, so they are marked as RDP reads.
static bool texture_load_snapshot_filter(uint32_t address, uint32_t length) {
  if (rdram_tracker.conflict(address, length, false) <=
      processor->get_completed_timeline())
    return true;
  rdram_tracker.mark(address, length, RdramTracker::Access::Read, mark_stamp());
  return false;
}

static TTF_Font *message_font;
static std::queue<Message> messages;

static float message_font_size = 25.0;
static float achievement_challenge_indicator_font_size = 12.0;

static TTF_Font *achievement_challenge_indicator_font;
static std::vector<const char *> achievement_challenge_indicators;
static Vulkan::ImageHandle achievement_challenge_indicator_image;
static Vulkan::ImageHandle achievement_progress_indicator_image;
static std::map<uint32_t, std::string> leaderboard_trackers;
static bool display_challenge_indicator;

static bool display_fps;
static Vulkan::ImageHandle fps_image;

static std::queue<JoystickEvent> joystick_events;

typedef struct {
  float SourceSize[4];
  float OutputSize[4];
} Push;

static const unsigned cmd_len_lut[64] = {
    1, 1, 1, 1, 1, 1, 1, 1, 4, 6, 12, 14, 12, 14, 20, 22, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  2,  2,  1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  1,  1,  1, 1, 1, 1,
};

static void add_joystick_event(void *userdata) {
  JoystickEvent *joystick_event = (JoystickEvent *)userdata;
  joystick_events.push(*joystick_event);
  delete joystick_event;
}

bool sdl_event_filter(void *userdata, SDL_Event *event) {
  if (event->type == SDL_EVENT_WINDOW_CLOSE_REQUESTED ||
      event->type == SDL_EVENT_TERMINATING) {
    callback.paused = false;
    callback.emu_running = false;
  } else if (event->type == SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED &&
             callback.emu_running) {
    wsi_platform->do_resize();

    if (message_font) {
      TTF_SetFontSize(message_font,
                      message_font_size * SDL_GetWindowDisplayScale(window));
    }
    if (achievement_challenge_indicator_font) {
      TTF_SetFontSize(achievement_challenge_indicator_font,
                      achievement_challenge_indicator_font_size *
                          SDL_GetWindowDisplayScale(window));
    }
  } else if (event->type == SDL_EVENT_WINDOW_MINIMIZED) {
    callback.paused = true;
  } else if (event->type == SDL_EVENT_WINDOW_RESTORED) {
    callback.paused = false;
    if (callback.emu_running) {
      wsi_platform->do_resize();
    }
  } else if (event->type == SDL_EVENT_KEY_DOWN && !event->key.repeat) {
    SDL_Event user_event;
    switch (event->key.scancode) {
    case SDL_SCANCODE_RETURN:
      if (event->key.mod & SDL_KMOD_ALT) {
        gfx_info.fullscreen = !gfx_info.fullscreen;
        SDL_SetWindowFullscreen(window, gfx_info.fullscreen);
      }
      break;
    case SDL_SCANCODE_F:
      if (event->key.mod & SDL_KMOD_ALT) {
        SDL_zero(user_event);
        user_event.type = SDL_EVENT_USER;
        user_event.user.code = USER_EVENT_FAST_FORWARD;
        SDL_PushEvent(&user_event);
      }
      break;
    case SDL_SCANCODE_P:
      if (event->key.mod & SDL_KMOD_ALT) {
        callback.paused = !callback.paused;
      }
      break;
    case SDL_SCANCODE_AC_BACK:
    case SDL_SCANCODE_ESCAPE:
      if (gfx_info.fullscreen) {
        SDL_zero(user_event);
        user_event.type = SDL_EVENT_USER;
        user_event.user.code = USER_EVENT_EXIT_GAME;
        SDL_PushEvent(&user_event);
      }
      break;
    case SDL_SCANCODE_F1:
      display_fps = !display_fps;
      break;
    case SDL_SCANCODE_F4:
      crop_letterbox = !crop_letterbox;
      break;
    case SDL_SCANCODE_F5:
      SDL_zero(user_event);
      user_event.type = SDL_EVENT_USER;
      user_event.user.code = USER_EVENT_SAVE_STATE;
      SDL_PushEvent(&user_event);
      break;
    case SDL_SCANCODE_F6:
      SDL_zero(user_event);
      user_event.type = SDL_EVENT_USER;
      user_event.user.code = USER_EVENT_LOAD_REWIND;
      SDL_PushEvent(&user_event);
      break;
    case SDL_SCANCODE_F7:
      SDL_zero(user_event);
      user_event.type = SDL_EVENT_USER;
      user_event.user.code = USER_EVENT_LOAD_STATE;
      SDL_PushEvent(&user_event);
      break;
    case SDL_SCANCODE_F8:
      if (messages.empty())
        SDL_RunOnMainThread(ra_display_inprogress_achievements, nullptr, false);
      break;
    case SDL_SCANCODE_F9:
      display_challenge_indicator = !display_challenge_indicator;
      rdp_onscreen_message(
          std::format("Challenge indicators: {}",
                      display_challenge_indicator ? "ON" : "OFF")
              .c_str(),
          MESSAGE_VERY_SHORT);
      break;
    case SDL_SCANCODE_F12:
      callback.reset_game = true;
      break;
    case SDL_SCANCODE_LEFTBRACKET:
      if (event->key.mod & SDL_KMOD_ALT) {
        callback.decrease_input_delay = true;
      } else {
        callback.lower_volume = true;
      }
      break;
    case SDL_SCANCODE_RIGHTBRACKET:
      if (event->key.mod & SDL_KMOD_ALT) {
        callback.increase_input_delay = true;
      } else {
        callback.raise_volume = true;
      }
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
      if (event->key.mod & SDL_KMOD_ALT) {
        if (event->key.scancode == SDL_SCANCODE_0)
          callback.save_state_slot = 0;
        else
          callback.save_state_slot = event->key.scancode - SDL_SCANCODE_1 + 1;
      }
      break;
    default:
      break;
    }
  } else if (event->type == SDL_EVENT_USER) {
    switch (event->user.code) {
    case USER_EVENT_SAVE_STATE:
      callback.save_state = true;
      break;
    case USER_EVENT_LOAD_REWIND:
      callback.load_rewind = true;
      break;
    case USER_EVENT_LOAD_STATE:
      callback.load_state = true;
      break;
    case USER_EVENT_EXIT_GAME:
      callback.emu_running = false;
      break;
    case USER_EVENT_FAST_FORWARD:
      callback.enable_speedlimiter = !callback.enable_speedlimiter;
      break;
    default:
      break;
    }
  } else if (event->type == SDL_EVENT_JOYSTICK_ADDED) {
    JoystickEvent *joystick_event = new JoystickEvent;
    joystick_event->joystick_id = event->jdevice.which;
    joystick_event->connected = true;
    SDL_RunOnMainThread(add_joystick_event, joystick_event, false);
  } else if (event->type == SDL_EVENT_JOYSTICK_REMOVED) {
    JoystickEvent *joystick_event = new JoystickEvent;
    joystick_event->joystick_id = event->jdevice.which;
    joystick_event->connected = false;
    SDL_RunOnMainThread(add_joystick_event, joystick_event, false);
  } else if (event->type == SDL_EVENT_WILL_ENTER_BACKGROUND) {
    wsi->deinit_surface_and_swapchain();
  } else if (event->type == SDL_EVENT_RENDER_DEVICE_RESET) {
    wsi->init_surface_swapchain();
  }

  return 0;
}

JoystickEvent get_joystick_event() {
  if (joystick_events.empty())
    return JoystickEvent{0, false};
  JoystickEvent joystick_event = joystick_events.front();
  joystick_events.pop();
  return joystick_event;
}

void rdp_idle() { processor->idle(); }

static void rdp_new_processor() {
  RDP::CommandProcessorFlags flags =
      RDP::COMMAND_PROCESSOR_FLAG_HOST_VISIBLE_HIDDEN_RDRAM_BIT |
      RDP::COMMAND_PROCESSOR_FLAG_HOST_VISIBLE_TMEM_BIT;

  if (gfx_info.upscale == 2) {
    flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
    flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_2X_BIT;
  } else if (gfx_info.upscale == 4) {
    flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
    flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_4X_BIT;
  } else if (gfx_info.upscale == 8) {
    flags |= RDP::COMMAND_PROCESSOR_FLAG_SUPER_SAMPLED_DITHER_BIT;
    flags |= RDP::COMMAND_PROCESSOR_FLAG_UPSCALING_8X_BIT;
  } else {
    gfx_info.upscale = 1;
  }

  processor = new RDP::CommandProcessor(wsi->get_device(), gfx_info.RDRAM, 0,
                                        gfx_info.RDRAM_SIZE,
                                        gfx_info.RDRAM_SIZE / 2, flags);
  g_tmem = processor->get_tmem();
  if (!g_tmem) {
    LOGE("Failed to get tmem\n");
  }
  g_hidden_rdram = processor->begin_read_hidden_rdram();
  if (!g_hidden_rdram) {
    LOGE("Failed to get hidden_rdram\n");
  }

  if (!processor->set_texture_load_snapshot_filter(
          texture_load_snapshot_filter)) {
    LOGE("Texture load snapshots need single-threaded command processing\n");
  }

  // the new processor's timeline starts over
  rdram_tracker.reset(gfx_info.RDRAM_SIZE);
}

static ImageHandle create_message_image(Vulkan::Device &device, int width,
                                        TTF_Font *font, const char *message) {
  if (strstr(message, "\n"))
    width = 0;

  SDL_Color fg = {255, 255, 255, 255};
  SDL_Color bg = {0, 0, 0, 0};
  SDL_Surface *surface =
      TTF_RenderText_LCD_Wrapped(font, message, 0, fg, bg, width);
  ImageCreateInfo info = ImageCreateInfo::immutable_2d_image(
      surface->w, surface->h, VK_FORMAT_B8G8R8A8_UNORM, false);
  ImageInitialData initial_data = {};
  initial_data.data = surface->pixels;
  initial_data.row_length = surface->pitch / 4;
  initial_data.image_height = surface->h;

  ImageHandle handle = device.create_image(info, &initial_data);
  SDL_DestroySurface(surface);
  return handle;
}

bool rdp_init(void *_window, GFX_INFO _gfx_info, const void *font,
              size_t font_size, uint32_t save_state_slot) {
  memset(&rdp_device, 0, sizeof(RDP_DEVICE));
  memset(&callback, 0, sizeof(CALL_BACK));

  window = (SDL_Window *)_window;
  SDL_SyncWindow(window);
  bool result = SDL_AddEventWatch(sdl_event_filter, nullptr);
  if (!result) {
    LOGE("Could not add event watch.\n");
    return false;
  }

  gfx_info = _gfx_info;

  if (gfx_info.crt) {
    fragment_spirv = crt_fragment_spirv;
    fragment_size = sizeof(crt_fragment_spirv);
  } else {
    fragment_spirv = plain_fragment_spirv;
    fragment_size = sizeof(plain_fragment_spirv);
  }

  wsi = new WSI;
  wsi_platform = new SDL_WSIPlatform;
  wsi_platform->set_window(window);
  wsi->set_platform(wsi_platform);
  if (gfx_info.vsync) {
    // VK_PRESENT_MODE_MAILBOX_KHR, fallback to VK_PRESENT_MODE_FIFO_KHR
    wsi->set_present_mode(PresentMode::UnlockedNoTearing);
  } else {
    // VK_PRESENT_MODE_MAILBOX_KHR, fallback to VK_PRESENT_MODE_IMMEDIATE_KHR
    wsi->set_present_mode(PresentMode::UnlockedMaybeTear);
  }
  wsi->set_backbuffer_srgb(false);
  Context::SystemHandles handles = {};
  if (!::Vulkan::Context::init_loader(
          (PFN_vkGetInstanceProcAddr)SDL_Vulkan_GetVkGetInstanceProcAddr())) {
    LOGE("Failed to init vulkan loader.\n");
    rdp_close();
    return false;
  }
  if (!wsi->init_simple(1, handles)) {
    LOGE("Failed to init parallel-rdp.\n");
    rdp_close();
    return false;
  }

  rdp_new_processor();

  if (!processor->device_is_supported()) {
    LOGE("Failed to init parallel-rdp processor.\n");
    rdp_close();
    return false;
  }

  message_font =
      TTF_OpenFontIO(SDL_IOFromConstMem(font, font_size), true,
                     message_font_size * SDL_GetWindowDisplayScale(window));
  achievement_challenge_indicator_font =
      TTF_OpenFontIO(SDL_IOFromConstMem(font, font_size), true,
                     achievement_challenge_indicator_font_size *
                         SDL_GetWindowDisplayScale(window));
  if (!message_font || !achievement_challenge_indicator_font) {
    LOGE("Failed to init fonts.\n");
    rdp_close();
    return false;
  }

  wsi->begin_frame();

  callback.emu_running = true;
  callback.enable_speedlimiter = true;
  callback.paused = false;
  callback.save_state_slot = save_state_slot;
  crop_letterbox = false;

  messages = std::queue<Message>();

  display_challenge_indicator = true;
  achievement_challenge_indicators.clear();
  leaderboard_trackers.clear();
  achievement_challenge_indicator_image = Vulkan::ImageHandle();
  achievement_progress_indicator_image = Vulkan::ImageHandle();
  fps_image = Vulkan::ImageHandle();
  display_fps = false;

  return true;
}

void rdp_close() {
  display_fps = false;

  g_tmem = nullptr;
  g_hidden_rdram = nullptr;

  messages = std::queue<Message>();
  achievement_challenge_indicator_image = Vulkan::ImageHandle();
  achievement_progress_indicator_image = Vulkan::ImageHandle();
  fps_image = Vulkan::ImageHandle();

  if (message_font) {
    TTF_CloseFont(message_font);
    message_font = nullptr;
  }
  if (achievement_challenge_indicator_font) {
    TTF_CloseFont(achievement_challenge_indicator_font);
    achievement_challenge_indicator_font = nullptr;
  }
  if (processor) {
    delete processor;
    processor = nullptr;
  }
  if (wsi) {
    delete wsi;
    wsi = nullptr;
  }
  if (wsi_platform) {
    delete wsi_platform;
    wsi_platform = nullptr;
  }

  SDL_RemoveEventWatch(sdl_event_filter, nullptr);
}

static void calculate_viewport(float *x, float *y, float *width, float *height,
                               uint32_t display_height) {
  uint32_t display_width =
      gfx_info.widescreen ? display_height * 16 / 9 : display_height * 4 / 3;

  int w, h;
  SDL_GetWindowSizeInPixels(window, &w, &h);

  if (gfx_info.integer_scaling) {
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
  } else {
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

static void draw_indicator(CommandBufferHandle cmd,
                           Vulkan::ImageHandle indicator_image, VkViewport vp) {
  cmd->set_texture(0, 0, indicator_image->get_view(),
                   Vulkan::StockSampler::NearestClamp);
  vp.x = vp.x + vp.width - indicator_image->get_width();
  vp.y = vp.y + vp.height - indicator_image->get_height();
  vp.height = indicator_image->get_height();
  vp.width = indicator_image->get_width();
  cmd->set_viewport(vp);

  cmd->draw(3);
}

static void draw_fps(CommandBufferHandle cmd, VkViewport vp) {
  cmd->set_texture(0, 0, fps_image->get_view(),
                   Vulkan::StockSampler::NearestClamp);
  vp.y = vp.y + vp.height - fps_image->get_height();
  vp.height = fps_image->get_height();
  vp.width = fps_image->get_width();
  cmd->set_viewport(vp);

  cmd->draw(3);
}

static void pop_message(void *userdata) { messages.pop(); }
static uint32_t pop_message_callback(void *userdata, SDL_TimerID timerID,
                                     uint32_t interval) {
  SDL_RunOnMainThread(pop_message, NULL, false);
  return 0;
}

static void render_frame(Vulkan::Device &device) {
  RDP::ScanoutOptions options = {};
  options.persist_frame_on_invalid_input = true;
  options.blend_previous_frame = !gfx_info.ssaa;
  options.upscale_deinterlacing = gfx_info.ssaa;
  if (gfx_info.ssaa) {
    switch (gfx_info.upscale) {
    case 2:
      options.downscale_steps = 1;
      break;
    case 4:
      options.downscale_steps = 2;
      break;
    case 8:
      options.downscale_steps = 3;
      break;
    default:
      options.downscale_steps = 0;
      break;
    }
  }

  if (crop_letterbox && gfx_info.widescreen) {
    options.crop_rect.enable = true;
    if (gfx_info.PAL) {
      options.crop_rect.top = 36;
      options.crop_rect.bottom = 36;
    } else {
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
  auto *program =
      device.request_program(vertex_spirv, sizeof(vertex_spirv), fragment_spirv,
                             fragment_size, &vertex_layout, &fragment_layout);

  // Blit image on screen.
  auto cmd = device.request_command_buffer();
  {
    auto rp = device.get_swapchain_render_pass(
        Vulkan::SwapchainRenderPass::ColorOnly);
    cmd->begin_render_pass(rp);

    cmd->set_program(program);

    // Basic default render state.
    cmd->set_opaque_state();
    cmd->set_depth_test(false, false);
    cmd->set_cull_mode(VK_CULL_MODE_NONE);

    VkViewport vp = cmd->get_viewport();
    // If we don't have an image, we just get a cleared screen in the render
    // pass.
    if (image) {
      calculate_viewport(&vp.x, &vp.y, &vp.width, &vp.height,
                         image->get_height() / gfx_info.upscale);

      if (gfx_info.crt) {
        // Set shader parameters
        Push push = {
            {float(image->get_width()), float(image->get_height()),
             1.0f / float(image->get_width()),
             1.0f / float(image->get_height())},
            {vp.width, vp.height, 1.0f / vp.width, 1.0f / vp.height},
        };
        cmd->push_constants(&push, 0, sizeof(push));
      }

      cmd->set_texture(0, 0, image->get_view(),
                       Vulkan::StockSampler::NearestClamp);
      cmd->set_viewport(vp);
      // The vertices are constants in the shader.
      // Draws fullscreen quad using oversized triangle.
      cmd->draw(3);
    }
    if (!messages.empty()) {
      Message *message = &messages.front();
      if (!message->image) {
        message->image = create_message_image(device, vp.width, message_font,
                                              message->message.c_str());
        SDL_AddTimer(message->milliseconds, pop_message_callback, NULL);
      }
      cmd->set_texture(0, 0, message->image->get_view(),
                       Vulkan::StockSampler::NearestClamp);
      vp.x = floor(vp.x + (vp.width - message->image->get_width()) / 2);
      vp.y = vp.y + vp.height - message->image->get_height();
      vp.height = message->image->get_height();
      vp.width = message->image->get_width();
      cmd->set_viewport(vp);

      cmd->draw(3);
    } else if (achievement_progress_indicator_image) {
      draw_indicator(cmd, achievement_progress_indicator_image, vp);
    } else if (achievement_challenge_indicator_image &&
               display_challenge_indicator) {
      draw_indicator(cmd, achievement_challenge_indicator_image, vp);
    }

    if (messages.empty() && display_fps && fps_image) {
      draw_fps(cmd, vp);
    }

    cmd->end_render_pass();
  }
  device.submit(cmd);
}

void rdp_set_vi_register(uint32_t reg, uint32_t value) {
  processor->set_vi_register(RDP::VIRegister(reg), value);
}

void rdp_render_frame() {
  auto &device = wsi->get_device();
  render_frame(device);
}

void rdp_update_screen() {
  if (SDL_GetWindowFlags(window) & SDL_WINDOW_MINIMIZED) {
    return;
  }
#ifdef __ANDROID__
  SDL_PropertiesID props = SDL_GetWindowProperties(window);
  void *android_pointer = SDL_GetPointerProperty(
      props, SDL_PROP_WINDOW_ANDROID_WINDOW_POINTER, nullptr);
  if (android_pointer && wsi_platform->is_surface_valid()) {
#endif
    wsi->end_frame();
    wsi->begin_frame();
#ifdef __ANDROID__
  } else {
    SDL_PumpEvents();
  }
#endif
}

CALL_BACK rdp_check_callback() {
  CALL_BACK return_value = callback;
  callback.save_state = false;
  callback.load_rewind = false;
  callback.load_state = false;
  callback.reset_game = false;
  callback.lower_volume = false;
  callback.raise_volume = false;
  callback.decrease_input_delay = false;
  callback.increase_input_delay = false;
  callback.frame_advance = false;
  return return_value;
}

void rdp_check_rdram(uint32_t address, uint32_t length, bool write) {
  uint64_t completed = processor->get_completed_timeline();
  // fast path: everything the RDP has touched is already done
  if (rdram_tracker.max_stamp() <= completed)
    return;

  uint64_t stamp = rdram_tracker.conflict(address, length, write);
  if (stamp <= completed)
    return;
  // commands since the last signal haven't been submitted to the GPU yet
  if (stamp > processor->get_timeline_value())
    processor->signal_timeline();
  processor->wait_for_timeline(stamp);
}

size_t rdp_state_size() {
  return sizeof(RDP_DEVICE) + 0x1000 + processor->get_hidden_rdram_size() +
         sizeof(RDP::CommandProcessor::RenderStateSnapshot);
}

void rdp_save_state(uint8_t *state) {
  memcpy(state, &rdp_device, sizeof(RDP_DEVICE));

  if (g_tmem) {
    memcpy(state + sizeof(RDP_DEVICE), g_tmem, 0x1000);
  }

  if (g_hidden_rdram) {
    memcpy(state + sizeof(RDP_DEVICE) + 0x1000, g_hidden_rdram,
           processor->get_hidden_rdram_size());
  }

  RDP::CommandProcessor::RenderStateSnapshot snapshot = {};
  processor->save_render_state(snapshot);
  memcpy(state + sizeof(RDP_DEVICE) + 0x1000 +
             processor->get_hidden_rdram_size(),
         &snapshot, sizeof(snapshot));
}

void rdp_load_state(GFX_INFO _gfx_info, const uint8_t *state,
                    size_t state_size) {
  gfx_info = _gfx_info;
  if (state_size < rdp_state_size()) {
    return;
  }
  memcpy(&rdp_device, state, sizeof(RDP_DEVICE));

  if (g_tmem) {
    memcpy(g_tmem, state + sizeof(RDP_DEVICE), 0x1000);
  }

  if (g_hidden_rdram) {
    memcpy(g_hidden_rdram, state + sizeof(RDP_DEVICE) + 0x1000,
           processor->get_hidden_rdram_size());
  }

  RDP::CommandProcessor::RenderStateSnapshot snapshot = {};
  memcpy(&snapshot,
         state + sizeof(RDP_DEVICE) + 0x1000 +
             processor->get_hidden_rdram_size(),
         sizeof(snapshot));
  processor->load_render_state(snapshot);
}

static void push_onscreen_message(void *data) {
  Message *message = (Message *)data;
  messages.push(*message);
  delete message;
}

void rdp_onscreen_message(const char *message, MESSAGE_LENGTH milliseconds) {
  Message *data = new Message;
  data->message = message;
  data->milliseconds = static_cast<uint64_t>(milliseconds);
  data->image = Vulkan::ImageHandle();
  SDL_RunOnMainThread(push_onscreen_message, data, false);
}

// Rough RDP timing model used to schedule the DP interrupt after SyncFull.
// busy_until tracks when the RDP will finish its queued work, so that work
// overlapping earlier submissions is not counted twice.
// Each primitive costs its (scissor-clipped) pixel count divided by the fill
// rate of the current cycle type, plus a small fixed setup cost. Fill and copy
// modes write 64 bits per clock; 1-cycle mode is ~1 pixel per clock and
// 2-cycle mode ~2 clocks per pixel. Texture loads move 64 bits per clock.
static const uint64_t RDP_PRIMITIVE_SETUP_CLOCKS = 16;
// Each scanline of a texture load or textured rectangle is a separate RDRAM
// transfer, which pays a fixed overhead on top of its data.
static const uint64_t RDP_SCANLINE_CLOCKS = 16;
// SyncLoad / SyncPipe / SyncTile / SyncFull wait for the pipeline to drain.
static const uint64_t RDP_SYNC_CLOCKS = 50;

// RDP clocks (62.5 MHz) to CPU cycles (93.75 MHz)
static void rdp_add_busy(uint64_t clocks) {
  rdp_device.busy_until += clocks * 3 / 2;
}

static uint64_t rdp_pixel_clocks(uint64_t pixels) {
  switch (rdp_device.cycle_type) {
  case 0: // 1-cycle
    return pixels;
  case 1: // 2-cycle
    return pixels * 2;
  default: { // copy / fill: 64 bits per clock
    uint32_t bits = 4u << rdp_device.frame_buffer_info.framebuffer_pixel_size;
    return (pixels * bits + 63) / 64;
  }
  }
}

// number of scanlines in [top, bottom) that are actually rasterized: in
// interlaced field mode only the odd or even lines are drawn
static uint64_t rdp_scissor_rows(int32_t top, int32_t bottom) {
  if (bottom <= top)
    return 0;
  if (!rdp_device.scissor_field)
    return uint64_t(bottom - top);
  // count lines y in [top, bottom) with (y & 1) == keep_odd
  int32_t p = int32_t(rdp_device.scissor_keep_odd);
  auto lines_below = [p](int32_t y) { return (y - p + 1) >> 1; };
  return uint64_t(lines_below(bottom) - lines_below(top));
}

// coordinates are in 10.2 (rectangles) or s11.2 (triangle Y) fixed point
static uint64_t rdp_clipped_area(int32_t xh, int32_t yh, int32_t xl,
                                 int32_t yl) {
  xh = std::max(xh >> 2, rdp_device.scissor_xh);
  yh = std::max(yh >> 2, rdp_device.scissor_yh);
  xl = std::min(xl >> 2, rdp_device.scissor_xl);
  yl = std::min(yl >> 2, rdp_device.scissor_yl);
  if (xl <= xh)
    return 0;
  return uint64_t(xl - xh) * rdp_scissor_rows(yh, yl);
}

static uint64_t rdp_rectangle_pixels(uint32_t w1, uint32_t w2) {
  int32_t xl = (w1 >> 12) & 0xFFF, yl = w1 & 0xFFF;
  int32_t xh = (w2 >> 12) & 0xFFF, yh = w2 & 0xFFF;
  if (rdp_device.cycle_type >= 2) { // copy/fill include the last row/column
    xl += 4;
    yl += 4;
  }
  return rdp_clipped_area(xh, yh, xl, yl);
}

// number of scanlines a rectangle covers after scissoring
static uint64_t rdp_rectangle_rows(uint32_t w1, uint32_t w2) {
  int32_t yl = w1 & 0xFFF, yh = w2 & 0xFFF;
  if (rdp_device.cycle_type >= 2) // copy/fill include the last row
    yl += 4;
  return rdp_scissor_rows(std::max(yh >> 2, rdp_device.scissor_yh),
                          std::min(yl >> 2, rdp_device.scissor_yl));
}

static int32_t sign_extend_14(uint32_t v) { return int32_t(v << 18) >> 18; }

// area of a triangle from its edge coefficients: half of height times the
// span width at the middle vertex
static uint64_t rdp_triangle_pixels(const uint32_t *cmd) {
  int32_t yl = sign_extend_14(cmd[0]);
  int32_t ym = sign_extend_14(cmd[1] >> 16);
  int32_t yh = sign_extend_14(cmd[1]);
  int32_t xl = int32_t(cmd[2]);
  int32_t xh = int32_t(cmd[4]);
  int32_t dxhdy = int32_t(cmd[5]);
  int64_t major_x_at_ym = int64_t(xh) + int64_t(dxhdy) * (ym - yh) / 4;
  int64_t width = std::llabs(major_x_at_ym - xl) >> 16;
  int32_t sxh = rdp_device.scissor_xh, sxl = rdp_device.scissor_xl;
  width = std::min<int64_t>(width, std::max(sxl - sxh, 0));
  int32_t top = std::max(yh >> 2, rdp_device.scissor_yh);
  int32_t bottom = std::min(yl >> 2, rdp_device.scissor_yl);
  return uint64_t(width) * rdp_scissor_rows(top, bottom) / 2;
}

// keep busy_until relative to COUNT when the CPU rewrites COUNT
void rdp_translate_busy_until(uint64_t old_count, uint64_t new_count) {
  uint64_t remaining =
      rdp_device.busy_until > old_count ? rdp_device.busy_until - old_count : 0;
  rdp_device.busy_until = new_count + remaining;
}

uint64_t rdp_process_commands(uint64_t now) {
  uint64_t interrupt_timer = 0;

  // the RDP starts on this batch once it has finished any earlier work
  if (rdp_device.busy_until < now)
    rdp_device.busy_until = now;

  const uint32_t DP_CURRENT = *gfx_info.DPC_CURRENT_REG & 0x00FFFFF8;
  const uint32_t DP_END = *gfx_info.DPC_END_REG & 0x00FFFFF8;

  int length = DP_END - DP_CURRENT;
  if (length <= 0)
    return interrupt_timer;

  length = unsigned(length) >> 3;
  if ((rdp_device.cmd_ptr + length) & ~(0x0003FFFF >> 3))
    return interrupt_timer;

  uint32_t offset = DP_CURRENT;
  if (*gfx_info.DPC_STATUS_REG & DP_STATUS_XBUS_DMA) {
    do {
      offset &= 0xFF8;
      rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 0] = SDL_Swap32BE(
          *reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset));
      rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 1] = SDL_Swap32BE(
          *reinterpret_cast<const uint32_t *>(gfx_info.DMEM + offset + 4));
      offset += sizeof(uint64_t);
      rdp_device.cmd_ptr++;
    } while (--length > 0);
  } else {
    if (DP_END > 0x7ffffff || DP_CURRENT > 0x7ffffff) {
      return interrupt_timer;
    } else {
      do {
        offset &= (gfx_info.RDRAM_SIZE - 1) & ~uint32_t(7);
        rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 0] =
            *reinterpret_cast<const uint32_t *>(gfx_info.RDRAM + offset);
        rdp_device.cmd_data[2 * rdp_device.cmd_ptr + 1] =
            *reinterpret_cast<const uint32_t *>(gfx_info.RDRAM + offset + 4);
        offset += sizeof(uint64_t);
        rdp_device.cmd_ptr++;
      } while (--length > 0);
    }
  }

  while (rdp_device.cmd_cur - rdp_device.cmd_ptr < 0) {
    uint32_t w1 = rdp_device.cmd_data[2 * rdp_device.cmd_cur];
    uint32_t w2 = rdp_device.cmd_data[2 * rdp_device.cmd_cur + 1];
    uint32_t command = (w1 >> 24) & 63;
    int cmd_length = cmd_len_lut[command];

    if (rdp_device.cmd_ptr - rdp_device.cmd_cur - cmd_length < 0) {
      *gfx_info.DPC_START_REG = *gfx_info.DPC_CURRENT_REG =
          *gfx_info.DPC_END_REG;
      return interrupt_timer;
    }

    if (command >= 8)
      processor->enqueue_command(cmd_length * 2,
                                 &rdp_device.cmd_data[2 * rdp_device.cmd_cur]);

    if (command >= 0x26 && command <= 0x29) { // Sync{Load,Pipe,Tile,Full}
      rdp_add_busy(RDP_SYNC_CLOCKS);
    } else if (command >= 0x08 && command <= 0x0F) {
      rdp_add_busy(RDP_PRIMITIVE_SETUP_CLOCKS +
                   rdp_pixel_clocks(rdp_triangle_pixels(
                       &rdp_device.cmd_data[2 * rdp_device.cmd_cur])));
    } else if (command == 0x24 || command == 0x25) { // TextureRectangle(Flip)
      rdp_add_busy(RDP_PRIMITIVE_SETUP_CLOCKS +
                   rdp_pixel_clocks(rdp_rectangle_pixels(w1, w2)) +
                   rdp_rectangle_rows(w1, w2) * RDP_SCANLINE_CLOCKS);
    } else if (command == 0x36) { // FillRectangle
      rdp_add_busy(RDP_PRIMITIVE_SETUP_CLOCKS +
                   rdp_pixel_clocks(rdp_rectangle_pixels(w1, w2)));
    } else if (command == 0x33 || command == 0x34) { // LoadBlock / LoadTile
      uint32_t sl = (w1 >> 12) & 0xFFF, tl = w1 & 0xFFF;
      uint32_t sh = (w2 >> 12) & 0xFFF, th = w2 & 0xFFF;
      uint64_t texels = command == 0x33
                            ? uint64_t(sh >= sl ? sh - sl + 1 : 0)
                            : uint64_t(sh >= sl ? ((sh - sl) >> 2) + 1 : 0) *
                                  uint64_t(th >= tl ? ((th - tl) >> 2) + 1 : 0);
      // LoadBlock is one contiguous transfer; LoadTile fetches row by row
      uint64_t rows =
          command == 0x33 ? 1 : uint64_t(th >= tl ? ((th - tl) >> 2) + 1 : 0);
      uint32_t bits = 4u << rdp_device.frame_buffer_info.texture_pixel_size;
      rdp_add_busy(RDP_PRIMITIVE_SETUP_CLOCKS + (texels * bits + 63) / 64 +
                   rows * RDP_SCANLINE_CLOCKS);
    }

    switch (RDP::Op(command)) {
    case RDP::Op::FillTriangle:
    case RDP::Op::FillZBufferTriangle:
    case RDP::Op::TextureTriangle:
    case RDP::Op::TextureZBufferTriangle:
    case RDP::Op::ShadeTriangle:
    case RDP::Op::ShadeZBufferTriangle:
    case RDP::Op::ShadeTextureTriangle:
    case RDP::Op::ShadeTextureZBufferTriangle: {
      // s11.2 Y range, rows [yh, yl); X is only bounded by the scissor
      int32_t yl = sign_extend_14(w1), yh = sign_extend_14(w2);
      mark_primitive(0, INT32_MAX, yh >> 2, (yl - 1) >> 2);
    } break;
    case RDP::Op::TextureRectangle:
    case RDP::Op::TextureRectangleFlip:
    case RDP::Op::FillRectangle: {
      // 10.2 coordinates; copy/fill modes include the last row and column
      int32_t xl = (w1 >> 12) & 0xFFF, yl = w1 & 0xFFF;
      int32_t xh = (w2 >> 12) & 0xFFF, yh = w2 & 0xFFF;
      bool copy_fill = rdp_device.cycle_type >= 2;
      if (copy_fill)
        yl |= 3;
      mark_primitive(xh >> 2, copy_fill ? xl >> 2 : (xl - 1) >> 2, yh >> 2,
                     (yl - 1) >> 2);
    } break;
    case RDP::Op::SetColorImage:
      rdp_device.frame_buffer_info.framebuffer_address = (w2 & 0x00FFFFFF);
      rdp_device.frame_buffer_info.framebuffer_pixel_size = (w1 >> 19) & 0x3;
      rdp_device.frame_buffer_info.framebuffer_width = (w1 & 0x3FF) + 1;
      break;
    case RDP::Op::SetMaskImage:
      rdp_device.frame_buffer_info.depthbuffer_address = (w2 & 0x00FFFFFF);
      break;
    case RDP::Op::SetTextureImage:
      rdp_device.frame_buffer_info.texture_pixel_size = (w1 >> 19) & 0x3;
      break;
    case RDP::Op::SetScissor: {
      uint32_t upper_left_x = ((w1 >> 12) & 0xFFF) >> 2;
      uint32_t upper_left_y = (w1 & 0xFFF) >> 2;
      uint32_t lower_right_x = ((w2 >> 12) & 0xFFF) >> 2;
      uint32_t lower_right_y = (w2 & 0xFFF) >> 2;
      rdp_device.scissor_xh = upper_left_x;
      rdp_device.scissor_yh = upper_left_y;
      rdp_device.scissor_xl = lower_right_x;
      rdp_device.scissor_yl = lower_right_y;
      rdp_device.scissor_field = (w2 >> 25) & 1;
      rdp_device.scissor_keep_odd = (w2 >> 24) & 1;

      // parallel-rdp draws subpixel rows/columns in [hi, lo) of the 10.2
      // scissor, so the last pixel touched is (lo - 1) >> 2
      rdp_device.scissor_x_last = (int32_t((w2 >> 12) & 0xFFF) - 1) >> 2;
      rdp_device.scissor_y_last = (int32_t(w2 & 0xFFF) - 1) >> 2;
    } break;
    case RDP::Op::SetOtherModes: {
      uint8_t cycle_type = (w1 >> 20) & 3;
      rdp_device.cycle_type = cycle_type;
      // z_compare_en is bit 4, z_update_en is bit 5; ignored in copy/fill
      bool depth_allowed = (cycle_type & 2) == 0;
      rdp_device.frame_buffer_info.depth_compare =
          depth_allowed && (w2 & (1u << 4));
      rdp_device.frame_buffer_info.depth_update =
          depth_allowed && (w2 & (1u << 5));
    } break;
    case RDP::Op::SyncFull:
      processor->signal_timeline();
      // SyncFull itself adds RDP_SYNC_CLOCKS, so busy_until > now here
      interrupt_timer = rdp_device.busy_until - now;
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

static void rdp_set_fps_callback(void *userdata) {
  FPS_DATA *data = (FPS_DATA *)userdata;
  if (!wsi) {
    delete data;
    return;
  }
  fps_image = create_message_image(
      wsi->get_device(), 0, achievement_challenge_indicator_font,
      std::format("FPS: {} VI/S: {}", data->fps, data->vis).c_str());
  delete data;
}

void rdp_set_fps(uint32_t fps, uint32_t vis) {
  if (display_fps) {
    FPS_DATA *data = new FPS_DATA;
    data->fps = fps;
    data->vis = vis;
    SDL_RunOnMainThread(rdp_set_fps_callback, data, false);
  }
}

static void update_challenge_indicator() {
  std::string message;
  for (const auto &leaderboard_tracker : leaderboard_trackers) {
    message += leaderboard_tracker.second;
    message += '\n';
  }

  if (!leaderboard_trackers.empty() &&
      !achievement_challenge_indicators.empty()) {
    message += "---\n";
  }

  const auto &v = achievement_challenge_indicators;
  for (size_t i = 0; i < std::min<size_t>(v.size(), 5); ++i) {
    message += v[i];
    message += '\n';
  }

  if (message.empty()) {
    achievement_challenge_indicator_image = Vulkan::ImageHandle();
  } else {
    achievement_challenge_indicator_image = create_message_image(
        wsi->get_device(), 0, achievement_challenge_indicator_font,
        message.c_str());
  }
}

void achievement_challenge_indicator_add(const char *achievement_title) {
  achievement_challenge_indicators.push_back(achievement_title);
  update_challenge_indicator();
}

void achievement_challenge_indicator_remove(const char *achievement_title) {
  std::erase(achievement_challenge_indicators, achievement_title);
  update_challenge_indicator();
}

void achievement_progress_add(const char *achievement_title,
                              const char *progress) {
  std::string message = std::format("{}: {}", achievement_title, progress);
  achievement_progress_indicator_image =
      create_message_image(wsi->get_device(), 0, message_font, message.c_str());
}

void achievement_progress_remove() {
  achievement_progress_indicator_image = Vulkan::ImageHandle();
}

void leaderboard_tracker_add(uint32_t id, const char *title,
                             const char *display) {
  if (title) {
    leaderboard_trackers[id] = std::format("{}: {}", title, display);
    update_challenge_indicator();
  }
}

void leaderboard_tracker_remove(uint32_t id) {
  leaderboard_trackers.erase(id);
  update_challenge_indicator();
}
