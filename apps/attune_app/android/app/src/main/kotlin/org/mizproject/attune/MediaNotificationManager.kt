package org.mizproject.attune

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import android.graphics.drawable.Icon
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File

/**
 * MediaNotificationManager manages Android system media playback notifications
 * and Watchguard hardware instability alerts.
 */
class MediaNotificationManager private constructor(
    private val context: Context,
    private val channel: MethodChannel
) : MethodChannel.MethodCallHandler {

    companion object {
        private const val CHANNEL_NAME = "org.mizproject.attune/notification"
        private const val TAG = "MediaNotificationMgr"

        const val CHANNEL_ID_PLAYBACK = "org.mizproject.attune.playback"
        const val CHANNEL_ID_ALERTS = "org.mizproject.attune.alerts"

        const val NOTIFICATION_ID_PLAYBACK = 1001
        const val NOTIFICATION_ID_ALERT = 1002

        const val ACTION_PLAY = "org.mizproject.attune.ACTION_PLAY"
        const val ACTION_PAUSE = "org.mizproject.attune.ACTION_PAUSE"
        const val ACTION_TOGGLE = "org.mizproject.attune.ACTION_TOGGLE"
        const val ACTION_NEXT = "org.mizproject.attune.ACTION_NEXT"
        const val ACTION_PREV = "org.mizproject.attune.ACTION_PREV"
        const val ACTION_SWITCH_CODEC = "org.mizproject.attune.ACTION_SWITCH_CODEC"

        @Volatile
        var instance: MediaNotificationManager? = null
            private set

        fun setup(context: Context, messenger: BinaryMessenger): MediaNotificationManager {
            val chan = MethodChannel(messenger, CHANNEL_NAME)
            val mgr = MediaNotificationManager(context, chan)
            chan.setMethodCallHandler(mgr)
            instance = mgr
            return mgr
        }

        fun handleAction(action: String) {
            instance?.processAction(action)
        }
    }

    private val notificationManager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    private val mainHandler = Handler(Looper.getMainLooper())
    private var lastAlertTimestamp: Long = 0L

    init {
        createNotificationChannels()
    }

    private fun createNotificationChannels() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val playbackChannel = NotificationChannel(
                CHANNEL_ID_PLAYBACK,
                "Playback Controls",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Media playback bar and track information"
                setShowBadge(false)
                lockscreenVisibility = Notification.VISIBILITY_PUBLIC
            }

            val alertChannel = NotificationChannel(
                CHANNEL_ID_ALERTS,
                "System & Hardware Alerts",
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Audio clipping and hardware underrun warnings"
                enableVibration(true)
                setShowBadge(true)
            }

            notificationManager.createNotificationChannel(playbackChannel)
            notificationManager.createNotificationChannel(alertChannel)
            Log.i(TAG, "Notification channels initialized")
        }
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "showPlaybackNotification" -> {
                val title = call.argument<String>("title") ?: "Unknown Track"
                val artist = call.argument<String>("artist") ?: "Unknown Artist"
                val isPlaying = call.argument<Boolean>("isPlaying") ?: false
                val albumArtPath = call.argument<String>("albumArtPath")
                showPlaybackNotification(title, artist, isPlaying, albumArtPath)
                result.success(true)
            }
            "hidePlaybackNotification" -> {
                hidePlaybackNotification()
                result.success(true)
            }
            "showOverloadAlert" -> {
                val title = call.argument<String>("title") ?: "System Overloaded: Audio Clipping"
                val message = call.argument<String>("message") ?: "Audio buffer underruns detected. Tap to switch to System Codec."
                val score = call.argument<Number>("score")?.toDouble() ?: 1.0
                showOverloadAlert(title, message, score)
                result.success(true)
            }
            "dismissOverloadAlert" -> {
                dismissOverloadAlert()
                result.success(true)
            }
            else -> result.notImplemented()
        }
    }

    fun showPlaybackNotification(title: String, artist: String, isPlaying: Boolean, albumArtPath: String?) {
        try {
            val openAppIntent = Intent(context, MainActivity::class.java).apply {
                action = Intent.ACTION_MAIN
                addCategory(Intent.CATEGORY_LAUNCHER)
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED
            }
            val contentPendingIntent = PendingIntent.getActivity(
                context, 0, openAppIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                Notification.Builder(context, CHANNEL_ID_PLAYBACK)
            } else {
                Notification.Builder(context)
            }

            builder.setSmallIcon(R.mipmap.ic_launcher)
                .setContentTitle(title)
                .setContentText(artist)
                .setSubText("Attune Player")
                .setOngoing(isPlaying)
                .setContentIntent(contentPendingIntent)
                .setVisibility(Notification.VISIBILITY_PUBLIC)

            // Album Artwork (Large Icon)
            if (!albumArtPath.isNullOrEmpty()) {
                val file = File(albumArtPath)
                if (file.exists() && file.canRead()) {
                    try {
                        val bitmap = BitmapFactory.decodeFile(file.absolutePath)
                        if (bitmap != null) {
                            builder.setLargeIcon(bitmap)
                        }
                    } catch (e: Exception) {
                        Log.w(TAG, "Failed to decode album artwork for notification: $albumArtPath", e)
                    }
                }
            }

            // Action 1: Previous
            val prevIntent = Intent(context, NotificationActionReceiver::class.java).apply { action = ACTION_PREV }
            val prevPending = PendingIntent.getBroadcast(
                context, 1, prevIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            builder.addAction(
                Notification.Action.Builder(
                    Icon.createWithResource(context, android.R.drawable.ic_media_previous),
                    "Previous", prevPending
                ).build()
            )

            // Action 2: Toggle Play / Pause
            val toggleIntent = Intent(context, NotificationActionReceiver::class.java).apply { action = ACTION_TOGGLE }
            val togglePending = PendingIntent.getBroadcast(
                context, 2, toggleIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            val playPauseIcon = if (isPlaying) android.R.drawable.ic_media_pause else android.R.drawable.ic_media_play
            val playPauseText = if (isPlaying) "Pause" else "Play"
            builder.addAction(
                Notification.Action.Builder(
                    Icon.createWithResource(context, playPauseIcon),
                    playPauseText, togglePending
                ).build()
            )

            // Action 3: Next
            val nextIntent = Intent(context, NotificationActionReceiver::class.java).apply { action = ACTION_NEXT }
            val nextPending = PendingIntent.getBroadcast(
                context, 3, nextIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            builder.addAction(
                Notification.Action.Builder(
                    Icon.createWithResource(context, android.R.drawable.ic_media_next),
                    "Next", nextPending
                ).build()
            )

            // Apply Android MediaStyle for media control lockscreen / shade layout
            val style = Notification.MediaStyle().setShowActionsInCompactView(0, 1, 2)
            builder.style = style

            notificationManager.notify(NOTIFICATION_ID_PLAYBACK, builder.build())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to display media playback notification", e)
        }
    }

    fun hidePlaybackNotification() {
        try {
            notificationManager.cancel(NOTIFICATION_ID_PLAYBACK)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to cancel playback notification", e)
        }
    }

    fun showOverloadAlert(title: String, message: String, score: Double) {
        val now = System.currentTimeMillis()
        if (now - lastAlertTimestamp < 15000) {
            // Debounce within 15 seconds to prevent spamming notifications on continuous underruns
            return
        }
        lastAlertTimestamp = now

        try {
            val openAppIntent = Intent(context, MainActivity::class.java).apply {
                action = Intent.ACTION_MAIN
                addCategory(Intent.CATEGORY_LAUNCHER)
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED
            }
            val contentPendingIntent = PendingIntent.getActivity(
                context, 0, openAppIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                Notification.Builder(context, CHANNEL_ID_ALERTS)
            } else {
                Notification.Builder(context)
            }

            // Action to Switch to System Codec directly from notification
            val switchIntent = Intent(context, NotificationActionReceiver::class.java).apply {
                action = ACTION_SWITCH_CODEC
            }
            val switchPending = PendingIntent.getBroadcast(
                context, 10, switchIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            builder.setSmallIcon(android.R.drawable.stat_notify_error)
                .setContentTitle(title)
                .setContentText(message)
                .setSubText("Watchguard Guardrail")
                .setAutoCancel(true)
                .setContentIntent(contentPendingIntent)
                .addAction(
                    Notification.Action.Builder(
                        Icon.createWithResource(context, android.R.drawable.ic_menu_rotate),
                        "Switch to System Codec", switchPending
                    ).build()
                )

            notificationManager.notify(NOTIFICATION_ID_ALERT, builder.build())
            Log.i(TAG, "Posted Watchguard overload alert notification (instability: $score)")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to display Watchguard overload alert", e)
        }
    }

    fun dismissOverloadAlert() {
        try {
            notificationManager.cancel(NOTIFICATION_ID_ALERT)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to dismiss overload alert", e)
        }
    }

    private fun processAction(action: String) {
        mainHandler.post {
            when (action) {
                ACTION_SWITCH_CODEC -> {
                    Log.i(TAG, "Forwarding switch codec action to Flutter")
                    channel.invokeMethod("onSwitchCodec", null)
                }
                ACTION_TOGGLE -> {
                    Log.i(TAG, "Forwarding toggle action to Flutter")
                    channel.invokeMethod("onPlaybackAction", "togglePlayPause")
                }
                ACTION_PLAY -> {
                    channel.invokeMethod("onPlaybackAction", "play")
                }
                ACTION_PAUSE -> {
                    channel.invokeMethod("onPlaybackAction", "pause")
                }
                ACTION_NEXT -> {
                    channel.invokeMethod("onPlaybackAction", "next")
                }
                ACTION_PREV -> {
                    channel.invokeMethod("onPlaybackAction", "previous")
                }
            }
        }
    }
}
