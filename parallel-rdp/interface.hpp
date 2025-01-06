#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif

	void lle_init(void *_window, GFX_INFO _gfx_info, bool fullscreen);
	void lle_close();
	void lle_set_vi_register(uint32_t reg, uint32_t value);
	bool lle_update_screen();
	uint64_t rdp_process_commands();
	void lle_full_sync();

#ifdef __cplusplus
}
#endif
