#pragma once

#ifdef __cplusplus
#include <cstdint>
#include <stddef.h>

extern "C" {
#endif

typedef struct {
  uint8_t *RDRAM;
  uint8_t *DMEM;
  uint32_t RDRAM_SIZE;
  uint32_t *DPC_CURRENT_REG;
  uint32_t *DPC_START_REG;
  uint32_t *DPC_END_REG;
  uint32_t *DPC_STATUS_REG;
  bool PAL;
  bool widescreen;
  bool fullscreen;
  bool vsync;
  bool integer_scaling;
  uint32_t upscale;
  bool crt;
} GFX_INFO;

typedef struct {
  bool emu_running;
  bool save_state;
  bool load_state;
  bool reset_game;
  bool enable_speedlimiter;
  bool lower_volume;
  bool raise_volume;
  bool paused;
  bool frame_advance;
  uint32_t save_state_slot;
} CALL_BACK;

typedef enum {
  MESSAGE_VERY_SHORT = 500,
  MESSAGE_SHORT = 3000,
  MESSAGE_LONG = 6000,
} MESSAGE_LENGTH;

void rdp_init(void *_window, GFX_INFO _gfx_info, const void *font,
              size_t font_size, uint32_t save_state_slot);
void rdp_close();
void rdp_set_vi_register(uint32_t reg, uint32_t value);
void rdp_update_screen();
void rdp_render_frame();
CALL_BACK rdp_check_callback();
uint64_t rdp_process_commands();
void rdp_onscreen_message(const char *message, MESSAGE_LENGTH milliseconds);
void rdp_new_processor(GFX_INFO _gfx_info);
void rdp_check_framebuffers(uint32_t address, uint32_t length);
size_t rdp_state_size();
void rdp_save_state(uint8_t *state);
void rdp_load_state(const uint8_t *state);
void rdp_set_fps(uint32_t fps, uint32_t vis);

void achievement_challenge_indicator_add(const char *achievement_title);
void achievement_challenge_indicator_remove(const char *achievement_title);
void achievement_progress_add(const char *achievement_title,
                              const char *progress);
void achievement_progress_remove();
void leaderboard_tracker_add(uint32_t id, const char *title,
                             const char *display);
void leaderboard_tracker_remove(uint32_t id);

#ifdef __cplusplus
}
#endif
