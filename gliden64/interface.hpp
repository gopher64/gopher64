#pragma once

#ifdef __cplusplus
#include <cstdint>
#include "m64p_plugin.h"

extern "C"
{
#endif

    void hle_init(GFX_INFO _gfx_info);
    void hle_close();
    uint64_t hle_process_dlist();
    bool hle_update_screen();

#ifdef __cplusplus
}
#endif
