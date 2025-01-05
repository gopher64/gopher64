#include "interface.hpp"
#include "m64p_frontend.h"
#include "Config.h"
#include <RSP.h>
#include <VI.h>
#include <DisplayWindow.h>
#include <PluginAPI.h>

Config config;
ptr_DebugCallback CoreDebugCallback = nullptr;
void *CoreDebugCallbackContext = nullptr;

uint64_t hle_process_dlist()
{
    RSP_ProcessDList();
    return 4000;
}

bool hle_update_screen()
{
    VI_UpdateScreen();
    return true;
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
}

bool DisplayWindowMupen64plus::_start()
{
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

void PluginAPI::GetUserDataPath(wchar_t *_strPath)
{
}

void PluginAPI::GetUserCachePath(wchar_t *_strPath)
{
}
