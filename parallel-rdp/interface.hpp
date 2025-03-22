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
		bool widescreen;
		bool fullscreen;
		bool integer_scaling;
		uint32_t upscale;
		bool crt;
	} GFX_INFO;

	typedef struct
	{
		bool emu_running;
		bool save_state;
		bool load_state;
		bool enable_speedlimiter;
		bool lower_volume;
		bool raise_volume;
	} CALL_BACK;

	void rdp_init(void *_window, GFX_INFO _gfx_info);
	void rdp_close();
	void rdp_set_vi_register(uint32_t reg, uint32_t value);
	void rdp_update_screen();
	CALL_BACK rdp_check_callback();
	uint64_t rdp_process_commands();
	void rdp_new_processor(GFX_INFO _gfx_info);
	void rdp_check_framebuffers(uint32_t address);
	uint64_t rdp_state_size();
	void rdp_save_state(uint8_t *state);
	void rdp_load_state(uint8_t *state);

#ifdef __cplusplus
}
#endif
