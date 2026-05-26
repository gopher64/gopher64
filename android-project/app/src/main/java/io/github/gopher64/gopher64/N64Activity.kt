package io.github.gopher64.gopher64

import android.content.Intent
import org.libsdl.app.SDLActivity

class N64Activity : SDLActivity() {
    companion object {
        const val CONFIGURE_INPUT_PROFILE = 2
        const val RUN_ROM = 3
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
            val deadzone = intent.getIntExtra("deadzone", -1)
            val args = mutableListOf(
                "--configure-input-profile",
                profile,
            )
            if (intent.getBooleanExtra("dinput", false)) {
                args.add("--use-dinput")
            }
            if (deadzone != -1) {
                args.add("--deadzone")
                args.add(deadzone.toString())
            }
            setResult(RESULT_OK) // so that the profiles are updated in the GUI
            return args.toTypedArray()
        } else if (request_code == RUN_ROM) {
            val file_path = intent.getStringExtra("file_path") ?: return super.getArguments()
            val overclock = intent.getBooleanExtra("overclock", false)
            val disable_expansion_pak = intent.getBooleanExtra("disable_expansion_pak", false)
            val args = mutableListOf(
                file_path,
                "--fullscreen",
                "--overclock",
                overclock.toString(),
                "--disable-expansion-pak",
                disable_expansion_pak.toString())

            val netplay_peer_addr = intent.getStringExtra("netplay_peer_addr")
            val cheats = intent.getStringExtra("cheats")
            if (netplay_peer_addr != null && cheats != null) {
                args.add("--netplay-peer-addr")
                args.add(netplay_peer_addr)
                args.add("--netplay-player-number")
                args.add(intent.getIntExtra("netplay_player_number", 4).toString())
                args.add("--cheats")
                args.add(cheats)
            }
            val dataIntent = Intent()
            dataIntent.putExtra("file_path", file_path)
            dataIntent.putExtra("cheats_path", cheats)
            setResult(RESULT_OK, dataIntent)
            return args.toTypedArray()
        } else {
            return super.getArguments()
        }
    }
}
