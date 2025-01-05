#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif

    void lle_init(void *mem_base, uint32_t rdram_size, uint8_t fullscreen);
    void lle_close();
    void lle_set_sdl_window(void *_window);
    void lle_set_vi_register(uint32_t reg, uint32_t value);
    bool lle_update_screen();
    uint64_t rdp_process_commands(uint32_t *dpc_regs, uint8_t *SP_DMEM);
    void lle_full_sync();

#ifdef __cplusplus
}
#endif
