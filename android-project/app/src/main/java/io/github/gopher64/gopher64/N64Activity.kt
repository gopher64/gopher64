package io.github.gopher64.gopher64

import android.content.Intent
import android.os.PowerManager
import android.content.Context
import android.os.Bundle
import android.util.Log
import org.libsdl.app.SDLActivity

class N64Activity : SDLActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val powerManager = getContext().getSystemService(Context.POWER_SERVICE) as PowerManager

        if (powerManager.isSustainedPerformanceModeSupported) {
            Log.v("SDL", "Enabling sustained performance mode")
            window.setSustainedPerformanceMode(true)
        } else {
            Log.v("SDL", "Sustained performance mode not supported")
        }
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
        val args = intent.getStringArrayExtra("args") ?: return super.getArguments()

        val dataIntent = Intent()
        val file_path = intent.getStringExtra("file_path")
        if (file_path != null) {
            dataIntent.putExtra("file_path", file_path)
        }
        val cheats_path = intent.getStringExtra("cheats_path")
        if (cheats_path != null) {
            dataIntent.putExtra("cheats_path", cheats_path)
        }
        setResult(RESULT_OK, dataIntent)
        return args
    }
}
