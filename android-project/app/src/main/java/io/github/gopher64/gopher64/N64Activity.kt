package io.github.gopher64.gopher64

import org.libsdl.app.SDLActivity

class N64Activity : SDLActivity() {
    override fun getLibraries(): Array<String> = arrayOf(
        "SDL3",
        "SDL3_ttf",
        "SDL3_image",
        "gopher64"
    )
}
