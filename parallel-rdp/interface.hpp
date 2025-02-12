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
		bool PAL;
	} GFX_INFO;

	typedef struct
	{
		bool emu_running;
		bool save_state;
		bool load_state;
	} CALL_BACK;

	void rdp_init(void *_window, GFX_INFO _gfx_info, bool _upscale, bool _integer_scaling, bool _fullscreen);
	void rdp_close();
	void rdp_set_vi_register(uint32_t reg, uint32_t value);
	void rdp_update_screen();
	CALL_BACK rdp_check_callback();
	uint64_t rdp_process_commands();
	void rdp_new_processor(GFX_INFO _gfx_info, bool _upscale);

#ifdef __cplusplus
}
#endif
