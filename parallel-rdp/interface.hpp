#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif

	typedef struct
	{
		uint8_t *RDRAM;
		uint8_t *DMEM;
		uint32_t RDRAM_SIZE;
		uint32_t *DPC_CURRENT_REG;
		uint32_t *DPC_START_REG;
		uint32_t *DPC_END_REG;
		uint32_t *DPC_STATUS_REG;
		uint32_t *VI_H_START_REG;
		uint32_t *VI_V_START_REG;
		uint32_t *VI_X_SCALE_REG;
		uint32_t *VI_Y_SCALE_REG;
		uint32_t *VI_WIDTH_REG;
	} GFX_INFO;

	void rdp_init(void *_window, GFX_INFO _gfx_info, bool fullscreen, bool _upscale);
	void rdp_close();
	void rdp_set_vi_register(uint32_t reg, uint32_t value);
	bool rdp_update_screen();
	uint64_t rdp_process_commands();
	void rdp_full_sync();

#ifdef __cplusplus
}
#endif
