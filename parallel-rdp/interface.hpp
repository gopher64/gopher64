#pragma once

#ifdef __cplusplus
#include <cstdint>
#include "m64p_plugin.h"

extern "C"
{
#endif

   void lle_init(GFX_INFO _gfx_info, bool fullscreen);
   void lle_close();
   void lle_set_sdl_window(void *_window);
   void lle_set_vi_register(uint32_t reg, uint32_t value);
   bool lle_update_screen();
   uint64_t rdp_process_commands();
   void lle_full_sync();

#ifdef __cplusplus
}
#endif
