package io.github.gopher64.gopher64

import android.app.NativeActivity
import android.content.Intent
import android.graphics.PixelFormat
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager

class SlintActivity : NativeActivity() {
    companion object {
        init {
            System.loadLibrary("gopher64")
        }

        // MotionEvent axes the profile wizard can bind — the same set
        // SDLControllerManager reports to the game (sticks, both trigger
        // conventions); the dpad hat is handled separately below.
        private val CAPTURE_AXES = intArrayOf(
            MotionEvent.AXIS_X, MotionEvent.AXIS_Y,
            MotionEvent.AXIS_Z, MotionEvent.AXIS_RZ,
            MotionEvent.AXIS_LTRIGGER, MotionEvent.AXIS_RTRIGGER,
            MotionEvent.AXIS_GAS, MotionEvent.AXIS_BRAKE,
        )
        private val HAT_AXES = intArrayOf(MotionEvent.AXIS_HAT_X, MotionEvent.AXIS_HAT_Y)
    }

    private external fun nativeOnActivityResult(requestCode: Int, resultCode: Int, data: Intent?)

    // type: 0 = key down (code = Android keycode, value/action unused),
    // 1 = motion axis (code = MotionEvent.AXIS_*, value normalized to -1..1
    // exactly like SDLControllerManager, action = resting-position sign).
    private external fun nativeOnCaptureInput(
        deviceId: Int, source: Int, type: Int, code: Int, value: Float, action: Int
    )

    @Volatile
    private var captureActive = false
    private val mainHandler = Handler(Looper.getMainLooper())
    private var captureOverlay: View? = null
    // Last forwarded value per (deviceId, axis): forward changes only, like SDL.
    private val lastAxisValues = HashMap<Long, Float>()

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)

        nativeOnActivityResult(requestCode, resultCode, data)
    }

    // Called from Rust (JNI) when the input-profile wizard opens/closes.
    //
    // Slint consumes events at the native InputQueue stage (keys it maps, and
    // ALL touch/motion ACTION_MOVE), so the Activity dispatch overrides below
    // never see dpad keys (mapped to arrows) or joystick motion (consumed as
    // touch). While capturing we therefore attach an invisible focused-but-
    // not-touchable overlay window: window focus routes key and joystick
    // events to it, while touch keeps hitting the Slint window underneath.
    fun setCaptureActive(active: Boolean) {
        captureActive = active
        mainHandler.post { if (active) addOverlay() else removeOverlay() }
    }

    private fun addOverlay() {
        if (captureOverlay != null) return
        val view = object : View(this) {
            override fun dispatchKeyEvent(event: KeyEvent): Boolean =
                captureKey(event) || super.dispatchKeyEvent(event)

            override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean =
                captureMotion(event) || super.dispatchGenericMotionEvent(event)
        }
        view.isFocusable = true
        view.isFocusableInTouchMode = true
        val params = WindowManager.LayoutParams(
            1, 1,
            WindowManager.LayoutParams.TYPE_APPLICATION_PANEL,
            WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE
                or WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL,
            PixelFormat.TRANSLUCENT,
        )
        try {
            windowManager.addView(view, params)
        } catch (e: Exception) {
            // Activity going away; the dispatch overrides still capture buttons.
            Log.w("gopher64", "capture overlay not attached: $e")
            return
        }
        view.requestFocus()
        captureOverlay = view
    }

    private fun removeOverlay() {
        captureOverlay?.let {
            try {
                windowManager.removeView(it)
            } catch (e: Exception) {
                Log.w("gopher64", "capture overlay not removed: $e")
            }
        }
        captureOverlay = null
        lastAxisValues.clear()
    }

    override fun onDestroy() {
        removeOverlay()
        super.onDestroy()
    }

    // Fallback path: events Slint leaves unhandled (gamepad buttons) reach the
    // Activity even without the overlay.
    override fun dispatchKeyEvent(event: KeyEvent): Boolean =
        captureKey(event) || super.dispatchKeyEvent(event)

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean =
        captureMotion(event) || super.dispatchGenericMotionEvent(event)

    // Containment must be (source and S) == S — Android SOURCE_* values share
    // class bits (a keyboard is 0x101, GAMEPAD 0x401: `!= 0` would match).
    private fun isController(source: Int): Boolean =
        source and InputDevice.SOURCE_GAMEPAD == InputDevice.SOURCE_GAMEPAD ||
            source and InputDevice.SOURCE_JOYSTICK == InputDevice.SOURCE_JOYSTICK ||
            source and InputDevice.SOURCE_DPAD == InputDevice.SOURCE_DPAD

    // Swallow (and forward on first DOWN) every key the wizard can use:
    // controller buttons, BACK, and non-system keyboard keys. Other system
    // keys (volume, power, ...) stay with the platform.
    private fun captureKey(event: KeyEvent): Boolean {
        if (!captureActive) return false
        if (!isController(event.source) &&
            event.keyCode != KeyEvent.KEYCODE_BACK && event.isSystem
        ) {
            return false
        }
        if (event.action == KeyEvent.ACTION_DOWN && event.repeatCount == 0) {
            nativeOnCaptureInput(event.deviceId, event.source, 0, event.keyCode, 0f, 0)
        }
        return true
    }

    private fun captureMotion(event: MotionEvent): Boolean {
        if (!captureActive) return false
        if (!isController(event.source) || event.actionMasked != MotionEvent.ACTION_MOVE) {
            return false
        }
        val device = event.device ?: return true
        for (axis in CAPTURE_AXES) {
            val range = device.getMotionRange(axis, event.source) ?: continue
            // Normalize to -1..1 exactly like SDLControllerManager, so what we
            // capture matches what the game will see from SDL.
            val value = (event.getAxisValue(axis) - range.min) / range.range * 2f - 1f
            val key = event.deviceId.toLong() shl 32 or (axis.toLong() and 0xffffffffL)
            if (lastAxisValues[key] == value) continue
            lastAxisValues[key] = value
            val restNorm = (0f.coerceIn(range.min, range.max) - range.min) / range.range * 2f - 1f
            val restSign = if (restNorm < -0.5f) -1 else if (restNorm > 0.5f) 1 else 0
            nativeOnCaptureInput(event.deviceId, event.source, 1, axis, value, restSign)
        }
        // The dpad arrives as HAT_X/HAT_Y motion; SDL translates it into DPAD
        // buttons for the game, so synthesize DPAD keycodes on press edges.
        for (axis in HAT_AXES) {
            device.getMotionRange(axis, event.source) ?: continue
            val value = Math.round(event.getAxisValue(axis)).toFloat()
            val key = event.deviceId.toLong() shl 32 or (axis.toLong() and 0xffffffffL)
            val last = lastAxisValues[key] ?: 0f
            if (last == value) continue
            lastAxisValues[key] = value
            if (value == 0f) continue // releases are not captured
            val keycode = if (axis == MotionEvent.AXIS_HAT_X) {
                if (value < 0f) KeyEvent.KEYCODE_DPAD_LEFT else KeyEvent.KEYCODE_DPAD_RIGHT
            } else {
                if (value < 0f) KeyEvent.KEYCODE_DPAD_UP else KeyEvent.KEYCODE_DPAD_DOWN
            }
            nativeOnCaptureInput(event.deviceId, event.source, 0, keycode, 0f, 0)
        }
        return true
    }
}
