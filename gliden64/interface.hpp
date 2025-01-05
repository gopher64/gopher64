#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif

    void hle_init();
    void hle_close();
    uint64_t hle_process_dlist();
    bool hle_update_screen();

#ifdef __cplusplus
}
#endif
