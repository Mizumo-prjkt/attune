package org.mizproject.attune

import android.content.Context
import android.content.Intent
import io.flutter.embedding.android.FlutterActivity

class TransitionInspectorActivity : FlutterActivity() {
    override fun getInitialRoute(): String {
        return "/transition_inspector"
    }

    companion object {
        fun createIntent(context: Context): Intent {
            return Intent(context, TransitionInspectorActivity::class.java)
        }
    }
}
