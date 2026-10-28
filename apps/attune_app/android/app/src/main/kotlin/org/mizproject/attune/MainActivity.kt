package org.mizproject.attune

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private val NAVIGATION_CHANNEL = "org.mizproject.attune/navigation"

    companion object {
        init {
            try {
                System.loadLibrary("attune_engine")
            } catch (e: UnsatisfiedLinkError) {
                Log.e("MainActivity", "Failed to load attune_engine library", e)
            }
        }
    }

    private external fun initNativeContext(context: Context)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        try {
            initNativeContext(applicationContext)
        } catch (e: Throwable) {
            Log.e("MainActivity", "initNativeContext in MainActivity failed", e)
        }
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        PermissionManager.setup(this, flutterEngine.dartExecutor.binaryMessenger)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, NAVIGATION_CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                "openSettings" -> {
                    val intent = Intent(this, SettingsActivity::class.java)
                    startActivity(intent)
                    result.success(true)
                }
                "openTransitionInspector" -> {
                    val intent = Intent(this, TransitionInspectorActivity::class.java)
                    startActivity(intent)
                    result.success(true)
                }
                else -> result.notImplemented()
            }
        }
    }
}
