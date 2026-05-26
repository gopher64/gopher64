package io.github.gopher64.gopher64

import org.libsdl.app.SDLActivity

class N64Activity : SDLActivity() {
    companion object {
        const val CONFIGURE_INPUT_PROFILE = 2
    }

    override fun getLibraries(): Array<String> = arrayOf(
        "SDL3",
        "SDL3_ttf",
        "SDL3_image",
        "gopher64"
    )

    override fun getMainFunction(): String = "gopher64_sdl_main"

    override fun getArguments(): Array<String> {
        val intent = intent ?: return super.getArguments()
        val request_code = intent.getIntExtra("request_code", 0)
        if (request_code == CONFIGURE_INPUT_PROFILE) {
            val profile = intent.getStringExtra("profile_name") ?: return super.getArguments()
            val args = mutableListOf(
                "--configure-input-profile",
                profile
            )
            return args.toTypedArray()
        } else {
            return super.getArguments()
        }
    }
}
