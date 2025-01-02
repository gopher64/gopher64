#ifndef GLIDEN64_MUPENPLUS_H
#define GLIDEN64_MUPENPLUS_H

#include "m64p_vidext.h"

extern ptr_VidExt_Init                  CoreVideo_Init;
extern ptr_VidExt_Quit                  CoreVideo_Quit;
extern ptr_VidExt_ListFullscreenModes   CoreVideo_ListFullscreenModes;
extern ptr_VidExt_ListFullscreenRates   CoreVideo_ListFullscreenRates;
extern ptr_VidExt_SetVideoMode          CoreVideo_SetVideoMode;
extern ptr_VidExt_SetVideoModeWithRate  CoreVideo_SetVideoModeWithRate;
extern ptr_VidExt_SetCaption            CoreVideo_SetCaption;
extern ptr_VidExt_ToggleFullScreen      CoreVideo_ToggleFullScreen;
extern ptr_VidExt_ResizeWindow          CoreVideo_ResizeWindow;
extern ptr_VidExt_GL_GetProcAddress     CoreVideo_GL_GetProcAddress;
extern ptr_VidExt_GL_SetAttribute       CoreVideo_GL_SetAttribute;
extern ptr_VidExt_GL_GetAttribute       CoreVideo_GL_GetAttribute;
extern ptr_VidExt_GL_SwapBuffers        CoreVideo_GL_SwapBuffers;
extern ptr_VidExt_GL_GetDefaultFramebuffer CoreVideo_GL_GetDefaultFramebuffer;

#endif // GLIDEN64_MUPENPLUS_H
