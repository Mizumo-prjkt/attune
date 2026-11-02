package org.mizproject.attune

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * BroadcastReceiver for handling notification action clicks (play, pause, next, prev, switch codec)
 * from Android status bar and lockscreen media notifications.
 */
class NotificationActionReceiver : BroadcastReceiver() {
    companion object {
        private const val TAG = "NotificationReceiver"
    }

    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action ?: return
        Log.i(TAG, "Notification action received: $action")
        MediaNotificationManager.handleAction(action)
    }
}
