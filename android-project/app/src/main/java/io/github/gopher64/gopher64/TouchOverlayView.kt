package io.github.gopher64.gopher64

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PointF
import android.util.TypedValue
import android.view.MotionEvent
import android.view.View
import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

/**
 * On-screen N64 controls for one side of the screen (left or right pillar).
 *
 * Button indices match gopher64's N64 bit order (R_DPAD=0 … L_TRIG=13).
 */
class TouchOverlayView(
    context: Context,
    private val side: Side,
    private val listener: Listener,
) : View(context) {

    enum class Side { LEFT, RIGHT }

    interface Listener {
        fun onTouchButton(button: Int, pressed: Boolean)
        fun onTouchAxis(x: Int, y: Int)
    }

    private data class ButtonControl(
        val button: Int,
        val label: String,
        val center: PointF,
        val radius: Float,
        val fillColor: Int = Color.argb(90, 255, 255, 255),
        val pressedColor: Int = Color.argb(160, 255, 255, 255),
        var pointerId: Int = -1,
    )

    private data class StickControl(
        val center: PointF,
        val radius: Float,
        var knobOffsetX: Float = 0f,
        var knobOffsetY: Float = 0f,
        var pointerId: Int = -1,
    )

    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = dp(2f)
        color = Color.argb(200, 255, 255, 255)
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.argb(220, 255, 255, 255)
        textAlign = Paint.Align.CENTER
        isFakeBoldText = true
    }

    private var buttons: List<ButtonControl> = emptyList()
    private var stick: StickControl? = null

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        layoutControls(w.toFloat(), h.toFloat())
    }

    private fun layoutControls(w: Float, h: Float) {
        if (w <= 0f || h <= 0f) {
            buttons = emptyList()
            stick = null
            return
        }

        val pad = dp(8f)
        val usableW = max(1f, w - pad * 2f)
        val cx = w / 2f
        val buttonR = min(usableW * 0.22f, h * 0.055f)
        textPaint.textSize = buttonR * 0.9f

        when (side) {
            Side.LEFT -> {
                // Keep stick and D-pad separated: D-pad upper-middle, stick lower.
                val stickR = min(usableW * 0.38f, h * 0.13f)
                val dpadCy = h * 0.34f
                val dpadSpread = buttonR * 1.85f
                val dpadBottom = dpadCy + dpadSpread + buttonR
                val zR = buttonR
                val zGap = dp(10f)
                // Leave room above the stick for Z between D-pad and stick.
                val stickCy = max(
                    h * 0.78f,
                    dpadBottom + zR * 2f + zGap * 2f + stickR + dp(8f),
                )
                val stickCenter = PointF(cx, min(stickCy, h - stickR - pad))
                stick = StickControl(stickCenter, stickR)

                // Z sits above the stick on its left side, with a clear gap.
                val zCenter = PointF(
                    max(pad + zR, stickCenter.x - stickR * 0.65f),
                    stickCenter.y - stickR - zR - zGap,
                )

                buttons = listOf(
                    ButtonControl(L_TRIG, "L", PointF(cx, pad + buttonR), buttonR * 1.1f),
                    ButtonControl(Z_TRIG, "Z", zCenter, zR),
                    ButtonControl(U_DPAD, "↑", PointF(cx, dpadCy - dpadSpread), buttonR),
                    ButtonControl(D_DPAD, "↓", PointF(cx, dpadCy + dpadSpread), buttonR),
                    ButtonControl(L_DPAD, "←", PointF(cx - dpadSpread, dpadCy), buttonR),
                    ButtonControl(R_DPAD, "→", PointF(cx + dpadSpread, dpadCy), buttonR),
                )
            }
            Side.RIGHT -> {
                stick = null
                val rR = buttonR * 1.1f
                val startR = buttonR * 0.85f
                val cR = buttonR * 0.85f
                val faceR = buttonR * 1.15f
                val cSpread = buttonR * 1.55f
                val faceSpread = buttonR * 1.55f
                val gap = dp(10f)
                val faceCGap = dp(28f)

                // Anchor A/B and C as low as possible, with clear space between them.
                val faceY = h - pad - faceR
                val cY = faceY - faceR - faceCGap - cSpread
                val cTop = cY - cSpread - cR

                // Keep R and Start clearly separated in the upper area.
                val rY = pad + rR
                val startY = min(rY + rR + gap + startR, cTop - gap - startR)

                buttons = listOf(
                    ButtonControl(R_TRIG, "R", PointF(cx, rY), rR),
                    ButtonControl(
                        START_BUTTON, "S", PointF(cx, startY), startR,
                        fillColor = Color.argb(120, 220, 40, 40),
                        pressedColor = Color.argb(200, 255, 70, 70),
                    ),
                    ButtonControl(
                        U_CBUTTON, "C↑", PointF(cx, cY - cSpread), cR,
                        fillColor = Color.argb(120, 230, 200, 40),
                        pressedColor = Color.argb(200, 255, 230, 80),
                    ),
                    ButtonControl(
                        D_CBUTTON, "C↓", PointF(cx, cY + cSpread), cR,
                        fillColor = Color.argb(120, 230, 200, 40),
                        pressedColor = Color.argb(200, 255, 230, 80),
                    ),
                    ButtonControl(
                        L_CBUTTON, "C←", PointF(cx - cSpread, cY), cR,
                        fillColor = Color.argb(120, 230, 200, 40),
                        pressedColor = Color.argb(200, 255, 230, 80),
                    ),
                    ButtonControl(
                        R_CBUTTON, "C→", PointF(cx + cSpread, cY), cR,
                        fillColor = Color.argb(120, 230, 200, 40),
                        pressedColor = Color.argb(200, 255, 230, 80),
                    ),
                    ButtonControl(
                        B_BUTTON, "B", PointF(cx - faceSpread, faceY), faceR,
                        fillColor = Color.argb(120, 40, 180, 70),
                        pressedColor = Color.argb(200, 70, 220, 100),
                    ),
                    ButtonControl(
                        A_BUTTON, "A", PointF(cx + faceSpread, faceY), faceR,
                        fillColor = Color.argb(120, 50, 90, 220),
                        pressedColor = Color.argb(200, 80, 120, 255),
                    ),
                )
            }
        }
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        for (button in buttons) {
            fillPaint.color = if (button.pointerId >= 0) button.pressedColor else button.fillColor
            canvas.drawCircle(button.center.x, button.center.y, button.radius, fillPaint)
            canvas.drawCircle(button.center.x, button.center.y, button.radius, strokePaint)
            val textY = button.center.y - (textPaint.descent() + textPaint.ascent()) / 2f
            canvas.drawText(button.label, button.center.x, textY, textPaint)
        }

        stick?.let { s ->
            fillPaint.color = if (s.pointerId >= 0) {
                Color.argb(160, 255, 255, 255)
            } else {
                Color.argb(90, 255, 255, 255)
            }
            canvas.drawCircle(s.center.x, s.center.y, s.radius, fillPaint)
            canvas.drawCircle(s.center.x, s.center.y, s.radius, strokePaint)
            val knobR = s.radius * 0.38f
            val kx = s.center.x + s.knobOffsetX
            val ky = s.center.y + s.knobOffsetY
            fillPaint.color = if (s.pointerId >= 0) {
                Color.argb(180, 255, 255, 255)
            } else {
                Color.argb(120, 255, 255, 255)
            }
            canvas.drawCircle(kx, ky, knobR, fillPaint)
            canvas.drawCircle(kx, ky, knobR, strokePaint)
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        var handled = false
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val index = event.actionIndex
                handled = pressAt(event.getPointerId(index), event.getX(index), event.getY(index))
            }
            MotionEvent.ACTION_MOVE -> {
                for (i in 0 until event.pointerCount) {
                    if (movePointer(event.getPointerId(i), event.getX(i), event.getY(i))) {
                        handled = true
                    }
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                handled = releasePointer(event.getPointerId(event.actionIndex))
            }
            MotionEvent.ACTION_CANCEL -> {
                releaseAll()
                return true
            }
        }
        if (handled) {
            invalidate()
        }
        return handled
    }

    private fun pressAt(pointerId: Int, x: Float, y: Float): Boolean {
        stick?.let { s ->
            if (s.pointerId < 0 && hypot(x - s.center.x, y - s.center.y) <= s.radius * 1.15f) {
                s.pointerId = pointerId
                updateStick(s, x, y)
                return true
            }
        }
        for (button in buttons) {
            if (button.pointerId < 0 &&
                hypot(x - button.center.x, y - button.center.y) <= button.radius * 1.15f
            ) {
                button.pointerId = pointerId
                listener.onTouchButton(button.button, true)
                return true
            }
        }
        return false
    }

    private fun movePointer(pointerId: Int, x: Float, y: Float): Boolean {
        stick?.let { s ->
            if (s.pointerId == pointerId) {
                updateStick(s, x, y)
                return true
            }
        }
        return buttons.any { it.pointerId == pointerId }
    }

    private fun releasePointer(pointerId: Int): Boolean {
        stick?.let { s ->
            if (s.pointerId == pointerId) {
                s.pointerId = -1
                s.knobOffsetX = 0f
                s.knobOffsetY = 0f
                listener.onTouchAxis(0, 0)
                return true
            }
        }
        for (button in buttons) {
            if (button.pointerId == pointerId) {
                button.pointerId = -1
                listener.onTouchButton(button.button, false)
                return true
            }
        }
        return false
    }

    private fun releaseAll() {
        stick?.let { s ->
            if (s.pointerId >= 0) {
                s.pointerId = -1
                s.knobOffsetX = 0f
                s.knobOffsetY = 0f
                listener.onTouchAxis(0, 0)
            }
        }
        for (button in buttons) {
            if (button.pointerId >= 0) {
                button.pointerId = -1
                listener.onTouchButton(button.button, false)
            }
        }
        invalidate()
    }

    private fun updateStick(s: StickControl, x: Float, y: Float) {
        var dx = x - s.center.x
        var dy = y - s.center.y
        val len = hypot(dx, dy)
        if (len > s.radius) {
            dx = dx / len * s.radius
            dy = dy / len * s.radius
        }
        s.knobOffsetX = dx
        s.knobOffsetY = dy
        // Match SDL axis range (i16): +X right, +Y up. Android Y grows downward.
        val axisX = ((dx / s.radius) * Short.MAX_VALUE)
            .roundToInt()
            .coerceIn(Short.MIN_VALUE.toInt(), Short.MAX_VALUE.toInt())
        val axisY = ((-dy / s.radius) * Short.MAX_VALUE)
            .roundToInt()
            .coerceIn(Short.MIN_VALUE.toInt(), Short.MAX_VALUE.toInt())
        listener.onTouchAxis(axisX, axisY)
    }

    private fun dp(value: Float): Float =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value, resources.displayMetrics)

    companion object {
        const val R_DPAD = 0
        const val L_DPAD = 1
        const val D_DPAD = 2
        const val U_DPAD = 3
        const val START_BUTTON = 4
        const val Z_TRIG = 5
        const val B_BUTTON = 6
        const val A_BUTTON = 7
        const val R_CBUTTON = 8
        const val L_CBUTTON = 9
        const val D_CBUTTON = 10
        const val U_CBUTTON = 11
        const val R_TRIG = 12
        const val L_TRIG = 13

        /** Left/right pillar widths for a centered 4:3 game area. */
        fun pillarWidth(viewWidth: Int, viewHeight: Int): Int {
            if (viewWidth <= 0 || viewHeight <= 0) {
                return 0
            }
            val gameW = min(viewWidth.toFloat(), viewHeight * 4f / 3f)
            return ((viewWidth - gameW) / 2f).roundToInt().coerceAtLeast(0)
        }
    }
}
