package io.github.gopher64.gopher64

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.os.PowerManager
import android.util.Log
import android.view.View
import android.view.ViewGroup
import android.widget.RelativeLayout
import org.libsdl.app.SDLActivity

class N64Activity : SDLActivity(), TouchOverlayView.Listener {
    private var leftOverlay: TouchOverlayView? = null
    private var rightOverlay: TouchOverlayView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val powerManager = getContext().getSystemService(Context.POWER_SERVICE) as PowerManager

        if (powerManager.isSustainedPerformanceModeSupported) {
            Log.v("SDL", "Enabling sustained performance mode")
            window.setSustainedPerformanceMode(true)
        } else {
            Log.v("SDL", "Sustained performance mode not supported")
        }

        if (intent?.getBooleanExtra("show_touch_overlay", false) == true) {
            setupTouchOverlay()
        }
    }

    private fun setupTouchOverlay() {
        val layout = mLayout ?: return
        val left = TouchOverlayView(this, TouchOverlayView.Side.LEFT, this)
        val right = TouchOverlayView(this, TouchOverlayView.Side.RIGHT, this)

        val leftParams = RelativeLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT).apply {
            addRule(RelativeLayout.ALIGN_PARENT_LEFT)
            addRule(RelativeLayout.ALIGN_PARENT_TOP)
            addRule(RelativeLayout.ALIGN_PARENT_BOTTOM)
        }
        val rightParams = RelativeLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT).apply {
            addRule(RelativeLayout.ALIGN_PARENT_RIGHT)
            addRule(RelativeLayout.ALIGN_PARENT_TOP)
            addRule(RelativeLayout.ALIGN_PARENT_BOTTOM)
        }

        layout.addView(left, leftParams)
        layout.addView(right, rightParams)
        leftOverlay = left
        rightOverlay = right

        layout.addOnLayoutChangeListener { _, leftBound, top, rightBound, bottom, _, _, _, _ ->
            updateOverlayLayout(rightBound - leftBound, bottom - top)
        }
        layout.post {
            updateOverlayLayout(layout.width, layout.height)
        }
    }

    private fun updateOverlayLayout(width: Int, height: Int) {
        val pillar = TouchOverlayView.pillarWidth(width, height)
        val visible = if (pillar > 0) View.VISIBLE else View.GONE
        leftOverlay?.let { view ->
            view.visibility = visible
            val params = view.layoutParams as RelativeLayout.LayoutParams
            params.width = pillar
            view.layoutParams = params
        }
        rightOverlay?.let { view ->
            view.visibility = visible
            val params = view.layoutParams as RelativeLayout.LayoutParams
            params.width = pillar
            view.layoutParams = params
        }
    }

    override fun onTouchButton(button: Int, pressed: Boolean) {
        nativeTouchButton(button, if (pressed) 1 else 0)
    }

    override fun onTouchAxis(x: Int, y: Int) {
        nativeTouchAxis(x, y)
    }

    override fun getLibraries(): Array<String> = arrayOf(
        "SDL3",
        "SDL3_ttf",
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

    private external fun nativeTouchButton(button: Int, pressed: Int)
    private external fun nativeTouchAxis(x: Int, y: Int)
}
