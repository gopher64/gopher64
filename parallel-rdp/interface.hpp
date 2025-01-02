#pragma once

#ifdef __cplusplus
extern "C"
{
#endif

    void vk_init(void *mem_base, uint32_t rdram_size, uint8_t fullscreen);
    void vk_close();
    void vk_set_sdl_window(void *_window);
    void rdp_set_vi_register(uint32_t reg, uint32_t value);
    uint8_t rdp_update_screen();
    uint64_t rdp_process_commands(uint32_t *dpc_regs, uint8_t *SP_DMEM);
    void full_sync();

#ifdef __cplusplus
}
#endif
