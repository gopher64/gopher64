#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif

	void rdp_init(void *_window, GFX_INFO _gfx_info, bool fullscreen);
	void rdp_close();
	void rdp_set_vi_register(uint32_t reg, uint32_t value);
	bool rdp_update_screen();
	uint64_t rdp_process_commands();
	void rdp_full_sync();
	int sdl_event_filter(void *userdata, SDL_Event *event);

#ifdef __cplusplus
}
#endif
