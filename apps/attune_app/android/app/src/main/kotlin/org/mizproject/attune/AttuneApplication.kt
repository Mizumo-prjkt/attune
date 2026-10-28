package org.mizproject.attune

import android.content.Context
import android.util.Log
import io.flutter.app.FlutterApplication

class AttuneApplication : FlutterApplication() {
    companion object {
        private const val TAG = "AttuneApp"

        init {
            try {
                System.loadLibrary("attune_engine")
                Log.i(TAG, "Native library libattune_engine.so loaded successfully")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load attune_engine library", e)
            }
        }
    }

    private external fun initNativeContext(context: Context)

    override fun onCreate() {
        super.onCreate()
        try {
            initNativeContext(this)
            Log.i(TAG, "Native context initialized in AttuneApplication.onCreate")
        } catch (e: Throwable) {
            Log.e(TAG, "initNativeContext call failed", e)
        }
    }
}
