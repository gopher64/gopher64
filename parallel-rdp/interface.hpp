#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif

	typedef struct
	{
		unsigned char *RDRAM;
		unsigned char *DMEM;
		const unsigned int *RDRAM_SIZE;
		unsigned int *DPC_CURRENT_REG;
		unsigned int *DPC_START_REG;
		unsigned int *DPC_END_REG;
		unsigned int *DPC_STATUS_REG;
		unsigned int *VI_H_START_REG;
		unsigned int *VI_V_START_REG;
		unsigned int *VI_X_SCALE_REG;
		unsigned int *VI_Y_SCALE_REG;
		unsigned int *VI_WIDTH_REG;
	} GFX_INFO;

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
