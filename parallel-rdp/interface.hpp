#pragma once

#ifdef __cplusplus
#include <cstdint>

extern "C"
{
#endif
   typedef struct
   {
      unsigned char *HEADER; /* This is the rom header (first 40h bytes of the rom) */
      unsigned char *RDRAM;
      unsigned char *DMEM;
      unsigned char *IMEM;

      unsigned int *MI_INTR_REG;

      unsigned int *DPC_START_REG;
      unsigned int *DPC_END_REG;
      unsigned int *DPC_CURRENT_REG;
      unsigned int *DPC_STATUS_REG;
      unsigned int *DPC_CLOCK_REG;
      unsigned int *DPC_BUFBUSY_REG;
      unsigned int *DPC_PIPEBUSY_REG;
      unsigned int *DPC_TMEM_REG;

      unsigned int *VI_STATUS_REG;
      unsigned int *VI_ORIGIN_REG;
      unsigned int *VI_WIDTH_REG;
      unsigned int *VI_INTR_REG;
      unsigned int *VI_V_CURRENT_LINE_REG;
      unsigned int *VI_TIMING_REG;
      unsigned int *VI_V_SYNC_REG;
      unsigned int *VI_H_SYNC_REG;
      unsigned int *VI_LEAP_REG;
      unsigned int *VI_H_START_REG;
      unsigned int *VI_V_START_REG;
      unsigned int *VI_V_BURST_REG;
      unsigned int *VI_X_SCALE_REG;
      unsigned int *VI_Y_SCALE_REG;

      void (*CheckInterrupts)(void);

      /* The GFX_INFO.version parameter was added in version 2.5.1 of the core.
         Plugins should ensure the core is at least this version before
         attempting to read GFX_INFO.version. */
      unsigned int version;
      /* SP_STATUS_REG and RDRAM_SIZE were added in version 2 of GFX_INFO.version.
         Plugins should only attempt to read these values if GFX_INFO.version is at least 2. */

      /* The RSP plugin should set (HALT | BROKE | TASKDONE) *before* calling ProcessDList.
         It should not modify SP_STATUS_REG after ProcessDList has returned.
         This will allow the GFX plugin to unset these bits if it needs. */
      unsigned int *SP_STATUS_REG;
      const unsigned int *RDRAM_SIZE;
   } GFX_INFO;

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
