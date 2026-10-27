package org.mizproject.attune

import android.content.Context
import android.content.Intent
import io.flutter.embedding.android.FlutterActivity

class SettingsActivity : FlutterActivity() {
    override fun getInitialRoute(): String {
        return "/settings"
    }

    companion object {
        fun createIntent(context: Context): Intent {
            return Intent(context, SettingsActivity::class.java)
        }
    }
}
