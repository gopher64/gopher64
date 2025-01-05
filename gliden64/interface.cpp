#include "interface.hpp"
#include "m64p_frontend.h"
#include "Config.h"
#include "GLFunctions.h"
#include <Graphics/OpenGLContext/ThreadedOpenGl/opengl_Wrapper.h>
#include <DisplayWindow.h>
#include <PluginAPI.h>
#include <N64.h>
#include <SDL.h>

using namespace opengl;

Config config;
ptr_DebugCallback CoreDebugCallback = nullptr;
void *CoreDebugCallbackContext = nullptr;
static SDL_Window *window;
static bool emu_running = false;
static bool fullscreen = false;

int hle_sdl_event_filter(void *userdata, SDL_Event *event)
{
	if (event->type == SDL_WINDOWEVENT)
	{
		switch (event->window.event)
		{
		case SDL_WINDOWEVENT_CLOSE:
			emu_running = false;
			break;
		case SDL_WINDOWEVENT_RESIZED:
			// wsi_platform->do_resize();
			break;
		default:
			break;
		}
	}
	else if (fullscreen && event->type == SDL_KEYDOWN)
	{
		switch (event->key.keysym.scancode)
		{
		case SDL_SCANCODE_ESCAPE:
			emu_running = false;
			break;
		default:
			break;
		}
	}

	return 0;
}

void hle_init(void *_window, GFX_INFO _gfx_info, bool _fullscreen)
{
	window = (SDL_Window *)_window;
	SDL_SetEventFilter(hle_sdl_event_filter, nullptr);
	api().InitiateGFX(_gfx_info);
	api().RomOpen();
	fullscreen = _fullscreen;
	emu_running = true;
}

void hle_close()
{
	api().RomClosed();
}

uint64_t hle_process_dlist()
{
	api().ProcessDList();
	return 4000;
}

bool hle_update_screen()
{
	api().UpdateScreen();
	return emu_running;
}

class DisplayWindowMupen64plus : public DisplayWindow
{
public:
	DisplayWindowMupen64plus() {}

private:
	void _setAttributes();
	void _getDisplaySize();

	bool _start() override;
	void _stop() override;
	void _restart() override;
	void _swapBuffers() override;
	void _saveScreenshot() override;
	void _saveBufferContent(graphics::ObjectHandle _fbo, CachedTexture *_pTexture) override;
	bool _resizeWindow() override;
	void _changeWindow() override;
	void _readScreen(void **_pDest, long *_pWidth, long *_pHeight) override {}
	void _readScreen2(void *_dest, int *_width, int *_height, int _front) override;
	graphics::ObjectHandle _getDefaultFramebuffer() override;
};

DisplayWindow &DisplayWindow::get()
{
	static DisplayWindowMupen64plus video;
	return video;
}

void DisplayWindowMupen64plus::_setAttributes()
{
	SDL_GL_SetSwapInterval(0);
}

bool DisplayWindowMupen64plus::_start()
{
	FunctionWrapper::setThreadedMode(0);

	_setAttributes();

	m_bFullscreen = fullscreen;
	int w, h;
	SDL_GL_GetDrawableSize(window, &w, &h);
	m_screenWidth = w;
	m_screenHeight = h;

	_getDisplaySize();
	_setBufferSize();

	initGLFunctions();
	return true;
}

void DisplayWindowMupen64plus::_stop()
{
}

void DisplayWindowMupen64plus::_restart()
{
}

void DisplayWindowMupen64plus::_swapBuffers()
{
	SDL_GL_SwapWindow(window);
}

void DisplayWindowMupen64plus::_saveScreenshot()
{
}

void DisplayWindowMupen64plus::_saveBufferContent(graphics::ObjectHandle /*_fbo*/, CachedTexture * /*_pTexture*/)
{
}

bool DisplayWindowMupen64plus::_resizeWindow()
{
	return true;
}

void DisplayWindowMupen64plus::_changeWindow()
{
}

void DisplayWindowMupen64plus::_getDisplaySize()
{
}

void DisplayWindowMupen64plus::_readScreen2(void *_dest, int *_width, int *_height, int _front)
{
}

graphics::ObjectHandle DisplayWindowMupen64plus::_getDefaultFramebuffer()
{
	return graphics::ObjectHandle::null;
}

int PluginAPI::InitiateGFX(const GFX_INFO &_gfx_info)
{
	_initiateGFX(_gfx_info);

	REG.SP_STATUS = _gfx_info.SP_STATUS_REG;
	RDRAMSize = *_gfx_info.RDRAM_SIZE - 1;

	return TRUE;
}

void PluginAPI::GetUserDataPath(wchar_t *_strPath)
{
}

void PluginAPI::GetUserCachePath(wchar_t *_strPath)
{
}

void PluginAPI::FindPluginPath(wchar_t *_strPath)
{
}

void Config_LoadConfig()
{
	config.resetToDefaults();
}
