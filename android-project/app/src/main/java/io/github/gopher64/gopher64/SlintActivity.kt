package io.github.gopher64.gopher64

import android.app.NativeActivity
import android.content.Intent
import android.os.Bundle

class SlintActivity : NativeActivity() {
    companion object {
        init {
            System.loadLibrary("gopher64")
        }
    }
    private external fun nativeOnActivityResult(requestCode: Int, resultCode: Int, data: Intent?)

    override fun onCreate(savedInstanceState: Bundle?) {
        // A restored launcher must not replay the ROM intent that opened its game activity.
        if (savedInstanceState != null && intent?.action == Intent.ACTION_VIEW) {
            setIntent(Intent(this, SlintActivity::class.java).setAction(Intent.ACTION_MAIN))
        }
        super.onCreate(savedInstanceState)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)

        nativeOnActivityResult(requestCode, resultCode, data)
    }
}
