package org.mizproject.attune

import android.app.Activity
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodChannel

object PermissionManager {
    private const val PERMISSIONS_CHANNEL = "org.mizproject.attune/permissions"

    fun setup(activity: Activity, messenger: BinaryMessenger) {
        MethodChannel(messenger, PERMISSIONS_CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                "isWakelockPermissionGranted" -> {
                    val pm = activity.getSystemService(Context.POWER_SERVICE) as? PowerManager
                    val isGranted = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                        pm?.isIgnoringBatteryOptimizations(activity.packageName) ?: false
                    } else {
                        true
                    }
                    result.success(isGranted)
                }
                "requestWakelockPermission" -> {
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                        try {
                            val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
                                data = Uri.parse("package:${activity.packageName}")
                            }
                            activity.startActivity(intent)
                            result.success(true)
                        } catch (e: Exception) {
                            try {
                                val fallbackIntent = Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)
                                activity.startActivity(fallbackIntent)
                                result.success(true)
                            } catch (_: Exception) {
                                result.success(false)
                            }
                        }
                    } else {
                        result.success(true)
                    }
                }
                "isNotificationPermissionGranted" -> {
                    val nm = activity.getSystemService(Context.NOTIFICATION_SERVICE) as? NotificationManager
                    val areEnabled = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                        nm?.areNotificationsEnabled() ?: true
                    } else {
                        true
                    }
                    val postNotification = if (Build.VERSION.SDK_INT >= 33) {
                        activity.checkSelfPermission("android.permission.POST_NOTIFICATIONS") == PackageManager.PERMISSION_GRANTED
                    } else {
                        true
                    }
                    result.success(areEnabled && postNotification)
                }
                "requestNotificationPermission" -> {
                    if (Build.VERSION.SDK_INT >= 33) {
                        activity.requestPermissions(arrayOf("android.permission.POST_NOTIFICATIONS"), 1002)
                        result.success(true)
                    } else {
                        try {
                            val intent = Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).apply {
                                putExtra(Settings.EXTRA_APP_PACKAGE, activity.packageName)
                            }
                            activity.startActivity(intent)
                            result.success(true)
                        } catch (e: Exception) {
                            result.success(false)
                        }
                    }
                }
                else -> result.notImplemented()
            }
        }
    }
}
