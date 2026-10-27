package org.mizproject.attune

import android.content.Context
import android.content.Intent
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine

class SettingsActivity : FlutterActivity() {
    override fun getInitialRoute(): String {
        return "/settings"
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        PermissionManager.setup(this, flutterEngine.dartExecutor.binaryMessenger)
    }

    companion object {
        fun createIntent(context: Context): Intent {
            return Intent(context, SettingsActivity::class.java)
        }
    }
}
