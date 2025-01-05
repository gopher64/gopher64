#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif

    uint64_t hle_process_dlist();
    bool hle_update_screen();

#ifdef __cplusplus
}
#endif
