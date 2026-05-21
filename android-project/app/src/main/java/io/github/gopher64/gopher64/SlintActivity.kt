package io.github.gopher64.gopher64

import android.app.NativeActivity
import android.content.Intent

class SlintActivity : NativeActivity() {
    private external fun nativeOnActivityResult(requestCode: Int, resultCode: Int, data: Intent?)

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)

        nativeOnActivityResult(requestCode, resultCode, data)
    }
}
